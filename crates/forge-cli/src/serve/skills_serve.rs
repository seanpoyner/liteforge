use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::{http::StatusCode, Router};
use serde::Deserialize;
use std::sync::Arc;
use liteforge::skills::SkillInput;

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/skills", get(list_skills))
        .route("/skills/:name", get(get_skill))
        .route("/skills/:name/execute", post(execute_skill))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "skills",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn list_skills(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let reg = state.skill_registry.read().await;
    let names = reg.list();
    let mut skills = Vec::new();
    for name in &names {
        if let Some(s) = reg.get(name) {
            let cfg = s.config();
            skills.push(serde_json::json!({
                "name": cfg.name,
                "description": cfg.description,
                "tags": cfg.tags,
            }));
        }
    }
    Json(serde_json::json!({ "skills": skills }))
}

async fn get_skill(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let reg = state.skill_registry.read().await;
    let skill = reg.get(&name).ok_or(StatusCode::NOT_FOUND)?;
    let cfg = skill.config();
    Ok(Json(serde_json::json!({
        "name": cfg.name,
        "description": cfg.description,
        "model": cfg.model,
        "system_prompt": cfg.system_prompt,
        "tags": cfg.tags,
        "input_schema": cfg.input_schema,
        "output_schema": cfg.output_schema,
    })))
}

#[derive(Deserialize)]
struct ExecuteReq {
    text: String,
    #[serde(default)]
    params: std::collections::HashMap<String, serde_json::Value>,
    context: Option<serde_json::Value>,
}

async fn execute_skill(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(req): Json<ExecuteReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let reg = state.skill_registry.read().await;
    let skill = reg.get(&name).ok_or(StatusCode::NOT_FOUND)?;

    let mut input = SkillInput::new(&req.text);
    for (k, v) in &req.params {
        input = input.with_param(k, v.clone());
    }
    if let Some(ctx) = req.context {
        input = input.with_context(ctx);
    }

    match skill.execute(&state.client, input).await {
        Ok(output) => Ok(Json(serde_json::json!({
            "text": output.text,
            "data": output.data,
            "metadata": output.metadata,
        }))),
        Err(e) => Ok(Json(serde_json::json!({ "error": e.to_string() }))),
    }
}
