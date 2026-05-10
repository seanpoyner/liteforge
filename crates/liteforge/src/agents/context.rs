//! Agent context and state management.

use crate::types::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory storage for an agent.
///
/// Provides short-term (conversation) and long-term (persistent) memory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMemory {
    /// Short-term memory (current conversation context).
    pub short_term: Vec<Message>,

    /// Long-term memory (key-value store for persistent facts).
    pub long_term: HashMap<String, serde_json::Value>,

    /// Working memory (temporary scratch space for current task).
    pub working: HashMap<String, serde_json::Value>,
}

impl AgentMemory {
    /// Create a new empty memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to short-term memory.
    pub fn add_message(&mut self, message: Message) {
        self.short_term.push(message);
    }

    /// Get all messages in short-term memory.
    pub fn messages(&self) -> &[Message] {
        &self.short_term
    }

    /// Clear short-term memory (conversation history).
    pub fn clear_short_term(&mut self) {
        self.short_term.clear();
    }

    /// Store a value in long-term memory.
    pub fn remember(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.long_term.insert(key.into(), value);
    }

    /// Retrieve a value from long-term memory.
    pub fn recall(&self, key: &str) -> Option<&serde_json::Value> {
        self.long_term.get(key)
    }

    /// Forget a value from long-term memory.
    pub fn forget(&mut self, key: &str) -> Option<serde_json::Value> {
        self.long_term.remove(key)
    }

    /// Store a value in working memory.
    pub fn set_working(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.working.insert(key.into(), value);
    }

    /// Get a value from working memory.
    pub fn get_working(&self, key: &str) -> Option<&serde_json::Value> {
        self.working.get(key)
    }

    /// Clear working memory.
    pub fn clear_working(&mut self) {
        self.working.clear();
    }

    /// Get the total number of messages in short-term memory.
    pub fn message_count(&self) -> usize {
        self.short_term.len()
    }
}

/// Current state of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is idle, waiting for input.
    #[default]
    Idle,

    /// Agent is thinking (waiting for LLM response).
    Thinking,

    /// Agent is executing a tool.
    ExecutingTool,

    /// Agent is waiting for human input/approval.
    WaitingForHuman,

    /// Agent has completed its task.
    Completed,

    /// Agent encountered an error.
    Error,

    /// Agent was stopped/cancelled.
    Stopped,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentState::Idle => "idle",
            AgentState::Thinking => "thinking",
            AgentState::ExecutingTool => "executing_tool",
            AgentState::WaitingForHuman => "waiting_for_human",
            AgentState::Completed => "completed",
            AgentState::Error => "error",
            AgentState::Stopped => "stopped",
        };
        write!(f, "{}", s)
    }
}

/// Context for agent execution.
///
/// Contains all the state and configuration needed for an agent to run.
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Unique identifier for this agent instance.
    pub agent_id: String,

    /// Current state of the agent.
    pub state: AgentState,

    /// Agent's memory (short-term, long-term, working).
    pub memory: AgentMemory,

    /// Current step number (0-indexed).
    pub current_step: usize,

    /// Maximum number of steps allowed.
    pub max_steps: usize,

    /// System prompt for the agent.
    pub system_prompt: Option<String>,

    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Correlation ID for tracing.
    pub correlation_id: Option<String>,

    /// Whether to preserve conversation history (short-term memory) across resets.
    /// When true, `reset()` will not clear short-term memory, enabling multi-turn
    /// conversations where the agent retains context between `run()` calls.
    pub preserve_history: bool,
}

impl AgentContext {
    /// Create a new agent context.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentState::Idle,
            memory: AgentMemory::new(),
            current_step: 0,
            max_steps: 10,
            system_prompt: None,
            metadata: HashMap::new(),
            correlation_id: None,
            preserve_history: false,
        }
    }

    /// Set the maximum number of steps.
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the correlation ID for tracing.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Configure history preservation for multi-turn conversations.
    pub fn with_preserve_history(mut self, preserve: bool) -> Self {
        self.preserve_history = preserve;
        self
    }

    /// Set whether to preserve history across resets (for runtime configuration).
    pub fn set_preserve_history(&mut self, preserve: bool) {
        self.preserve_history = preserve;
    }

    /// Check if the agent can continue (not at max steps and not in terminal state).
    pub fn can_continue(&self) -> bool {
        self.current_step < self.max_steps
            && !matches!(
                self.state,
                AgentState::Completed | AgentState::Error | AgentState::Stopped
            )
    }

    /// Increment the step counter.
    pub fn increment_step(&mut self) {
        self.current_step += 1;
    }

    /// Get messages for LLM request (with system prompt prepended if set).
    pub fn get_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        if let Some(ref system) = self.system_prompt {
            messages.push(Message::system(system));
        }

        messages.extend(self.memory.short_term.iter().cloned());
        messages
    }

    /// Reset the context for a new task.
    ///
    /// If `preserve_history` is true, short-term memory (conversation history)
    /// is retained. This enables multi-turn conversations where the agent
    /// maintains context between `run()` calls.
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.current_step = 0;
        if !self.preserve_history {
            self.memory.clear_short_term();
        }
        self.memory.clear_working();
    }
}

