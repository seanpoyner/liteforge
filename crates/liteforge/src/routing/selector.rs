//! The Layer-2 selection seam.
//!
//! A [`ModelSelector`] inspects an incoming request and returns model groups
//! in preference order. The [`Router`](super::Router) load-balances within the
//! top group and falls back down the ranking on failure. This trait lives in
//! the Layer-1 `routing` module (not `model_routing`) so the router can depend
//! on it without a cycle; the concrete selectors live in `model_routing`.

use crate::error::Result;
use crate::types::ChatCompletionRequest;
use async_trait::async_trait;

/// A model group with a preference score and optional explanation.
#[derive(Debug, Clone)]
pub struct ScoredGroup {
    /// The model group name (matches a `model_name` in the router's model_list).
    pub group: String,
    /// Preference score; higher is more preferred. Comparable within one
    /// selector's output, not necessarily across selectors.
    pub score: f32,
    /// Optional human-readable reason for observability.
    pub reason: Option<String>,
}

impl ScoredGroup {
    /// Create a scored group.
    pub fn new(group: impl Into<String>, score: f32) -> Self {
        Self {
            group: group.into(),
            score,
            reason: None,
        }
    }

    /// Attach a reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Borrowed context handed to a selector. Kept minimal and object-safe.
#[derive(Debug, Clone, Copy)]
pub struct SelectionContext<'a> {
    /// The incoming request.
    pub request: &'a ChatCompletionRequest,
}

impl<'a> SelectionContext<'a> {
    /// Wrap a request.
    pub fn new(request: &'a ChatCompletionRequest) -> Self {
        Self { request }
    }

    /// The model the caller asked for (often a routing alias like `auto`).
    pub fn requested_model(&self) -> &str {
        &self.request.model
    }

    /// The text a selector should embed / classify: the last user message,
    /// falling back to the last message of any role.
    pub fn prompt_text(&self) -> String {
        self.request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .or_else(|| self.request.messages.last())
            .and_then(|m| m.content.clone())
            .unwrap_or_default()
    }
}

/// Chooses model groups for a request, in preference order.
#[async_trait]
pub trait ModelSelector: Send + Sync {
    /// Return a ranked list (best first) of candidate groups. An empty list
    /// means the selector abstains and the router uses `request.model` as-is.
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>>;

    /// Stable identifier for logging / metrics.
    fn name(&self) -> &str {
        "selector"
    }
}

/// The outcome of a routing decision, for introspection (`forge route test`,
/// `which_model`, and response headers).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RouteDecision {
    /// The model group that was (or would be) tried first.
    pub group: String,
    /// The concrete deployment model id chosen within that group.
    pub model: String,
    /// The base URL of the chosen deployment.
    pub base_url: String,
    /// The selection strategy name.
    pub strategy: String,
    /// The selector's score for the chosen group, if a selector ran.
    pub score: Option<f32>,
    /// The full ordered list of groups the router would try (incl. fallbacks).
    pub fallback_chain: Vec<String>,
}
