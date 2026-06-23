//! End-to-end routing tests against a hand-rolled mock OpenAI server.
//!
//! No live gateway is used: each mock binds to `127.0.0.1:0`, serves a fixed
//! status/body per connection, and counts hits so we can assert fallback and
//! cooldown behaviour.
#![cfg(feature = "routing")]

use liteforge::routing::{RouterBuilder, RoutingStrategy};
use liteforge::{ChatCompletionRequest, ForgeConfig, Message};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

fn ok_completion() -> String {
    serde_json::json!({
        "id": "cmpl-1",
        "object": "chat.completion",
        "created": 0,
        "model": "mock",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hi"},
            "finish_reason": "stop"
        }]
    })
    .to_string()
}

fn err_body(msg: &str) -> String {
    serde_json::json!({"error": {"message": msg}}).to_string()
}

/// Spawn a mock server that replies to every connection with `(status, body)`.
async fn spawn(status: u16, body: String, hits: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => break,
            };
            hits.fetch_add(1, Ordering::SeqCst);
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason(status),
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });
    format!("http://{addr}")
}

fn base() -> ForgeConfig {
    ForgeConfig {
        api_key: Some("test-key".into()),
        default_model: "unused".into(),
        base_url: "http://unused".into(),
        timeout: Duration::from_secs(5),
        default_headers: Default::default(),
        default_metadata: Default::default(),
        otel: None,
    }
}

fn req(group: &str) -> ChatCompletionRequest {
    ChatCompletionRequest::new(group, vec![Message::user("hello")])
}

#[tokio::test]
async fn single_healthy_deployment_succeeds() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn(200, ok_completion(), hits.clone()).await;
    let router = RouterBuilder::new()
        .base_config(base())
        .add_deployment("g", "real-model", url)
        .build()
        .unwrap();

    let resp = router.chat_completions(req("g")).await.unwrap();
    assert_eq!(resp.content(), Some("hi"));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_over_to_sibling_deployment_on_5xx() {
    let hits_a = Arc::new(AtomicUsize::new(0));
    let hits_b = Arc::new(AtomicUsize::new(0));
    let a = spawn(500, err_body("boom"), hits_a.clone()).await;
    let b = spawn(200, ok_completion(), hits_b.clone()).await;

    let router = RouterBuilder::new()
        .base_config(base())
        .strategy(RoutingStrategy::RoundRobin) // deterministic: tries A first
        .add_deployment("g", "m", a)
        .add_deployment("g", "m", b)
        .build()
        .unwrap();

    let resp = router.chat_completions(req("g")).await.unwrap();
    assert_eq!(resp.content(), Some("hi"));
    assert!(hits_a.load(Ordering::SeqCst) >= 1, "A should be tried");
    assert_eq!(hits_b.load(Ordering::SeqCst), 1, "B should serve once");
}

#[tokio::test]
async fn falls_over_to_fallback_group() {
    let hits_a = Arc::new(AtomicUsize::new(0));
    let hits_b = Arc::new(AtomicUsize::new(0));
    let a = spawn(500, err_body("down"), hits_a.clone()).await;
    let b = spawn(200, ok_completion(), hits_b.clone()).await;

    let router = RouterBuilder::new()
        .base_config(base())
        .add_deployment("premium", "m", a)
        .add_deployment("cheap", "m", b)
        .fallback("premium", vec!["cheap".into()])
        .build()
        .unwrap();

    let resp = router.chat_completions(req("premium")).await.unwrap();
    assert_eq!(resp.content(), Some("hi"));
    assert_eq!(hits_b.load(Ordering::SeqCst), 1, "fallback group served");
}

#[tokio::test]
async fn non_retryable_400_does_not_exhaust_budget() {
    // A returns 400 (non-retryable) -> router should move to the next group
    // rather than retrying A repeatedly.
    let hits_a = Arc::new(AtomicUsize::new(0));
    let hits_b = Arc::new(AtomicUsize::new(0));
    let a = spawn(400, err_body("bad"), hits_a.clone()).await;
    let b = spawn(200, ok_completion(), hits_b.clone()).await;

    let router = RouterBuilder::new()
        .base_config(base())
        .add_deployment("premium", "m", a)
        .add_deployment("cheap", "m", b)
        .fallback("premium", vec!["cheap".into()])
        .build()
        .unwrap();

    let resp = router.chat_completions(req("premium")).await.unwrap();
    assert_eq!(resp.content(), Some("hi"));
    assert_eq!(hits_a.load(Ordering::SeqCst), 1, "A tried exactly once");
}

#[tokio::test]
async fn all_failing_returns_last_error() {
    let hits = Arc::new(AtomicUsize::new(0));
    let a = spawn(500, err_body("nope"), hits.clone()).await;
    let router = RouterBuilder::new()
        .base_config(base())
        .add_deployment("g", "m", a)
        .build()
        .unwrap();

    let result = router.chat_completions(req("g")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn concurrent_requests_are_safe() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn(200, ok_completion(), hits.clone()).await;
    let router = Arc::new(
        RouterBuilder::new()
            .base_config(base())
            .add_deployment("g", "m", url)
            .build()
            .unwrap(),
    );

    let mut handles = Vec::new();
    for _ in 0..25 {
        let r = Arc::clone(&router);
        handles.push(tokio::spawn(
            async move { r.chat_completions(req("g")).await },
        ));
    }
    for h in handles {
        assert_eq!(h.await.unwrap().unwrap().content(), Some("hi"));
    }
    assert_eq!(hits.load(Ordering::SeqCst), 25);
}
