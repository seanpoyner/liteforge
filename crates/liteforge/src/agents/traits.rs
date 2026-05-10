//! Agent trait and configuration.

use super::context::AgentContext;
use super::step::{AgentStep, StepResult};
use crate::error::ForgeError;
use crate::hooks::{HookContext, HookManager};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// Error type for agent operations.
#[derive(Debug)]
pub enum AgentError {
    /// LLM request failed.
    LlmError(ForgeError),

    /// Tool execution failed.
    ToolError { tool_name: String, message: String },

    /// Maximum steps exceeded.
    MaxStepsExceeded { max_steps: usize },

    /// Agent was stopped/cancelled.
    Stopped,

    /// Invalid configuration.
    ConfigError(String),

    /// Other error.
    Other(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::LlmError(e) => write!(f, "LLM error: {}", e),
            AgentError::ToolError { tool_name, message } => {
                write!(f, "Tool '{}' error: {}", tool_name, message)
            }
            AgentError::MaxStepsExceeded { max_steps } => {
                write!(f, "Maximum steps ({}) exceeded", max_steps)
            }
            AgentError::Stopped => write!(f, "Agent was stopped"),
            AgentError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AgentError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AgentError::LlmError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ForgeError> for AgentError {
    fn from(e: ForgeError) -> Self {
        AgentError::LlmError(e)
    }
}

/// Result type for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;

/// Configuration for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name/identifier.
    pub name: String,

    /// System prompt.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Model to use.
    #[serde(default)]
    pub model: Option<String>,

    /// Maximum number of steps.
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,

    /// Temperature for LLM.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Maximum tokens for responses.
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Whether to enable streaming.
    #[serde(default)]
    pub streaming: bool,

    /// Tool names to enable.
    #[serde(default)]
    pub tools: Vec<String>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

fn default_max_steps() -> usize {
    10
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "agent".to_string(),
            system_prompt: None,
            model: None,
            max_steps: default_max_steps(),
            temperature: None,
            max_tokens: None,
            streaming: false,
            tools: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl AgentConfig {
    /// Create a new agent configuration.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the maximum steps.
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming.
    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// Add a tool.
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tools.push(tool_name.into());
        self
    }

    /// Add multiple tools.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tools.extend(tools.into_iter().map(|t| t.into()));
        self
    }
}

/// Trait for implementing agents.
///
/// Agents are stateful entities that can execute multi-step tasks
/// using LLMs and tools.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's name.
    fn name(&self) -> &str;

    /// Get the agent's configuration.
    fn config(&self) -> &AgentConfig;

    /// Get a mutable reference to the agent's context.
    fn context_mut(&mut self) -> &mut AgentContext;

    /// Get an immutable reference to the agent's context.
    fn context(&self) -> &AgentContext;

    /// Optional hook manager for lifecycle events.
    ///
    /// Default returns `None`, meaning no hooks fire. Implementations can
    /// override this to expose a registered `HookManager`. When present,
    /// the default `run()` body will fire `BeforeAgentStart`, `BeforeAgentStep`,
    /// `AfterAgentStep`, and `AfterAgentEnd` events; concrete agents are
    /// responsible for firing `BeforeLlmRequest`, `AfterLlmResponse`,
    /// `BeforeToolCall`, `AfterToolCall` from inside `step()`.
    fn hooks(&self) -> Option<&Arc<HookManager>> {
        None
    }

