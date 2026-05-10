//! Hook manager for registering and executing hooks.

use super::types::{Hook, HookContext, HookEvent, HookResult};
use std::sync::RwLock;

/// Manages a collection of hooks and executes them in order.
///
/// Hooks are executed in priority order (lower priority runs first).
/// If any hook returns Skip, SkipWith, or Abort, execution stops.
///
/// # Example
///
/// ```
/// use liteforge::hooks::{Hook, HookContext, HookResult, HookManager, HookEvent};
///
/// struct LogHook;
/// impl Hook for LogHook {
///     fn name(&self) -> &str { "log" }
///     fn on_before_tool_call(&self, ctx: &HookContext) -> HookResult {
///         println!("Tool call: {:?}", ctx.data);
///         HookResult::Continue
///     }
/// }
///
/// let mut manager = HookManager::new();
/// manager.register(Box::new(LogHook));
///
/// let ctx = HookContext::new(HookEvent::BeforeToolCall);
/// let result = manager.run(&ctx);
/// assert!(result.should_continue());
/// ```
pub struct HookManager {
    hooks: RwLock<Vec<Box<dyn Hook>>>,
}

impl HookManager {
    /// Create a new empty hook manager.
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Register a hook.
    pub fn register(&self, hook: Box<dyn Hook>) {
        let mut hooks = self.hooks.write().unwrap();
        hooks.push(hook);
        // Sort by priority (lower first)
        hooks.sort_by_key(|h| h.priority());
    }

    /// Unregister a hook by name.
    ///
    /// Returns true if a hook was removed.
    pub fn unregister(&self, name: &str) -> bool {
        let mut hooks = self.hooks.write().unwrap();
        let len_before = hooks.len();
        hooks.retain(|h| h.name() != name);
        hooks.len() < len_before
    }

    /// Run all hooks for an event.
    ///
    /// Hooks are executed in priority order. Execution stops if any
    /// hook returns Skip, SkipWith, or Abort.
    pub fn run(&self, ctx: &HookContext) -> HookResult {
        let hooks = self.hooks.read().unwrap();

        for hook in hooks.iter() {
            let result = hook.on_event(ctx);
            match &result {
                HookResult::Continue => continue,
                HookResult::ContinueWith(_) => continue,
                _ => return result,
            }
        }

        HookResult::Continue
    }

    /// Run hooks with mutable context, allowing hooks to modify data.
    ///
    /// If a hook returns ContinueWith, the context data is updated
    /// before calling the next hook.
    pub fn run_mut(&self, ctx: &mut HookContext) -> HookResult {
        let hooks = self.hooks.read().unwrap();

        for hook in hooks.iter() {
            let result = hook.on_event(ctx);
            match result {
                HookResult::Continue => continue,
                HookResult::ContinueWith(data) => {
                    ctx.data = data;
                    continue;
                }
                other => return other,
            }
        }

        HookResult::Continue
    }

    /// Get the number of registered hooks.
    pub fn len(&self) -> usize {
        self.hooks.read().unwrap().len()
    }

