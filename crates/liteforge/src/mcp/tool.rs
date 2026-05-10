//! MCP tool integration.
//!
//! This module provides wrappers for MCP tools that implement the SDK's Tool trait.

use super::types::{CallToolParams, CallToolResult, McpTool, ToolResultContent};
use crate::tools::{Tool, ToolRegistry};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for tool executor callback to reduce complexity.
type ToolExecutorFn = Arc<dyn Fn(CallToolParams) -> Result<CallToolResult, String> + Send + Sync>;

/// A wrapper around an MCP tool that implements the Tool trait.
///
/// This allows MCP tools to be used with the standard ToolRegistry and ToolExecutor.
#[derive(Clone)]
pub struct McpToolWrapper {
    /// The underlying MCP tool definition.
    tool: McpTool,
    /// Server name this tool belongs to.
    server_name: String,
    /// Callback for executing the tool (set by the transport).
    executor: Option<ToolExecutorFn>,
}

impl McpToolWrapper {
    /// Create a new MCP tool wrapper.
    pub fn new(tool: McpTool, server_name: impl Into<String>) -> Self {
        Self {
            tool,
            server_name: server_name.into(),
            executor: None,
        }
    }

    /// Set the execution callback.
    pub fn with_executor<F>(mut self, executor: F) -> Self
    where
        F: Fn(CallToolParams) -> Result<CallToolResult, String> + Send + Sync + 'static,
    {
        self.executor = Some(Arc::new(executor));
        self
    }

    /// Get the server name this tool belongs to.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the underlying MCP tool definition.
    pub fn mcp_tool(&self) -> &McpTool {
        &self.tool
    }

    /// Convert tool result content to a JSON value.
    fn content_to_value(content: &[ToolResultContent]) -> Value {
        if content.len() == 1 {
            match &content[0] {
                ToolResultContent::Text { text } => Value::String(text.clone()),
                ToolResultContent::Image { data, mime_type } => {
                    serde_json::json!({
                        "type": "image",
                        "data": data,
                        "mime_type": mime_type
                    })
                }
                ToolResultContent::Resource { resource, text } => {
                    serde_json::json!({
                        "type": "resource",
                        "uri": resource.uri,
                        "text": text
                    })
                }
            }
        } else {
            let items: Vec<Value> = content
                .iter()
                .map(|c| match c {
                    ToolResultContent::Text { text } => {
                        serde_json::json!({"type": "text", "text": text})
                    }
                    ToolResultContent::Image { data, mime_type } => {
                        serde_json::json!({"type": "image", "data": data, "mime_type": mime_type})
                    }
                    ToolResultContent::Resource { resource, text } => {
                        serde_json::json!({"type": "resource", "uri": resource.uri, "text": text})
                    }
                })
                .collect();
            Value::Array(items)
        }
    }
}

impl std::fmt::Debug for McpToolWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpToolWrapper")
            .field("name", &self.tool.name)
            .field("server_name", &self.server_name)
            .field("has_executor", &self.executor.is_some())
            .finish()
    }
}

impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        self.tool.description.as_deref().unwrap_or("")
    }

    fn parameters_schema(&self) -> Value {
        self.tool.input_schema.clone()
    }

    fn execute(&self, args: Value) -> Result<Value, String> {
        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| "Tool executor not set".to_string())?;

        // Convert args to HashMap
        let arguments: Option<HashMap<String, Value>> = if args.is_object() {
            serde_json::from_value(args).ok()
        } else {
            None
        };

        let params = CallToolParams {
            name: self.tool.name.clone(),
            arguments,
        };

        let result = executor(params)?;

        // Check for error
        if result.is_error.unwrap_or(false) {
            let error_msg = result
                .content
                .iter()
                .filter_map(|c| match c {
                    ToolResultContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(error_msg);
        }

        Ok(Self::content_to_value(&result.content))
    }
}

/// Result of discovering tools from an MCP server.
#[derive(Debug, Clone)]
pub struct ToolDiscoveryResult {
    /// Server name.
    pub server_name: String,
    /// Discovered tools.
    pub tools: Vec<McpTool>,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

impl ToolDiscoveryResult {
    /// Create a successful discovery result.
    pub fn success(server_name: impl Into<String>, tools: Vec<McpTool>) -> Self {
        Self {
            server_name: server_name.into(),
            tools,
            errors: Vec::new(),
        }
    }

    /// Create a failed discovery result.
    pub fn failure(server_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            tools: Vec::new(),
            errors: vec![error.into()],
        }
    }

    /// Check if discovery was successful.
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of discovered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// A registry extension for managing MCP tools.
pub trait McpToolRegistry {
    /// Register tools from an MCP server.
    fn register_mcp_tools(&mut self, result: &ToolDiscoveryResult) -> usize;

    /// Unregister all tools from an MCP server.
    fn unregister_mcp_server(&mut self, server_name: &str) -> usize;

