use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/mcp", post(jsonrpc_handler))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "mcp",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

async fn jsonrpc_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RpcRequest>,
) -> Json<serde_json::Value> {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    let result = match req.method.as_str() {
        "initialize" => handle_initialize(&state).await,
        "tools/list" => handle_list_tools(&state).await,
        "tools/call" => handle_call_tool(&state, req.params).await,
        "resources/list" => handle_list_resources(&state).await,
        "resources/read" => handle_read_resource(&state, req.params).await,
        "prompts/list" => handle_list_prompts(&state).await,
        "prompts/get" => handle_get_prompt(&state, req.params).await,
        _ => Err(rpc_error(-32601, "Method not found")),
    };

    match result {
        Ok(val) => Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": val,
        })),
        Err(err) => Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": err,
        })),
    }
}

fn rpc_error(code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({ "code": code, "message": message })
}

async fn handle_initialize(_state: &AppState) -> Result<serde_json::Value, serde_json::Value> {
    Ok(serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false },
        },
        "serverInfo": {
            "name": "forge-serve",
            "version": env!("CARGO_PKG_VERSION"),
        },
    }))
}

async fn handle_list_tools(state: &AppState) -> Result<serde_json::Value, serde_json::Value> {
    let reg = state.tool_registry.read().await;
    let tools: Vec<serde_json::Value> = reg
        .tools()
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name(),
                "description": t.description(),
                "inputSchema": t.parameters_schema(),
            })
        })
        .collect();

    let mcp_mgr = state.mcp_manager.read().await;
    let mut all_tools = tools;
    for (_server, server_tools) in mcp_mgr.list_all_tools().await {
        for t in server_tools {
            all_tools.push(serde_json::json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            }));
        }
    }

    Ok(serde_json::json!({ "tools": all_tools }))
}

async fn handle_call_tool(
    state: &AppState,
    params: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_error(-32602, "Missing 'name' parameter"))?
        .to_string();

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let reg = state.tool_registry.read().await;
    if let Some(tool) = reg.get(&name) {
        return match tool.execute(arguments) {
            Ok(val) => Ok(serde_json::json!({
                "content": [{ "type": "text", "text": val.to_string() }],
            })),
            Err(e) => Ok(serde_json::json!({
                "content": [{ "type": "text", "text": e }],
                "isError": true,
            })),
        };
    }
    drop(reg);

    let mcp_mgr = state.mcp_manager.read().await;
    for (server_name, tools) in mcp_mgr.list_all_tools().await {
        if tools.iter().any(|t| t.name == name) {
            if let Some(server) = mcp_mgr.get(&server_name) {
                let args: Option<HashMap<String, serde_json::Value>> =
                    serde_json::from_value(arguments).ok();
                let call_params = liteforge::mcp::CallToolParams {
                    name: name.clone(),
                    arguments: args,
                };
                return match server.call_tool(call_params).await {
                    Ok(result) => Ok(serde_json::to_value(&result).unwrap_or_default()),
                    Err(e) => Ok(serde_json::json!({
                        "content": [{ "type": "text", "text": e.to_string() }],
                        "isError": true,
                    })),
                };
            }
        }
    }

    Err(rpc_error(-32602, &format!("Tool not found: {}", name)))
}

async fn handle_list_resources(state: &AppState) -> Result<serde_json::Value, serde_json::Value> {
    let knowledge = &state.knowledge;
    let list_opts = liteforge::knowledge::ListOptions::default().limit(100);
    let docs = knowledge.list(list_opts).await.unwrap_or_default();

    let resources: Vec<serde_json::Value> = docs
        .iter()
        .map(|d| {
            serde_json::json!({
                "uri": format!("knowledge://{}", d.id),
                "name": d.id,
                "description": format!("Knowledge document: {}", d.id),
                "mimeType": "text/plain",
            })
        })
        .collect();

    Ok(serde_json::json!({ "resources": resources }))
}

async fn handle_read_resource(
    state: &AppState,
    params: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let uri = params
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_error(-32602, "Missing 'uri' parameter"))?;

    let id = uri.strip_prefix("knowledge://").unwrap_or(uri);

    match state.knowledge.get(id).await {
        Ok(Some(doc)) => Ok(serde_json::json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": doc.content,
            }],
        })),
        Ok(None) => Err(rpc_error(-32602, &format!("Resource not found: {}", uri))),
        Err(e) => Err(rpc_error(-32603, &e.to_string())),
    }
}

async fn handle_list_prompts(_state: &AppState) -> Result<serde_json::Value, serde_json::Value> {
    let builtins = vec![
        serde_json::json!({
            "name": "summarize",
            "description": "Summarize the given text",
            "arguments": [{ "name": "text", "required": true }],
        }),
        serde_json::json!({
            "name": "translate",
            "description": "Translate text to a target language",
            "arguments": [
                { "name": "text", "required": true },
                { "name": "language", "required": true },
            ],
        }),
    ];
    Ok(serde_json::json!({ "prompts": builtins }))
}

async fn handle_get_prompt(
    _state: &AppState,
    params: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_error(-32602, "Missing 'name' parameter"))?;

    let arguments: HashMap<String, String> = params
        .get("arguments")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    match name {
        "summarize" => {
            let text = arguments.get("text").cloned().unwrap_or_default();
            Ok(serde_json::json!({
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": format!("Please summarize the following text:\n\n{}", text) },
                }],
            }))
        }
        "translate" => {
            let text = arguments.get("text").cloned().unwrap_or_default();
            let lang = arguments
                .get("language")
                .cloned()
                .unwrap_or("English".to_string());
            Ok(serde_json::json!({
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": format!("Translate the following text to {}:\n\n{}", lang, text) },
                }],
            }))
        }
        _ => Err(rpc_error(-32602, &format!("Prompt not found: {}", name))),
    }
}
