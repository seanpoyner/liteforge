use crate::agents::execution::{build_tool_definitions, execute_tool};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use liteforge::{ChatCompletionRequest, Message as SdkMessage};

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/agents", get(list_agents))
        .route("/v1/agents/:name", get(get_agent))
        .route("/v1/agents/:name/chat", post(agent_chat))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "user",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.client.list_models().await {
        Ok(models) => Ok(Json(serde_json::to_value(&models).unwrap_or_default())),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

async fn embeddings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| state.client.model());
    let input = body.get("input").cloned().unwrap_or_default();

    let text = if let Some(s) = input.as_str() {
        s.to_string()
    } else {
        input.to_string()
    };

    match state.client.embed(&text).await {
        Ok(resp) => Ok(Json(serde_json::to_value(&resp).unwrap_or_default())),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

#[derive(Deserialize)]
struct ChatReq {
    model: Option<String>,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: Option<bool>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    #[serde(default)]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ChatMessage {
    role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_calls: Option<serde_json::Value>,
}

fn chat_msg_to_sdk(m: ChatMessage) -> SdkMessage {
    match m.role.as_str() {
        "system" => SdkMessage::system(m.content.unwrap_or_default()),
        "assistant" => SdkMessage::assistant(m.content.unwrap_or_default()),
        "tool" => SdkMessage::tool(
            m.tool_call_id.unwrap_or_default(),
            m.content.unwrap_or_default(),
        ),
        _ => SdkMessage::user(m.content.unwrap_or_default()),
    }
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatReq>,
) -> Result<Response, StatusCode> {
    let messages: Vec<SdkMessage> = req.messages.into_iter().map(chat_msg_to_sdk).collect();
    let model = req
        .model
        .unwrap_or_else(|| state.client.model().to_string());

    let mut sdk_req = ChatCompletionRequest::new(model.clone(), messages);
    if let Some(t) = req.temperature {
        sdk_req = sdk_req.temperature(t);
    }
    if let Some(mt) = req.max_tokens {
        sdk_req = sdk_req.max_tokens(mt);
    }

    let streaming = req.stream.unwrap_or(false);

    if streaming {
        match state.client.chat_completions_stream(sdk_req).await {
            Ok(stream) => {
                let sse_stream = stream.map(|chunk_result| match chunk_result {
                    Ok(chunk) => {
                        let data = serde_json::to_string(&chunk).unwrap_or_default();
                        Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", data))
                    }
                    Err(_) => Ok("data: [DONE]\n\n".to_string()),
                });

                let body_stream = sse_stream.chain(futures::stream::once(async {
                    Ok::<_, std::convert::Infallible>("data: [DONE]\n\n".to_string())
                }));

                let body = Body::from_stream(body_stream);
                Ok(Response::builder()
                    .status(200)
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .header(header::CACHE_CONTROL, "no-cache")
                    .body(body)
                    .unwrap())
            }
            Err(_) => Err(StatusCode::BAD_GATEWAY),
        }
    } else {
        match state.client.chat_completions(sdk_req).await {
            Ok(completion) => {
                let json = serde_json::to_value(&completion).unwrap_or_default();
                Ok(Json(json).into_response())
            }
            Err(_) => Err(StatusCode::BAD_GATEWAY),
        }
    }
}

#[derive(Serialize)]
struct AgentInfo {
    name: String,
    description: String,
    model: Option<String>,
    tool_count: usize,
}

#[derive(Serialize)]
struct AgentDetail {
    name: String,
    description: String,
    model: Option<String>,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Vec<String>,
}

async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agents = state.agents.read().await;
    let list: Vec<AgentInfo> = agents
        .iter()
        .map(|a| AgentInfo {
            name: a.name.clone(),
            description: a.description.clone(),
            model: a.model.clone(),
            tool_count: a.tools.len(),
        })
        .collect();
    Ok(Json(serde_json::json!({ "agents": list })))
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agents = state.agents.read().await;
    let agent = agents
        .iter()
        .find(|a| a.name == name)
        .ok_or(StatusCode::NOT_FOUND)?;
    let detail = AgentDetail {
        name: agent.name.clone(),
        description: agent.description.clone(),
        model: agent.model.clone(),
        system_prompt: agent.system_prompt.clone(),
        temperature: agent.temperature,
        max_tokens: agent.max_tokens,
        tools: agent.tools.iter().map(|t| t.name.clone()).collect(),
    };
    Ok(Json(serde_json::to_value(&detail).unwrap_or_default()))
}

#[derive(Deserialize)]
struct AgentChatReq {
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: Option<bool>,
}

async fn agent_chat(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<AgentChatReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let agents = state.agents.read().await;
    let agent = agents
        .iter()
        .find(|a| a.name == name)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();
    drop(agents);

    let mcp_mgr = state.mcp_manager.read().await;
    let tool_defs = build_tool_definitions(&agent.tools, &mcp_mgr).await;

    let mut messages: Vec<SdkMessage> = Vec::new();
    if let Some(sp) = &agent.system_prompt {
        messages.push(SdkMessage::system(sp));
    }
    for m in req.messages {
        messages.push(chat_msg_to_sdk(m));
    }

    let model = agent
        .model
        .clone()
        .unwrap_or_else(|| state.client.model().to_string());

    let build_request = |msgs: Vec<SdkMessage>| {
        let mut r = ChatCompletionRequest::new(model.clone(), msgs);
        if let Some(t) = agent.temperature {
            r = r.temperature(t);
        }
        if let Some(mt) = agent.max_tokens {
            r = r.max_tokens(mt);
        }
        if !tool_defs.is_empty() {
            r = r.tools(tool_defs.clone());
        }
        r
    };

    let max_steps = 10;
    for _ in 0..max_steps {
        let sdk_req = build_request(messages.clone());
        let completion = state
            .client
            .chat_completions(sdk_req)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        let choice = completion.choices.first().ok_or(StatusCode::BAD_GATEWAY)?;

        let has_tool_calls = choice
            .message
            .tool_calls
            .as_ref()
            .map(|tc| !tc.is_empty())
            .unwrap_or(false);

        if !has_tool_calls || choice.finish_reason.as_deref() == Some("stop") {
            return Ok(Json(serde_json::to_value(&completion).unwrap_or_default()));
        }

        let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();

        messages.push(SdkMessage {
            role: "assistant".to_string(),
            content: choice.message.content.clone(),
            name: None,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        for tc in &tool_calls {
            let result = execute_tool(&agent.tools, &mcp_mgr, tc).await;
            let content = match result {
                Ok(val) => val.to_string(),
                Err(e) => format!("Error: {}", e),
            };
            messages.push(SdkMessage::tool(&tc.id, content));
        }
    }

    Err(StatusCode::INTERNAL_SERVER_ERROR)
}
