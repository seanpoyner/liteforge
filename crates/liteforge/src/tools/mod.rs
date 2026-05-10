//! Tool framework for function calling with LLMs.
//!
//! This module provides a complete tool framework including:
//! - [`Tool`] trait for defining callable tools
//! - [`ToolRegistry`] for managing collections of tools
//! - [`ToolExecutor`] for executing tools with validation
//!
//! # Example
//!
//! ```
//! use liteforge::tools::{Tool, ToolRegistry, ToolExecutor};
//! use liteforge::types::ToolDefinition;
//! use serde_json::{json, Value};
//!
//! // Define a simple tool
//! struct WeatherTool;
//!
//! impl Tool for WeatherTool {
//!     fn name(&self) -> &str { "get_weather" }
//!     fn description(&self) -> &str { "Get weather for a location" }
//!     fn parameters_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "location": { "type": "string", "description": "City name" }
//!             },
//!             "required": ["location"]
//!         })
//!     }
//!     fn execute(&self, args: Value) -> Result<Value, String> {
//!         let location = args["location"].as_str().unwrap_or("unknown");
//!         Ok(json!({ "temperature": 72, "location": location }))
//!     }
//! }
//!
//! // Register and use
//! let mut registry = ToolRegistry::new();
//! registry.register(Box::new(WeatherTool));
//!
//! let executor = ToolExecutor::new(registry);
//! let result = executor.execute("get_weather", json!({"location": "Paris"}));
//! ```

mod executor;
mod registry;
mod schema;

pub use executor::{ToolExecutor, ToolResult};
pub use registry::ToolRegistry;
pub use schema::{validate_json_schema, SchemaValidationError};

// Re-export types from the types module for convenience
pub use crate::types::{
    FunctionCall, FunctionDefinition, ToolCall, ToolDefinition, ToolParameters,
};

use serde_json::Value;

/// A callable tool that can be invoked by an LLM.
pub trait Tool: Send + Sync {
    /// The unique name of this tool.
    fn name(&self) -> &str;

    /// A description of what this tool does.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters as a raw JSON value.
    ///
    /// This should return an object with "type", "properties", and optionally "required".
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given arguments.
    fn execute(&self, args: Value) -> Result<Value, String>;

    /// Whether this tool requires confirmation before execution.
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Convert to a tool definition for the OpenAI API.
    fn to_definition(&self) -> ToolDefinition {
        let schema = self.parameters_schema();

        // Convert JSON schema to ToolParameters
        let parameters = if let Some(obj) = schema.as_object() {
            let properties = obj
                .get("properties")
                .and_then(|p| p.as_object())
                .cloned()
                .unwrap_or_default();

            let required = obj.get("required").and_then(|r| r.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(ToolParameters {
                schema_type: "object".to_string(),
                properties,
                required,
            })
        } else {
            None
        };

        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: Some(self.description().to_string()),
                parameters,
            },
        }
    }
}

/// A simple function-based tool.
pub struct FnTool<F>
where
    F: Fn(Value) -> Result<Value, String> + Send + Sync,
{
    name: String,
    description: String,
    parameters: Value,
    func: F,
    requires_confirmation: bool,
}

impl<F> FnTool<F>
where
    F: Fn(Value) -> Result<Value, String> + Send + Sync,
{
    /// Create a new function-based tool.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
        func: F,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            func,
            requires_confirmation: false,
        }
    }

    /// Set whether this tool requires confirmation.
    pub fn requires_confirmation(mut self, requires: bool) -> Self {
        self.requires_confirmation = requires;
        self
    }
}

impl<F> Tool for FnTool<F>
where
    F: Fn(Value) -> Result<Value, String> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.parameters.clone()
    }

    fn execute(&self, args: Value) -> Result<Value, String> {
        (self.func)(args)
    }

    fn requires_confirmation(&self) -> bool {
        self.requires_confirmation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestTool;

    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn parameters_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            })
        }

        fn execute(&self, args: Value) -> Result<Value, String> {
            let input = args["input"].as_str().unwrap_or("default");
            Ok(json!({ "output": format!("processed: {}", input) }))
        }
    }

    #[test]
    fn test_tool_definition() {
        let tool = TestTool;
        let def = tool.to_definition();

        assert_eq!(def.tool_type, "function");
        assert_eq!(def.function.name, "test_tool");
        assert_eq!(def.function.description, Some("A test tool".to_string()));
    }

    #[test]
    fn test_tool_execute() {
        let tool = TestTool;
        let result = tool.execute(json!({"input": "hello"}));

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["output"], "processed: hello");
    }

    #[test]
    fn test_fn_tool() {
        let tool = FnTool::new(
            "add",
            "Add two numbers",
            json!({
                "type": "object",
                "properties": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                },
                "required": ["a", "b"]
            }),
            |args| {
                let a = args["a"].as_f64().unwrap_or(0.0);
                let b = args["b"].as_f64().unwrap_or(0.0);
                Ok(json!({ "result": a + b }))
            },
        );

        assert_eq!(tool.name(), "add");
        let result = tool.execute(json!({"a": 1, "b": 2}));
        assert_eq!(result.unwrap()["result"], 3.0);
    }

    #[test]
    fn test_tool_call_parse() {
        let call = ToolCall {
            index: None,
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "test".to_string(),
                arguments: r#"{"foo": "bar"}"#.to_string(),
            },
        };

        let args = call.parse_arguments().unwrap();
        assert_eq!(args["foo"], "bar");
    }
}
