//! Hook types and traits.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of hook events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before an agent starts execution.
    BeforeAgentStart,
    /// After an agent completes execution.
    AfterAgentEnd,
    /// Before each agent step.
    BeforeAgentStep,
    /// After each agent step.
    AfterAgentStep,

    /// Before a tool is called.
    BeforeToolCall,
    /// After a tool returns.
    AfterToolCall,

    /// Before an LLM request.
    BeforeLlmRequest,
    /// After an LLM response.
    AfterLlmResponse,

    /// Before a knowledge search.
    BeforeKnowledgeSearch,
    /// After a knowledge search.
    AfterKnowledgeSearch,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HookEvent::BeforeAgentStart => "before_agent_start",
            HookEvent::AfterAgentEnd => "after_agent_end",
            HookEvent::BeforeAgentStep => "before_agent_step",
            HookEvent::AfterAgentStep => "after_agent_step",
            HookEvent::BeforeToolCall => "before_tool_call",
            HookEvent::AfterToolCall => "after_tool_call",
            HookEvent::BeforeLlmRequest => "before_llm_request",
            HookEvent::AfterLlmResponse => "after_llm_response",
            HookEvent::BeforeKnowledgeSearch => "before_knowledge_search",
            HookEvent::AfterKnowledgeSearch => "after_knowledge_search",
        };
        write!(f, "{}", s)
    }
}

/// Context passed to hooks.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// The hook event type.
    pub event: HookEvent,

    /// Associated data (tool name, agent ID, etc.).
    pub data: serde_json::Value,

    /// Correlation ID for tracing.
    pub correlation_id: Option<String>,

    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookContext {
    /// Create a new hook context.
    pub fn new(event: HookEvent) -> Self {
        Self {
            event,
            data: serde_json::Value::Null,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the data.
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Set the correlation ID.
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get a string value from the data.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Get a numeric value from the data.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(|v| v.as_i64())
    }
}

/// Result from a hook execution.
#[derive(Debug, Clone, Default)]
pub enum HookResult {
    /// Continue with normal execution.
    #[default]
    Continue,

    /// Continue but with modified data.
    ContinueWith(serde_json::Value),

    /// Skip the operation (e.g., skip tool call).
    Skip,

    /// Skip and return this value instead.
    SkipWith(serde_json::Value),

    /// Abort with an error.
    Abort(String),
}

impl HookResult {
    /// Check if this result allows continuation.
    pub fn should_continue(&self) -> bool {
        matches!(self, HookResult::Continue | HookResult::ContinueWith(_))
    }

    /// Check if this result skips the operation.
    pub fn should_skip(&self) -> bool {
        matches!(self, HookResult::Skip | HookResult::SkipWith(_))
    }

    /// Check if this result aborts.
    pub fn is_abort(&self) -> bool {
        matches!(self, HookResult::Abort(_))
    }

    /// Get the modified data if any.
    pub fn modified_data(&self) -> Option<&serde_json::Value> {
        match self {
            HookResult::ContinueWith(data) | HookResult::SkipWith(data) => Some(data),
            _ => None,
        }
    }

    /// Get the error message if abort.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            HookResult::Abort(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Trait for implementing hooks.
///
/// Implement specific methods to intercept events.
/// Default implementations return `HookResult::Continue`.
pub trait Hook: Send + Sync {
    /// The name of this hook (for logging/debugging).
    fn name(&self) -> &str;

    /// Priority (lower runs first). Default is 100.
    fn priority(&self) -> i32 {
        100
    }

    /// Called before an agent starts.
    fn on_before_agent_start(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called after an agent ends.
    fn on_after_agent_end(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called before each agent step.
    fn on_before_agent_step(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called after each agent step.
    fn on_after_agent_step(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called before a tool is executed.
    fn on_before_tool_call(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called after a tool returns.
    fn on_after_tool_call(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called before an LLM request.
    fn on_before_llm_request(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called after an LLM response.
    fn on_after_llm_response(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called before a knowledge search.
    fn on_before_knowledge_search(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Called after a knowledge search.
    fn on_after_knowledge_search(&self, _ctx: &HookContext) -> HookResult {
        HookResult::Continue
    }

    /// Generic hook method that dispatches to specific handlers.
    fn on_event(&self, ctx: &HookContext) -> HookResult {
        match ctx.event {
            HookEvent::BeforeAgentStart => self.on_before_agent_start(ctx),
            HookEvent::AfterAgentEnd => self.on_after_agent_end(ctx),
            HookEvent::BeforeAgentStep => self.on_before_agent_step(ctx),
            HookEvent::AfterAgentStep => self.on_after_agent_step(ctx),
            HookEvent::BeforeToolCall => self.on_before_tool_call(ctx),
            HookEvent::AfterToolCall => self.on_after_tool_call(ctx),
            HookEvent::BeforeLlmRequest => self.on_before_llm_request(ctx),
            HookEvent::AfterLlmResponse => self.on_after_llm_response(ctx),
            HookEvent::BeforeKnowledgeSearch => self.on_before_knowledge_search(ctx),
            HookEvent::AfterKnowledgeSearch => self.on_after_knowledge_search(ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHook;

    impl Hook for TestHook {
        fn name(&self) -> &str {
            "test"
        }

        fn on_before_tool_call(&self, ctx: &HookContext) -> HookResult {
            if ctx.get_str("tool") == Some("blocked") {
                HookResult::Skip
            } else {
                HookResult::Continue
            }
        }
    }

    #[test]
    fn test_hook_context() {
        let ctx = HookContext::new(HookEvent::BeforeToolCall)
            .data(serde_json::json!({"tool": "calculator"}))
            .correlation_id("corr-123");

        assert_eq!(ctx.event, HookEvent::BeforeToolCall);
        assert_eq!(ctx.get_str("tool"), Some("calculator"));
        assert_eq!(ctx.correlation_id, Some("corr-123".to_string()));
    }

    #[test]
    fn test_hook_result() {
        assert!(HookResult::Continue.should_continue());
        assert!(HookResult::ContinueWith(serde_json::json!({})).should_continue());
        assert!(HookResult::Skip.should_skip());
        assert!(HookResult::Abort("error".to_string()).is_abort());
    }

    #[test]
    fn test_hook_dispatch() {
        let hook = TestHook;

        let ctx = HookContext::new(HookEvent::BeforeToolCall)
            .data(serde_json::json!({"tool": "calculator"}));
        assert!(hook.on_event(&ctx).should_continue());

        let ctx = HookContext::new(HookEvent::BeforeToolCall)
            .data(serde_json::json!({"tool": "blocked"}));
        assert!(hook.on_event(&ctx).should_skip());
    }
}
