use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::sync::Arc;

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a", post(a2a_handler))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "a2a",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn agent_card(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = state.agents.read().await;
    let skills: Vec<serde_json::Value> = agents
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.name,
                "name": a.name,
                "description": a.description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "name": "forge-serve",
        "description": "LiteForge Agent Server",
        "url": format!("http://{}:{}", state.config.a2a.host, state.config.a2a.port),
        "version": env!("CARGO_PKG_VERSION"),
        "capabilities": {
            "streaming": false,
            "pushNotifications": false,
        },
        "skills": skills,
        "defaultInputModes": ["text"],
        "defaultOutputModes": ["text"],
    }))
}

#[derive(Deserialize)]
struct A2ARequest {
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

async fn a2a_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<A2ARequest>,
) -> Json<serde_json::Value> {
    let id = req.id.clone().unwrap_or(serde_json::Value::Null);

    let result = match req.method.as_str() {
        "tasks/send" => handle_task_send(&state, req.params).await,
        "tasks/get" => handle_task_get(req.params),
        "tasks/cancel" => handle_task_cancel(req.params),
        _ => Err(serde_json::json!({
            "code": -32601,
            "message": "Method not found",
        })),
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

async fn handle_task_send(
    state: &AppState,
    params: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let message_text = params
        .get("message")
        .and_then(|m| m.get("parts"))
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.first())
        .and_then(|part| part.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let messages = vec![liteforge::Message::user(message_text)];
    let model = state.client.model().to_string();
    let sdk_req = liteforge::ChatCompletionRequest::new(model, messages);

    match state.client.chat_completions(sdk_req).await {
        Ok(completion) => {
            let content = completion.content().unwrap_or("").to_string();
            Ok(serde_json::json!({
                "id": task_id,
                "status": { "state": "completed" },
                "artifacts": [{
                    "parts": [{ "type": "text", "text": content }],
                }],
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "id": task_id,
            "status": {
                "state": "failed",
                "message": e.to_string(),
            },
        })),
    }
}

fn handle_task_get(params: serde_json::Value) -> Result<serde_json::Value, serde_json::Value> {
    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(serde_json::json!({
        "id": task_id,
        "status": { "state": "unknown", "message": "Task tracking is synchronous-only in this version" },
    }))
}

fn handle_task_cancel(params: serde_json::Value) -> Result<serde_json::Value, serde_json::Value> {
    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(serde_json::json!({
        "id": task_id,
        "status": { "state": "canceled" },
    }))
}