    /// Run the agent with a user message.
    ///
    /// This is the main entry point for interacting with the agent.
    async fn run(&mut self, input: &str) -> AgentResult<String> {
        // Snapshot the hook manager so we can fire events without holding
        // a borrow on `self` across `step()` calls.
        let hooks = self.hooks().cloned();
        let agent_name = self.name().to_string();

        // BeforeAgentStart
        if let Some(h) = &hooks {
            let r = h.run(&HookContext::agent_start(&agent_name));
            if r.is_abort() {
                return Err(AgentError::Other(
                    r.error_message().unwrap_or("aborted by hook").to_string(),
                ));
            }
        }

        // Add user message to memory
        self.context_mut()
            .memory
            .add_message(crate::types::Message::user(input));

        // Run the step loop
        let outcome: AgentResult<String> = loop {
            // BeforeAgentStep
            if let Some(h) = &hooks {
                let r = h.run(&HookContext::before_step(self.context().current_step));
                if r.is_abort() {
                    break Err(AgentError::Other(
                        r.error_message().unwrap_or("aborted by hook").to_string(),
                    ));
                }
            }

            // Check if we can continue
            if !self.context().can_continue() {
                if self.context().current_step >= self.context().max_steps {
                    break Err(AgentError::MaxStepsExceeded {
                        max_steps: self.context().max_steps,
                    });
                }
                break Err(AgentError::Other(
                    "Agent did not produce a response".to_string(),
                ));
            }

            // Execute a step
            let step = match self.step().await {
                Ok(s) => s,
                Err(e) => break Err(e),
            };

            // AfterAgentStep
            if let Some(h) = &hooks {
                let _ = h.run(&HookContext::after_step(
                    self.context().current_step,
                    &step.step_type.to_string(),
                ));
            }

            // Check the result
            match &step.result {
                StepResult::Done { response } => {
                    break Ok(response.clone());
                }
                StepResult::Error { message } => {
                    break Err(AgentError::Other(message.clone()));
                }
                StepResult::WaitForHuman { .. } => {
                    break Err(AgentError::Other(
                        "Human input required but not supported".to_string(),
                    ));
                }
                StepResult::Continue | StepResult::ToolCalls { .. } => {
                    self.context_mut().increment_step();
                }
            }
        };

        // AfterAgentEnd (always fires, even on error)
        if let Some(h) = &hooks {
            let final_answer: String = match &outcome {
                Ok(s) => s.clone(),
                Err(e) => e.to_string(),
            };
            let total_steps = self.context().current_step;
            let _ = h.run(&HookContext::agent_end(
                &agent_name,
                &final_answer,
                total_steps,
            ));
        }

        outcome
    }

    /// Execute a single step.
    ///
    /// This should be implemented by concrete agent types.
    async fn step(&mut self) -> AgentResult<AgentStep>;

    /// Stop the agent.
    fn stop(&mut self) {
        self.context_mut().state = super::context::AgentState::Stopped;
    }

    /// Reset the agent for a new task.
    fn reset(&mut self) {
        self.context_mut().reset();
    }

    /// Run the agent with prior conversation history.
    ///
    /// This enables multi-turn conversations by injecting prior messages
    /// into the agent's memory before processing the current input.
    /// Messages are added in order, then the current input is processed.
    ///
    /// # Example
    /// ```ignore
    /// let history = vec![
    ///     Message::user("What is the capital of France?"),
    ///     Message::assistant("The capital of France is Paris."),
    /// ];
    /// let response = agent.run_with_history("And what about Germany?", history).await?;
    /// // The agent sees the prior context and can answer "Berlin"
    /// ```
    async fn run_with_history(
        &mut self,
        input: &str,
        history: Vec<crate::types::Message>,
    ) -> AgentResult<String> {
        for msg in history {
            self.context_mut().memory.add_message(msg);
        }
        self.run(input).await
    }

    /// Load conversation history into the agent's memory.
    ///
    /// This is useful when you want to inject prior context before calling
    /// `run()` multiple times. Unlike `run_with_history()`, this doesn't
    /// immediately run the agent.
    fn load_history(&mut self, history: Vec<crate::types::Message>) {
        for msg in history {
            self.context_mut().memory.add_message(msg);
        }
    }

    /// Get the current conversation messages from the agent's memory.
    ///
    /// This can be used to persist conversation state externally (e.g., to
    /// a database) between sessions.
    fn conversation_messages(&self) -> Vec<crate::types::Message> {
        self.context().memory.short_term.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::new("test-agent")
            .with_system_prompt("You are helpful")
            .with_model("gpt-4")
            .with_max_steps(5)
            .with_temperature(0.7)
            .with_tool("calculator")
            .with_tool("search");

        assert_eq!(config.name, "test-agent");
        assert_eq!(config.system_prompt, Some("You are helpful".to_string()));
        assert_eq!(config.model, Some("gpt-4".to_string()));
        assert_eq!(config.max_steps, 5);
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.tools, vec!["calculator", "search"]);
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::MaxStepsExceeded { max_steps: 10 };
        assert_eq!(err.to_string(), "Maximum steps (10) exceeded");

        let err = AgentError::ToolError {
            tool_name: "calc".to_string(),
            message: "division by zero".to_string(),
        };
        assert_eq!(err.to_string(), "Tool 'calc' error: division by zero");
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_steps, 10);
        assert!(config.tools.is_empty());
        assert!(!config.streaming);
    }
}
