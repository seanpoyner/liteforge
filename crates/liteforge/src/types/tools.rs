//! Tool types for function calling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A function call within a tool call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call.
    #[serde(default)]
    pub name: String,

    /// The arguments to pass to the function as a JSON string.
    #[serde(default)]
    pub arguments: String,
}

impl FunctionCall {
    /// Parse the arguments as JSON.
    pub fn parse_arguments(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Index of this tool call (used in streaming deltas to identify which call is being updated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,

    /// The unique identifier for this tool call.
    #[serde(default)]
    pub id: String,

    /// The type of tool call (always "function").
    #[serde(rename = "type", default)]
    pub call_type: String,

    /// The function to call.
    #[serde(default)]
    pub function: FunctionCall,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            index: None,
            id: id.into(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }

    /// Parse the function arguments as JSON.
    pub fn parse_arguments(&self) -> Result<Value, serde_json::Error> {
        self.function.parse_arguments()
    }
}

// --- Tool definition types (for requests) ---

/// JSON Schema for tool parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    /// Schema type (always "object").
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Parameter properties.
    pub properties: serde_json::Map<String, Value>,

    /// Required parameter names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl Default for ToolParameters {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: serde_json::Map::new(),
            required: None,
        }
    }
}

/// A function definition for tool use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function.
    pub name: String,

    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The parameters the function accepts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ToolParameters>,
}

/// A tool definition for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function definition.
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: None,
                parameters: None,
            },
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.function.description = Some(description.into());
        self
    }

    /// Set the parameters.
    pub fn parameters(mut self, parameters: ToolParameters) -> Self {
        self.function.parameters = Some(parameters);
        self
    }
}
