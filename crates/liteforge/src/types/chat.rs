//! Chat completion types.

use serde::{Deserialize, Serialize};

use super::tools::ToolCall;

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author (system, user, assistant, tool).
    pub role: String,

    /// The content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Optional name for the participant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID for tool responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool response message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,

    /// Number of tokens in the completion.
    pub completion_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Index of this choice.
    pub index: u32,

    /// The generated message.
    pub message: Message,

    /// Reason the generation stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletion {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type (always "chat.completion").
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Model used for the completion.
    pub model: String,

    /// List of completion choices.
    pub choices: Vec<Choice>,

    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl ChatCompletion {
    /// Get the content of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.message.content.as_deref())
    }

    /// Get the first choice's message.
    pub fn message(&self) -> Option<&Message> {
        self.choices.first().map(|c| &c.message)
    }
}

// --- Streaming types ---

/// Delta content in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChoiceDelta {
    /// The role (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// The content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool calls delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A streaming choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index of this choice.
    pub index: u32,

    /// The delta content.
    pub delta: ChoiceDelta,

    /// Reason the generation stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A streaming chunk from a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type (always "chat.completion.chunk").
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// Model used for the completion.
    pub model: String,

    /// List of streaming choices.
    pub choices: Vec<StreamChoice>,
}

impl ChatCompletionChunk {
    /// Get the content delta of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_deref())
    }
}

// --- Request types ---

/// Request body for chat completions.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    /// The model to use.
    pub model: String,

    /// The messages to send.
    pub messages: Vec<Message>,

    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<super::tools::ToolDefinition>>,

    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// User identifier for tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Per-request metadata serialised as the top-level `metadata` field
    /// in the request body. The LiteLLM gateway forwards this
    /// dict as OTel span attributes, which is the supported channel for
    /// tagging gateway spans with caller context (session_id, user_eid,
    /// purpose, etc.), confirmed via endpoint probe (`extra_body` is
    /// rejected by Bedrock-routed models).
    ///
    /// The transport layer additionally merges
    /// [`ForgeConfig::default_metadata`](crate::ForgeConfig::default_metadata)
    /// into the same field; per-request keys win on collision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl ChatCompletionRequest {
    /// Create a new request with required fields.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            stream: None,
            tools: None,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            metadata: None,
        }
    }

    /// Set per-request metadata. Merged with
    /// [`ForgeConfig::default_metadata`](crate::ForgeConfig::default_metadata)
    /// at transport time.
    pub fn metadata(
        mut self,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the max tokens.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set tools.
    pub fn tools(mut self, tools: Vec<super::tools::ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }
}