    /// Check if no hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.hooks.read().unwrap().is_empty()
    }

    /// Get the names of all registered hooks.
    pub fn hook_names(&self) -> Vec<String> {
        self.hooks
            .read()
            .unwrap()
            .iter()
            .map(|h| h.name().to_string())
            .collect()
    }

    /// Clear all hooks.
    pub fn clear(&self) {
        self.hooks.write().unwrap().clear();
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to create a context for tool calls
impl HookContext {
    /// Create a context for a tool call.
    pub fn tool_call(tool_name: &str, arguments: &str) -> Self {
        Self::new(HookEvent::BeforeToolCall).data(serde_json::json!({
            "tool": tool_name,
            "arguments": arguments
        }))
    }

    /// Create a context for a tool result.
    pub fn tool_result(tool_name: &str, result: serde_json::Value) -> Self {
        Self::new(HookEvent::AfterToolCall).data(serde_json::json!({
            "tool": tool_name,
            "result": result
        }))
    }

    /// Create a context for an LLM request.
    pub fn llm_request(model: &str, messages: &[serde_json::Value]) -> Self {
        Self::new(HookEvent::BeforeLlmRequest).data(serde_json::json!({
            "model": model,
            "message_count": messages.len()
        }))
    }

    /// Create a context for an agent start.
    pub fn agent_start(agent_id: &str) -> Self {
        Self::new(HookEvent::BeforeAgentStart).data(serde_json::json!({"agent_id": agent_id}))
    }

    /// Create a context for an agent end (final answer produced).
    pub fn agent_end(agent_id: &str, final_answer: &str, total_steps: usize) -> Self {
        Self::new(HookEvent::AfterAgentEnd).data(serde_json::json!({
            "agent_id": agent_id,
            "final_answer": final_answer,
            "total_steps": total_steps,
        }))
    }

    /// Create a context fired before each agent step iteration.
    pub fn before_step(step_index: usize) -> Self {
        Self::new(HookEvent::BeforeAgentStep).data(serde_json::json!({
            "step_index": step_index,
        }))
    }

    /// Create a context fired after each agent step iteration.
    pub fn after_step(step_index: usize, step_type: &str) -> Self {
        Self::new(HookEvent::AfterAgentStep).data(serde_json::json!({
            "step_index": step_index,
            "step_type": step_type,
        }))
    }

    /// Create a context fired after the LLM returns a chat completion.
    pub fn llm_response(
        content: &str,
        tool_call_count: usize,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Self {
        Self::new(HookEvent::AfterLlmResponse).data(serde_json::json!({
            "content": content,
            "tool_call_count": tool_call_count,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CounterHook {
        name: String,
        priority: i32,
    }

    impl Hook for CounterHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn on_before_tool_call(&self, ctx: &HookContext) -> HookResult {
            // Add hook name to track execution order
            if let Some(order) = ctx.metadata.get("order") {
                if let Some(arr) = order.as_array() {
                    let mut new_arr = arr.clone();
                    new_arr.push(serde_json::json!(self.name));
                    return HookResult::ContinueWith(serde_json::json!({"order": new_arr}));
                }
            }
            HookResult::Continue
        }
    }

    struct BlockingHook;

    impl Hook for BlockingHook {
        fn name(&self) -> &str {
            "blocker"
        }

        fn on_before_tool_call(&self, ctx: &HookContext) -> HookResult {
            if ctx.get_str("tool") == Some("dangerous") {
                HookResult::Abort("Dangerous tool blocked".to_string())
            } else {
                HookResult::Continue
            }
        }
    }

    #[test]
    fn test_manager_register() {
        let manager = HookManager::new();
        assert!(manager.is_empty());

        manager.register(Box::new(CounterHook {
            name: "test".to_string(),
            priority: 100,
        }));

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.hook_names(), vec!["test"]);
    }

    #[test]
    fn test_manager_unregister() {
        let manager = HookManager::new();
        manager.register(Box::new(CounterHook {
            name: "test".to_string(),
            priority: 100,
        }));

        assert!(manager.unregister("test"));
        assert!(manager.is_empty());
        assert!(!manager.unregister("nonexistent"));
    }

    #[test]
    fn test_priority_order() {
        let manager = HookManager::new();

        // Register in reverse priority order
        manager.register(Box::new(CounterHook {
            name: "low".to_string(),
            priority: 200,
        }));
        manager.register(Box::new(CounterHook {
            name: "high".to_string(),
            priority: 50,
        }));
        manager.register(Box::new(CounterHook {
            name: "mid".to_string(),
            priority: 100,
        }));

        // Should be sorted by priority
        let names = manager.hook_names();
        assert_eq!(names, vec!["high", "mid", "low"]);
    }

    #[test]
    fn test_run_continues() {
        let manager = HookManager::new();
        manager.register(Box::new(CounterHook {
            name: "a".to_string(),
            priority: 100,
        }));
        manager.register(Box::new(CounterHook {
            name: "b".to_string(),
            priority: 100,
        }));

        let ctx = HookContext::tool_call("calc", "{}");
        let result = manager.run(&ctx);
        assert!(result.should_continue());
    }

    #[test]
    fn test_run_aborts() {
        let manager = HookManager::new();
        manager.register(Box::new(BlockingHook));

        let ctx = HookContext::tool_call("dangerous", "{}");
        let result = manager.run(&ctx);

        assert!(result.is_abort());
        assert_eq!(result.error_message(), Some("Dangerous tool blocked"));
    }

    #[test]
    fn test_clear() {
        let manager = HookManager::new();
        manager.register(Box::new(CounterHook {
            name: "test".to_string(),
            priority: 100,
        }));

        manager.clear();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_context_helpers() {
        let ctx = HookContext::tool_call("calc", r#"{"x": 1}"#);
        assert_eq!(ctx.get_str("tool"), Some("calc"));

        let ctx = HookContext::agent_start("agent-1");
        assert_eq!(ctx.get_str("agent_id"), Some("agent-1"));
    }
}