impl Default for AgentContext {
    fn default() -> Self {
        Self {
            agent_id: generate_agent_id(),
            state: AgentState::Idle,
            memory: AgentMemory::new(),
            current_step: 0,
            max_steps: 10,
            system_prompt: None,
            metadata: HashMap::new(),
            correlation_id: None,
            preserve_history: false,
        }
    }
}

/// Generate a unique agent ID.
fn generate_agent_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("agent_{:x}_{:04x}", ts, count & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_short_term() {
        let mut memory = AgentMemory::new();
        assert_eq!(memory.message_count(), 0);

        memory.add_message(Message::user("Hello"));
        memory.add_message(Message::assistant("Hi there!"));

        assert_eq!(memory.message_count(), 2);
        assert_eq!(memory.messages()[0].content, Some("Hello".to_string()));

        memory.clear_short_term();
        assert_eq!(memory.message_count(), 0);
    }

    #[test]
    fn test_memory_long_term() {
        let mut memory = AgentMemory::new();

        memory.remember("user_name", serde_json::json!("Alice"));
        assert_eq!(
            memory.recall("user_name"),
            Some(&serde_json::json!("Alice"))
        );

        memory.forget("user_name");
        assert_eq!(memory.recall("user_name"), None);
    }

    #[test]
    fn test_context_creation() {
        let ctx = AgentContext::new("test-agent")
            .with_max_steps(5)
            .with_system_prompt("You are helpful.");

        assert_eq!(ctx.agent_id, "test-agent");
        assert_eq!(ctx.max_steps, 5);
        assert_eq!(ctx.system_prompt, Some("You are helpful.".to_string()));
        assert!(ctx.can_continue());
    }

    #[test]
    fn test_context_can_continue() {
        let mut ctx = AgentContext::new("test").with_max_steps(2);

        assert!(ctx.can_continue());

        ctx.increment_step();
        assert!(ctx.can_continue());

        ctx.increment_step();
        assert!(!ctx.can_continue()); // At max steps

        ctx.current_step = 0;
        ctx.state = AgentState::Completed;
        assert!(!ctx.can_continue()); // Terminal state
    }

    #[test]
    fn test_context_get_messages() {
        let mut ctx = AgentContext::new("test").with_system_prompt("System prompt");

        ctx.memory.add_message(Message::user("Hello"));
        ctx.memory.add_message(Message::assistant("Hi!"));

        let messages = ctx.get_messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[2].role, "assistant");
    }

    #[test]
    fn test_agent_state_display() {
        assert_eq!(AgentState::Idle.to_string(), "idle");
        assert_eq!(AgentState::Thinking.to_string(), "thinking");
        assert_eq!(AgentState::ExecutingTool.to_string(), "executing_tool");
    }

    #[test]
    fn test_context_reset_clears_history_by_default() {
        let mut ctx = AgentContext::new("test");
        ctx.memory.add_message(Message::user("Hello"));
        ctx.memory.add_message(Message::assistant("Hi!"));
        ctx.current_step = 3;
        ctx.state = AgentState::Completed;

        ctx.reset();

        assert_eq!(ctx.memory.message_count(), 0);
        assert_eq!(ctx.current_step, 0);
        assert_eq!(ctx.state, AgentState::Idle);
    }

    #[test]
    fn test_context_reset_preserves_history_when_enabled() {
        let mut ctx = AgentContext::new("test").with_preserve_history(true);
        ctx.memory.add_message(Message::user("Hello"));
        ctx.memory.add_message(Message::assistant("Hi!"));
        ctx.current_step = 3;
        ctx.state = AgentState::Completed;

        ctx.reset();

        assert_eq!(ctx.memory.message_count(), 2);
        assert_eq!(ctx.current_step, 0);
        assert_eq!(ctx.state, AgentState::Idle);
    }

    #[test]
    fn test_set_preserve_history() {
        let mut ctx = AgentContext::new("test");
        assert!(!ctx.preserve_history);

        ctx.set_preserve_history(true);
        assert!(ctx.preserve_history);

        ctx.memory.add_message(Message::user("Hello"));
        ctx.reset();
        assert_eq!(ctx.memory.message_count(), 1);

        ctx.set_preserve_history(false);
        ctx.reset();
        assert_eq!(ctx.memory.message_count(), 0);
    }
}
