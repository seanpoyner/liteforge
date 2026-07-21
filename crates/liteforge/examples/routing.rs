//! Offline demo of native model routing (Layer 1 + Layer 2).
//!
//! Run with: `cargo run -p liteforge --features model-routing --example routing`
//!
//! This example uses a mock embedding backend so it runs without any network or
//! API key. It shows a semantic selector routing prompts to model groups, and
//! the Layer-1 router resolving each to a concrete deployment.

#[cfg(feature = "model-routing")]
#[tokio::main]
async fn main() -> liteforge::Result<()> {
    use liteforge::model_routing::{EmbeddingSource, SemanticRoute, SemanticSelector};
    use liteforge::routing::{Router, RoutingStrategy};
    use liteforge::{ChatCompletionRequest, ForgeConfig, Message};
    use std::sync::Arc;

    // A deterministic mock embedder: maps a couple of keywords to basis vectors.
    let embedder = Arc::new(EmbeddingSource::mock(3, |t| {
        let t = t.to_lowercase();
        if t.contains("code") || t.contains("refactor") || t.contains("bug") {
            vec![1.0, 0.0, 0.0]
        } else if t.contains("poem") || t.contains("story") || t.contains("write") {
            vec![0.0, 1.0, 0.0]
        } else {
            vec![0.0, 0.0, 1.0]
        }
    }));

    let selector = SemanticSelector::build(
        embedder,
        vec![
            SemanticRoute::new(
                "premium",
                vec!["refactor this module".into(), "fix this bug".into()],
            ),
            SemanticRoute::new(
                "balanced",
                vec!["write a poem".into(), "tell a story".into()],
            ),
        ],
        Some("cheap".into()),
        0.5,
    )
    .await?;

    let base = ForgeConfig::builder()
        .api_key("demo")
        .base_url("https://litellm.poyner.ai/v1")
        .build();

    let router = Router::builder()
        .base_config(base)
        .strategy(RoutingStrategy::RoundRobin)
        .add_deployment("premium", "claude-opus-4.7", "https://litellm.poyner.ai/v1")
        .add_deployment(
            "balanced",
            "claude-sonnet-4.6",
            "https://litellm.poyner.ai/v1",
        )
        .add_deployment("cheap", "claude-haiku-4.5", "https://litellm.poyner.ai/v1")
        .fallback("premium", vec!["balanced".into(), "cheap".into()])
        .build()?
        .with_selector(Arc::from(selector));

    let prompts = [
        "Please refactor this 500-line file",
        "Write a short story about the sea",
        "What's the weather like?",
    ];

    println!("Model groups: {:?}\n", router.model_groups());
    for p in prompts {
        let req = ChatCompletionRequest::new("auto", vec![Message::user(p)]);
        let decision = router.route_decision(&req).await?;
        println!("prompt:   {p}");
        println!(
            "  -> group={} model={} (strategy={}, chain={:?})\n",
            decision.group, decision.model, decision.strategy, decision.fallback_chain
        );
    }

    Ok(())
}

#[cfg(not(feature = "model-routing"))]
fn main() {
    eprintln!("This example requires: --features model-routing");
}
