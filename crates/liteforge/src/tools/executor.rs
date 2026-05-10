//! Tool executor with validation and error handling.

use super::registry::ToolRegistry;
use super::schema::validate_json_schema;
use super::{Tool, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool call ID (for correlating with the original call).
    pub tool_call_id: String,
    /// Name of the tool that was executed.
    pub name: String,
    /// Whether the execution was successful.
    pub success: bool,
    /// The result value (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        result: Value,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            name: name.into(),
            success: true,
            result: Some(result),
            error: None,
            execution_time_ms: None,
        }
    }

    /// Create a failed result.
    pub fn error(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            name: name.into(),
            success: false,
            result: None,
            error: Some(error.into()),
            execution_time_ms: None,
        }
    }

    /// Set the execution time.
    pub fn with_execution_time(mut self, duration: Duration) -> Self {
        self.execution_time_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Convert to a message format for including in conversation.
    pub fn to_message_content(&self) -> String {
        if self.success {
            if let Some(result) = &self.result {
                serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            }
        } else {
            format!(
                r#"{{"error": "{}"}}"#,
                self.error.as_deref().unwrap_or("Unknown error")
            )
        }
    }
}

/// Executor for running tools with validation.
pub struct ToolExecutor {
    registry: ToolRegistry,
    validate_args: bool,
    timeout: Option<Duration>,
}

impl ToolExecutor {
    /// Create a new executor with the given registry.
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            validate_args: true,
            timeout: None,
        }
    }

    /// Set whether to validate arguments against the tool's schema.
    pub fn validate_args(mut self, validate: bool) -> Self {
        self.validate_args = validate;
        self
    }

    /// Set a timeout for tool execution.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Get the underlying registry.
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Get a mutable reference to the registry.
    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    /// Execute a tool by name with the given arguments.
    pub fn execute(&self, name: &str, args: Value) -> ToolResult {
        self.execute_with_id("", name, args)
    }

    /// Execute a tool by name with a call ID and arguments.
    pub fn execute_with_id(&self, call_id: &str, name: &str, args: Value) -> ToolResult {
        let start = Instant::now();

        // Look up the tool
        let tool = match self.registry.get(name) {
            Some(t) => t,
            None => {
                return ToolResult::error(call_id, name, format!("Tool '{}' not found", name));
            }
        };

        // Validate arguments if enabled
        if self.validate_args {
            let schema = tool.parameters_schema();
            if let Err(errors) = validate_json_schema(&args, &schema) {
                let error_msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                return ToolResult::error(
                    call_id,
                    name,
                    format!("Invalid arguments: {}", error_msg),
                );
            }
        }

        // Execute the tool
        match tool.execute(args) {
            Ok(result) => {
                ToolResult::success(call_id, name, result).with_execution_time(start.elapsed())
            }
            Err(e) => ToolResult::error(call_id, name, e).with_execution_time(start.elapsed()),
        }
    }

    /// Execute a tool call from an LLM response.
    pub fn execute_call(&self, call: &ToolCall) -> ToolResult {
        let args = match call.parse_arguments() {
            Ok(args) => args,
            Err(e) => {
                return ToolResult::error(
                    &call.id,
                    &call.function.name,
                    format!("Failed to parse arguments: {}", e),
                );
            }
        };

        self.execute_with_id(&call.id, &call.function.name, args)
    }

    /// Execute multiple tool calls.
    pub fn execute_calls(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        calls.iter().map(|call| self.execute_call(call)).collect()
    }

    /// Check if a tool exists.
    pub fn has_tool(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    /// Get a tool by name.
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.registry.get(name)
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new(ToolRegistry::new())
    }
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolExecutor")
            .field("registry", &self.registry)
            .field("validate_args", &self.validate_args)
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct AddTool;

    impl Tool for AddTool {
        fn name(&self) -> &str {
            "add"
        }

        fn description(&self) -> &str {
            "Add two numbers"
        }

        fn parameters_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["a", "b"]
            })
        }

        fn execute(&self, args: Value) -> Result<Value, String> {
            let a = args["a"].as_f64().ok_or("Missing 'a'")?;
            let b = args["b"].as_f64().ok_or("Missing 'b'")?;
            Ok(json!({"result": a + b}))
        }
    }

    #[test]
    fn test_executor_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(AddTool));

        let executor = ToolExecutor::new(registry);
        let result = executor.execute("add", json!({"a": 1, "b": 2}));

        assert!(result.success);
        assert_eq!(result.result.unwrap()["result"], 3.0);
    }

    #[test]
    fn test_executor_not_found() {
        let executor = ToolExecutor::new(ToolRegistry::new());
        let result = executor.execute("nonexistent", json!({}));

        assert!(!result.success);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[test]
    fn test_executor_validation_error() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(AddTool));

        let executor = ToolExecutor::new(registry);
        let result = executor.execute("add", json!({"a": 1})); // missing 'b'

        assert!(!result.success);
        assert!(result.error.unwrap().contains("required"));
    }

    #[test]
    fn test_executor_skip_validation() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(AddTool));

        let executor = ToolExecutor::new(registry).validate_args(false);
        // This would fail validation but we skip it
        let result = executor.execute("add", json!({"a": 1, "b": 2}));

        assert!(result.success);
    }

    #[test]
    fn test_executor_execute_call() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(AddTool));

        let executor = ToolExecutor::new(registry);
        let call = ToolCall {
            index: None,
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: super::super::FunctionCall {
                name: "add".to_string(),
                arguments: r#"{"a": 5, "b": 3}"#.to_string(),
            },
        };

        let result = executor.execute_call(&call);

        assert!(result.success);
        assert_eq!(result.tool_call_id, "call_123");
        assert_eq!(result.result.unwrap()["result"], 8.0);
    }

    #[test]
    fn test_tool_result_to_message() {
        let success = ToolResult::success("id", "tool", json!({"value": 42}));
        assert_eq!(success.to_message_content(), r#"{"value":42}"#);

        let error = ToolResult::error("id", "tool", "Something went wrong");
        assert!(error.to_message_content().contains("Something went wrong"));
    }
}
