//! Demonstrates the OTel-instrumented chat path:
//! 1. Initialise OTel exporter (here pointing at a local OTel collector
//!    on `localhost:4318`; in production pass the Dynatrace tenant URL).
//! 2. Create a client with `default_metadata` so every request body
//!    carries `app=btsales` for filtering on the gateway side.
//! 3. Call `chat_completions` with per-request `metadata` for session
//!    correlation. The transport layer merges the two metadata maps
//!    (per-request keys win on collision) and serialises them as the
//!    top-level `metadata` field, confirmed via probe to be the shape
//!    LiteLLM accepts.
//!
//! Build & run:
//! ```bash
//! cargo run -p liteforge --example with_otel --features otel
//! ```
//!
//! With a local collector for inspecting spans:
//! ```bash
//! docker run --rm -p 4318:4318 -p 4317:4317 \
//!     otel/opentelemetry-collector-contrib:latest
//! cargo run -p liteforge --example with_otel --features otel
//! ```
//! You should see a span tree on stdout of the collector:
//! `agent.step → gen_ai.completion → http.client.request`.

use std::collections::HashMap;
use liteforge::{AsyncForgeClient, ChatCompletionRequest, Message, OtelConfig, ForgeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. OTel init -------------------------------------------------
    let mut otel_headers = HashMap::new();
    if let Ok(token) = std::env::var("DYNATRACE_API_TOKEN") {
        otel_headers.insert("Authorization".to_string(), format!("Api-Token {}", token));
    }
    let otel = OtelConfig {
        endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .or_else(|| Some("http://localhost:4318/v1/traces".to_string())),
        headers: otel_headers,
        service_name: Some("liteforge-example".to_string()),
        resource_attributes: HashMap::from([(
            "deployment.environment".to_string(),
            "local".to_string(),
        )]),
        capture_prompts: false,
    };
    liteforge::init_otel(&otel)?;
    println!(
        "otel feature enabled at compile time = {}",
        liteforge::otel_feature_enabled()
    );

    // --- 2. Build a client with sticky metadata -----------------------
    let mut sticky = HashMap::new();
    sticky.insert("app".to_string(), serde_json::json!("btsales"));
    sticky.insert("deployment_env".to_string(), serde_json::json!("local"));

    let config = ForgeConfig::builder()
        .default_metadata(sticky)
        .otel(otel)
        .build();
    let client = AsyncForgeClient::with_config(config);

    // --- 3. Per-request metadata --------------------------------------
    let mut per_request = HashMap::new();
    per_request.insert("session_id".to_string(), serde_json::json!("session-001"));
    per_request.insert("user_eid".to_string(), serde_json::json!("EXAMPLE"));
    per_request.insert("purpose".to_string(), serde_json::json!("agent"));

    let request = ChatCompletionRequest::new(
        std::env::var("LITEFORGE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "claude-haiku-4.5".to_string()),
        vec![Message::user("Reply with the single word 'pong'")],
    )
    .max_tokens(8)
    .metadata(per_request);

    let response = client.chat_completions(request).await?;

    println!(
        "response: {:?}",
        response.choices.first().map(|c| &c.message)
    );
    println!(
        "tokens: input={} output={} total={}",
        response
            .usage
            .as_ref()
            .map(|u| u.prompt_tokens)
            .unwrap_or(0),
        response
            .usage
            .as_ref()
            .map(|u| u.completion_tokens)
            .unwrap_or(0),
        response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
    );

    // Give the batch span exporter a moment to flush before the process
    // exits, otherwise the example may finish faster than the export.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
