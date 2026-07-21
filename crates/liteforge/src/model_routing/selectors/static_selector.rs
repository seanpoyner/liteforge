//! Pass-through selector: routes to the model the caller requested.

use crate::error::Result;
use crate::routing::{ModelSelector, ScoredGroup, SelectionContext};
use async_trait::async_trait;

/// A selector that simply returns the requested model as the single group.
///
/// This is the default that preserves a plain client's behaviour and is the
/// recommended fallback for latency-critical paths.
#[derive(Debug, Default, Clone)]
pub struct StaticSelector;

impl StaticSelector {
    /// Create a static selector.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ModelSelector for StaticSelector {
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
        Ok(vec![
            ScoredGroup::new(ctx.requested_model(), 1.0).with_reason("static passthrough")
        ])
    }

    fn name(&self) -> &str {
        "static"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatCompletionRequest, Message};

    #[tokio::test]
    async fn returns_requested_model() {
        let req = ChatCompletionRequest::new("premium", vec![Message::user("hi")]);
        let ctx = SelectionContext::new(&req);
        let out = StaticSelector::new().select(&ctx).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].group, "premium");
    }
}
