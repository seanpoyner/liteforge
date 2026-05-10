use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::{http::StatusCode, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tools", get(list_tools))
        .route("/tools/:name", get(get_tool))
        .route("/tools/:name/call", post(call_tool))
        .route("/tools/batch", post(batch_call))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "tools",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Serialize)]
struct ToolInfo {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

async fn list_tools(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let reg = state.tool_registry.read().await;
    let tools: Vec<ToolInfo> = reg
        .tools()
        .iter()
        .map(|t| ToolInfo {
            name: t.name().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters_schema(),
        })
        .collect();

    let mcp_mgr = state.mcp_manager.read().await;
    let mut mcp_tools = Vec::new();
    for (_server, server_tools) in mcp_mgr.list_all_tools().await {
        for t in server_tools {
            mcp_tools.push(ToolInfo {
                name: t.name.clone(),
                description: t.description.clone().unwrap_or_default(),
                parameters: t.input_schema.clone(),
            });
        }
    }

    let mut all_tools = tools;
    all_tools.extend(mcp_tools);

    Json(serde_json::json!({ "tools": all_tools }))
}

async fn get_tool(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let reg = state.tool_registry.read().await;
    if let Some(tool) = reg.get(&name) {
        return Ok(Json(serde_json::json!({
            "name": tool.name(),
            "description": tool.description(),
            "parameters": tool.parameters_schema(),
        })));
    }

    let mcp_mgr = state.mcp_manager.read().await;
    for (_server, tools) in mcp_mgr.list_all_tools().await {
        if let Some(t) = tools.iter().find(|t| t.name == name) {
            return Ok(Json(serde_json::json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.input_schema,
            })));
        }
    }

    Err(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct CallToolReq {
    arguments: serde_json::Value,
}

async fn call_tool(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(req): Json<CallToolReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let reg = state.tool_registry.read().await;
    if let Some(tool) = reg.get(&name) {
        return match tool.execute(req.arguments) {
            Ok(val) => Ok(Json(serde_json::json!({ "result": val }))),
            Err(e) => Ok(Json(serde_json::json!({ "error": e }))),
        };
    }
    drop(reg);

    let mcp_mgr = state.mcp_manager.read().await;
    for (server_name, tools) in mcp_mgr.list_all_tools().await {
        if tools.iter().any(|t| t.name == name) {
            if let Some(server) = mcp_mgr.get(&server_name) {
                let arguments: Option<std::collections::HashMap<String, serde_json::Value>> =
                    serde_json::from_value(req.arguments.clone()).ok();
                let params = liteforge::mcp::CallToolParams {
                    name: name.clone(),
                    arguments,
                };
                return match server.call_tool(params).await {
                    Ok(result) => Ok(Json(serde_json::to_value(&result).unwrap_or_default())),
                    Err(e) => Ok(Json(serde_json::json!({ "error": e.to_string() }))),
                };
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct BatchReq {
    calls: Vec<BatchCallItem>,
}

#[derive(Deserialize)]
struct BatchCallItem {
    name: String,
    arguments: serde_json::Value,
}

async fn batch_call(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchReq>,
) -> Json<serde_json::Value> {
    let mut results = Vec::new();
    let reg = state.tool_registry.read().await;

    for call in &req.calls {
        if let Some(tool) = reg.get(&call.name) {
            match tool.execute(call.arguments.clone()) {
                Ok(val) => results.push(serde_json::json!({
                    "name": call.name, "result": val
                })),
                Err(e) => results.push(serde_json::json!({
                    "name": call.name, "error": e
                })),
            }
        } else {
            results.push(serde_json::json!({
                "name": call.name, "error": "Tool not found"
            }));
        }
    }

    Json(serde_json::json!({ "results": results }))
}