    /// Get all tools from an MCP server.
    fn get_mcp_tools(&self, server_name: &str) -> Vec<Arc<dyn Tool>>;
}

impl McpToolRegistry for ToolRegistry {
    fn register_mcp_tools(&mut self, result: &ToolDiscoveryResult) -> usize {
        let mut count = 0;
        for mcp_tool in &result.tools {
            let wrapper = McpToolWrapper::new(mcp_tool.clone(), &result.server_name);
            self.register(Box::new(wrapper));
            count += 1;
        }
        count
    }

    fn unregister_mcp_server(&mut self, server_name: &str) -> usize {
        let tools_to_remove: Vec<String> = self
            .tools()
            .iter()
            .filter_map(|t| {
                // Check if this is an MCP tool from the specified server
                // by checking the name prefix pattern (server_name::tool_name)
                let name = t.name();
                if name.starts_with(server_name) && name.chars().nth(server_name.len()) == Some(':')
                {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();

        let mut count = 0;
        for name in tools_to_remove {
            if self.unregister(&name).is_some() {
                count += 1;
            }
        }
        count
    }

    fn get_mcp_tools(&self, server_name: &str) -> Vec<Arc<dyn Tool>> {
        self.tools()
            .into_iter()
            .filter(|t| {
                let name = t.name();
                name.starts_with(server_name) && name.chars().nth(server_name.len()) == Some(':')
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_mcp_tool() -> McpTool {
        McpTool {
            name: "read_file".to_string(),
            description: Some("Read contents of a file".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path"}
                },
                "required": ["path"]
            }),
        }
    }

    #[test]
    fn test_mcp_tool_wrapper_creation() {
        let tool = sample_mcp_tool();
        let wrapper = McpToolWrapper::new(tool, "filesystem");

        assert_eq!(wrapper.name(), "read_file");
        assert_eq!(wrapper.description(), "Read contents of a file");
        assert_eq!(wrapper.server_name(), "filesystem");
    }

    #[test]
    fn test_mcp_tool_wrapper_parameters() {
        let tool = sample_mcp_tool();
        let wrapper = McpToolWrapper::new(tool, "filesystem");

        let schema = wrapper.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
    }

    #[test]
    fn test_mcp_tool_wrapper_execute_no_executor() {
        let tool = sample_mcp_tool();
        let wrapper = McpToolWrapper::new(tool, "filesystem");

        let result = wrapper.execute(json!({"path": "/tmp/test.txt"}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("executor not set"));
    }

    #[test]
    fn test_mcp_tool_wrapper_execute_success() {
        let tool = sample_mcp_tool();
        let wrapper = McpToolWrapper::new(tool, "filesystem").with_executor(|_params| {
            Ok(CallToolResult {
                content: vec![ToolResultContent::Text {
                    text: "file contents".to_string(),
                }],
                is_error: None,
            })
        });

        let result = wrapper.execute(json!({"path": "/tmp/test.txt"}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("file contents"));
    }

    #[test]
    fn test_mcp_tool_wrapper_execute_error() {
        let tool = sample_mcp_tool();
        let wrapper = McpToolWrapper::new(tool, "filesystem").with_executor(|_params| {
            Ok(CallToolResult {
                content: vec![ToolResultContent::Text {
                    text: "File not found".to_string(),
                }],
                is_error: Some(true),
            })
        });

        let result = wrapper.execute(json!({"path": "/nonexistent"}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_tool_discovery_result() {
        let result = ToolDiscoveryResult::success("fs", vec![sample_mcp_tool()]);
        assert!(result.is_success());
        assert_eq!(result.tool_count(), 1);

        let result = ToolDiscoveryResult::failure("fs", "Connection failed");
        assert!(!result.is_success());
        assert_eq!(result.tool_count(), 0);
    }

    #[test]
    fn test_content_to_value_single_text() {
        let content = vec![ToolResultContent::Text {
            text: "hello".to_string(),
        }];
        let value = McpToolWrapper::content_to_value(&content);
        assert_eq!(value, json!("hello"));
    }

    #[test]
    fn test_content_to_value_multiple() {
        let content = vec![
            ToolResultContent::Text {
                text: "line1".to_string(),
            },
            ToolResultContent::Text {
                text: "line2".to_string(),
            },
        ];
        let value = McpToolWrapper::content_to_value(&content);
        assert!(value.is_array());
        assert_eq!(value.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_content_to_value_image() {
        let content = vec![ToolResultContent::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        }];
        let value = McpToolWrapper::content_to_value(&content);
        assert_eq!(value["type"], "image");
        assert_eq!(value["mime_type"], "image/png");
    }

    #[test]
    fn test_register_mcp_tools() {
        let mut registry = ToolRegistry::new();
        let result = ToolDiscoveryResult::success(
            "filesystem",
            vec![
                sample_mcp_tool(),
                McpTool {
                    name: "write_file".to_string(),
                    description: Some("Write to a file".to_string()),
                    input_schema: json!({"type": "object"}),
                },
            ],
        );

        let count = registry.register_mcp_tools(&result);
        assert_eq!(count, 2);
        assert!(registry.contains("read_file"));
        assert!(registry.contains("write_file"));
    }
}
