//! Model types.

use serde::{Deserialize, Serialize};

/// Capabilities of a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Whether the model supports tool calls.
    pub supports_tools: bool,
    /// Whether the model supports vision (image inputs).
    pub supports_vision: bool,
    /// Whether the model supports streaming responses.
    pub supports_streaming: bool,
    /// Whether the model supports JSON output mode.
    pub supports_json_mode: bool,
    /// Whether the model supports thinking/reasoning mode.
    pub supports_thinking: bool,
    /// Whether the model supports output configuration.
    pub supports_output_config: bool,
    /// Maximum context window size in tokens.
    pub context_window: Option<u32>,
    /// Maximum number of tokens that can be generated.
    pub max_output_tokens: Option<u32>,
    /// Style of thinking/reasoning the model uses.
    pub thinking_style: Option<String>,
}

/// A model available for completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier.
    pub id: String,

    /// Object type (always "model").
    pub object: String,

    /// Unix timestamp when the model was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// The organization that owns the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

/// A list of models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    /// Object type (always "list").
    pub object: String,

    /// The list of models.
    pub data: Vec<Model>,
}

impl ModelList {
    /// Get model IDs as a vector.
    pub fn ids(&self) -> Vec<&str> {
        self.data.iter().map(|m| m.id.as_str()).collect()
    }

    /// Find a model by ID.
    pub fn find(&self, id: &str) -> Option<&Model> {
        self.data.iter().find(|m| m.id == id)
    }
}
