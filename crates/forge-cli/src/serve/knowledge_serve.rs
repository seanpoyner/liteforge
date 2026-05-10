use axum::extract::State;
use axum::response::Json;
use axum::routing::{delete, get, post};
use axum::{http::StatusCode, Router};
use serde::Deserialize;
use std::sync::Arc;
use liteforge::knowledge::{Document, ListOptions, SearchOptions};

use super::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/knowledge/search", post(search))
        .route("/knowledge/upload", post(upload))
        .route("/knowledge/documents", get(list_documents))
        .route("/knowledge/documents/:id", get(get_document))
        .route("/knowledge/documents/:id", delete(delete_document))
        .route("/knowledge/stats", get(stats))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "role": "knowledge",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Deserialize)]
struct SearchReq {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    namespace: Option<String>,
    min_score: Option<f32>,
}

fn default_limit() -> usize {
    10
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut opts = SearchOptions::default().limit(req.limit);
    if let Some(ns) = req.namespace {
        opts = opts.namespace(ns);
    }
    if let Some(ms) = req.min_score {
        opts = opts.min_score(ms);
    }

    match state.knowledge.search(&req.query, opts).await {
        Ok(results) => Ok(Json(serde_json::json!({
            "results": results.iter().map(|r| serde_json::json!({
                "document": {
                    "id": r.document.id,
                    "content": r.document.content,
                    "metadata": r.document.metadata,
                },
                "score": r.score,
            })).collect::<Vec<_>>()
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct UploadReq {
    documents: Vec<DocInput>,
}

#[derive(Deserialize)]
struct DocInput {
    id: String,
    content: String,
    #[serde(default)]
    metadata: std::collections::HashMap<String, serde_json::Value>,
    namespace: Option<String>,
}

async fn upload(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let docs: Vec<Document> = req
        .documents
        .into_iter()
        .map(|d| {
            let mut doc = Document::new(&d.id, &d.content);
            for (k, v) in d.metadata {
                doc = doc.metadata(k, v);
            }
            if let Some(ns) = d.namespace {
                doc = doc.namespace(ns);
            }
            doc
        })
        .collect();

    match state.knowledge.upload(docs).await {
        Ok(ids) => Ok(Json(serde_json::json!({ "uploaded": ids }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    namespace: Option<String>,
}

async fn list_documents(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut opts = ListOptions::default();
    if let Some(l) = q.limit {
        opts = opts.limit(l);
    }
    if let Some(o) = q.offset {
        opts = opts.offset(o);
    }
    if let Some(ns) = q.namespace {
        opts = opts.namespace(ns);
    }

    match state.knowledge.list(opts).await {
        Ok(docs) => Ok(Json(serde_json::json!({
            "documents": docs.iter().map(|d| serde_json::json!({
                "id": d.id,
                "content": d.content,
                "metadata": d.metadata,
            })).collect::<Vec<_>>()
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_document(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.knowledge.get(&id).await {
        Ok(Some(doc)) => Ok(Json(serde_json::json!({
            "id": doc.id,
            "content": doc.content,
            "metadata": doc.metadata,
        }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_document(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.knowledge.delete(&id).await {
        Ok(deleted) => Ok(Json(serde_json::json!({ "deleted": deleted }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn stats(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.knowledge.stats().await {
        Ok(s) => Ok(Json(serde_json::json!({
            "document_count": s.document_count,
            "namespace_count": s.namespace_count,
            "namespaces": s.namespaces,
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
