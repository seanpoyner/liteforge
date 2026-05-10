//! Agent step types and results.

use crate::types::{Message, ToolCall};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Type of step an agent can take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    /// Agent is thinking (LLM inference).
    Think,

    /// Agent is calling a tool.
    ToolCall {
        /// Name of the tool being called.
        tool_name: String,
        /// Tool call ID.
        call_id: String,
    },

    /// Agent received a tool result.
    ToolResult {
        /// Name of the tool that returned.
        tool_name: String,
        /// Tool call ID.
        call_id: String,
    },

    /// Agent produced a final response.
    Response,

    /// Agent encountered an error.
    Error,

    /// Agent is waiting for human input.
    WaitForHuman,
}

impl std::fmt::Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::Think => write!(f, "think"),
            StepType::ToolCall { tool_name, .. } => write!(f, "tool_call:{}", tool_name),
            StepType::ToolResult { tool_name, .. } => write!(f, "tool_result:{}", tool_name),
            StepType::Response => write!(f, "response"),
            StepType::Error => write!(f, "error"),
            StepType::WaitForHuman => write!(f, "wait_for_human"),
        }
    }
}

/// Result of a single agent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepResult {
    /// Continue with more steps.
    Continue,

    /// Agent produced a final response.
    Done {
        /// The final response message.
        response: String,
    },

    /// Agent needs to call tools.
    ToolCalls {
        /// List of tool calls to execute.
        calls: Vec<ToolCall>,
    },

    /// Agent is waiting for human input/approval.
    WaitForHuman {
        /// Message to show the human.
        prompt: String,
    },

    /// Agent encountered an error.
    Error {
        /// Error message.
        message: String,
    },
}

impl StepResult {
    /// Check if this result indicates the agent is done.
    pub fn is_done(&self) -> bool {
        matches!(self, StepResult::Done { .. })
    }

    /// Check if this result has tool calls.
    pub fn has_tool_calls(&self) -> bool {
        matches!(self, StepResult::ToolCalls { .. })
    }

    /// Check if this result is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, StepResult::Error { .. })
    }

    /// Get the response text if this is a Done result.
    pub fn response(&self) -> Option<&str> {
        match self {
            StepResult::Done { response } => Some(response),
            _ => None,
        }
    }

    /// Get the tool calls if this result has them.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            StepResult::ToolCalls { calls } => Some(calls),
            _ => None,
        }
    }

    /// Get the error message if this is an error.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            StepResult::Error { message } => Some(message),
            _ => None,
        }
    }
}

/// A single step in agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Step number (0-indexed).
    pub step_number: usize,

    /// Type of step.
    pub step_type: StepType,

    /// Input to this step (e.g., user message, tool result).
    pub input: Option<serde_json::Value>,

    /// Output from this step (e.g., LLM response, tool call).
    pub output: Option<serde_json::Value>,

    /// Result of the step.
    pub result: StepResult,

    /// Duration of the step.
    #[serde(with = "duration_serde")]
    pub duration: Duration,

    /// Tokens used (if applicable).
    pub tokens: Option<TokenUsage>,

    /// Any messages produced during this step.
    pub messages: Vec<Message>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Token usage for a step.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens.
    pub prompt_tokens: u32,
    /// Completion tokens.
    pub completion_tokens: u32,
    /// Total tokens.
    pub total_tokens: u32,
}

impl AgentStep {
    /// Create a new step.
    pub fn new(step_number: usize, step_type: StepType) -> Self {
        Self {
            step_number,
            step_type,
            input: None,
            output: None,
            result: StepResult::Continue,
            duration: Duration::ZERO,
            tokens: None,
            messages: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the input.
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the output.
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set the result.
    pub fn with_result(mut self, result: StepResult) -> Self {
        self.result = result;
        self
    }

    /// Set the duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set token usage.
    pub fn with_tokens(mut self, tokens: TokenUsage) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Add a message.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Serde helper for Duration.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_type_display() {
        assert_eq!(StepType::Think.to_string(), "think");
        assert_eq!(
            StepType::ToolCall {
                tool_name: "calc".to_string(),
                call_id: "1".to_string()
            }
            .to_string(),
            "tool_call:calc"
        );
        assert_eq!(StepType::Response.to_string(), "response");
    }

    #[test]
    fn test_step_result_methods() {
        let done = StepResult::Done {
            response: "Hello!".to_string(),
        };
        assert!(done.is_done());
        assert_eq!(done.response(), Some("Hello!"));

        let tool_calls = StepResult::ToolCalls { calls: vec![] };
        assert!(tool_calls.has_tool_calls());
        assert!(!tool_calls.is_done());

        let error = StepResult::Error {
            message: "Oops".to_string(),
        };
        assert!(error.is_error());
        assert_eq!(error.error_message(), Some("Oops"));
    }

    #[test]
    fn test_agent_step_builder() {
        let step = AgentStep::new(0, StepType::Think)
            .with_input(serde_json::json!({"prompt": "Hello"}))
            .with_output(serde_json::json!({"response": "Hi!"}))
            .with_result(StepResult::Done {
                response: "Hi!".to_string(),
            })
            .with_duration(Duration::from_millis(100));

        assert_eq!(step.step_number, 0);
        assert!(step.result.is_done());
        assert_eq!(step.duration.as_millis(), 100);
    }

    #[test]
    fn test_token_usage() {
        let tokens = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        let step = AgentStep::new(0, StepType::Think).with_tokens(tokens);

        assert_eq!(step.tokens.unwrap().total_tokens, 150);
    }
}
