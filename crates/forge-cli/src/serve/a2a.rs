use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::sync::Arc;

use super::state::AppState;
use crate::agents::config::AgentConfig;
use crate::agents::execution::{build_tool_definitions, execute_tool, parse_text_leaked_tool_call};

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

/// Maximum tool-call iterations before we stop and return whatever content
/// the model produced. Matches the implicit limit in the REPL loop.
const MAX_TOOL_TURNS: usize = 8;

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

    // Optional: callers can target a specific agent via params.agent or
    // params.skill_id. Default = first loaded agent (matches what
    // `.well-known/agent.json` advertises as the primary skill).
    let agent_name = params
        .get("agent")
        .or_else(|| params.get("skill_id"))
        .and_then(|v| v.as_str());

    let agent = pick_agent(state, agent_name).await.ok_or_else(|| {
        serde_json::json!({
            "code": -32602,
            "message": "no agent available on this server",
        })
    })?;

    // Build tools from the agent's `tools:` block + every MCP server
    // currently registered on the shared mcp_manager (populated by
    // adk/dev.rs::spawn_agent_mcp_servers at server startup).
    let mcp_mgr = state.mcp_manager.read().await;
    let tool_defs = build_tool_definitions(&agent.tools, &mcp_mgr).await;

    let model = agent
        .model
        .clone()
        .unwrap_or_else(|| state.client.model().to_string());

    // Seed message history with the agent's system prompt + the caller's task.
    let mut messages: Vec<liteforge::Message> = Vec::new();
    if let Some(sp) = &agent.system_prompt {
        messages.push(liteforge::Message::system(sp));
    }
    messages.push(liteforge::Message::user(message_text));

    // Tool-call loop. Mirror the REPL's structure: non-streaming chat,
    // execute any returned tool_calls via execute_tool, feed results back
    // as `tool` role messages, repeat until the model returns no tool_calls
    // (or MAX_TOOL_TURNS is hit).
    let mut final_content = String::new();
    for _turn in 0..MAX_TOOL_TURNS {
        let mut req = liteforge::ChatCompletionRequest::new(&model, messages.clone());
        if !tool_defs.is_empty() {
            req = req.tools(tool_defs.clone());
        }
        if let Some(temp) = agent.temperature {
            req = req.temperature(temp);
        }
        if let Some(mt) = agent.max_tokens {
            req = req.max_tokens(mt);
        }

        let completion = match state.client.chat_completions(req).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(serde_json::json!({
                    "id": task_id,
                    "status": {
                        "state": "failed",
                        "message": format!("chat_completions failed: {}", e),
                    },
                }));
            }
        };

        let Some(choice) = completion.choices.into_iter().next() else {
            break;
        };
        let msg = choice.message;
        let mut content = msg.content.clone().unwrap_or_default();
        let mut tool_calls = msg.tool_calls.clone().unwrap_or_default();

        // Same text-leak recovery as the REPL: some models emit tool
        // calls as JSON content instead of structured tool_calls.
        if tool_calls.is_empty() && !content.is_empty() {
            if let Some(parsed) = parse_text_leaked_tool_call(&content) {
                tool_calls.push(parsed);
                content.clear();
            }
        }

        if tool_calls.is_empty() {
            final_content = content;
            break;
        }

        // Otherwise: record the assistant turn with its tool_calls,
        // execute each call, append the tool results, loop.
        messages.push(liteforge::Message {
            role: "assistant".to_string(),
            content: if content.is_empty() { None } else { Some(content) },
            name: None,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        for call in &tool_calls {
            let result = execute_tool(&agent.tools, &mcp_mgr, call).await;
            let tool_content = match result {
                Ok(v) => {
                    if v.is_string() {
                        v.as_str().unwrap_or("").to_string()
                    } else {
                        serde_json::to_string(&v).unwrap_or_else(|_| v.to_string())
                    }
                }
                Err(e) => format!("Error: {}", e),
            };
            messages.push(liteforge::Message::tool(&call.id, tool_content));
        }
    }

    Ok(serde_json::json!({
        "id": task_id,
        "status": { "state": "completed" },
        "artifacts": [{
            "parts": [{ "type": "text", "text": final_content }],
        }],
    }))
}

/// Pick an agent by name; if name is None, fall back to the first agent
/// in the loaded list (matches how the agent card advertises skills).
async fn pick_agent(state: &AppState, name: Option<&str>) -> Option<AgentConfig> {
    let agents = state.agents.read().await;
    if let Some(n) = name {
        agents.iter().find(|a| a.name == n).cloned()
    } else {
        agents.first().cloned()
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
