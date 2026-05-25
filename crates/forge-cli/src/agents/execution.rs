//! Shared agent execution logic: tool definitions, tool execution, and tool-call loop.

use super::config::{ToolConfig, ToolType};
use liteforge::mcp::{CallToolParams, McpServerManager, McpTool, ToolResultContent};
use liteforge::{ToolCall, ToolDefinition, ToolParameters};

/// Build tool definitions from agent config and connected MCP servers.
pub async fn build_tool_definitions(
    agent_tools: &[ToolConfig],
    manager: &McpServerManager,
) -> Vec<ToolDefinition> {
    let mut definitions = Vec::new();

    for tool in agent_tools {
        if tool.tool_type != ToolType::Mcp {
            let def = agent_tool_to_definition(tool);
            definitions.push(def);
        }
    }

    for (server_name, tools) in manager.list_all_tools().await {
        for mcp_tool in tools {
            let def = mcp_tool_to_definition(&mcp_tool, &server_name);
            definitions.push(def);
        }
    }

    definitions
}

/// Convert an agent-configured tool to a ToolDefinition for the LLM.
pub fn agent_tool_to_definition(tool: &ToolConfig) -> ToolDefinition {
    let parameters = if let Some(schema) = &tool.parameters {
        if let Ok(json) = serde_json::to_value(schema) {
            if let Some(obj) = json.as_object() {
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

                ToolParameters {
                    schema_type: "object".to_string(),
                    properties,
                    required,
                }
            } else {
                ToolParameters::default()
            }
        } else {
            ToolParameters::default()
        }
    } else {
        ToolParameters::default()
    };

    ToolDefinition::new(&tool.name)
        .description(&tool.description)
        .parameters(parameters)
}

/// Convert an MCP tool to a ToolDefinition for the LLM.
pub fn mcp_tool_to_definition(mcp_tool: &McpTool, _server_name: &str) -> ToolDefinition {
    let parameters = if let Some(schema) = mcp_tool.input_schema.as_object() {
        let properties = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();

        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        ToolParameters {
            schema_type: "object".to_string(),
            properties,
            required,
        }
    } else {
        ToolParameters::default()
    };

    ToolDefinition::new(&mcp_tool.name)
        .description(mcp_tool.description.clone().unwrap_or_default())
        .parameters(parameters)
}

/// Execute a tool call and return the result.
/// Checks agent-configured tools first, then MCP servers.
pub async fn execute_tool(
    agent_tools: &[ToolConfig],
    manager: &McpServerManager,
    call: &ToolCall,
) -> Result<serde_json::Value, String> {
    if let Some(tool) = agent_tools.iter().find(|t| t.name == call.function.name) {
        return execute_agent_tool(tool, call).await;
    }

    for (server_name, tools) in manager.list_all_tools().await {
        if tools.iter().any(|t| t.name == call.function.name) {
            if let Some(server) = manager.get(&server_name) {
                let arguments: Option<std::collections::HashMap<String, serde_json::Value>> =
                    call.function.parse_arguments().ok().and_then(|v| {
                        if v.is_object() {
                            serde_json::from_value(v).ok()
                        } else {
                            None
                        }
                    });

                let params = CallToolParams {
                    name: call.function.name.clone(),
                    arguments,
                };

                match server.call_tool(params).await {
                    Ok(result) => {
                        if result.is_error.unwrap_or(false) {
                            let error_text = result
                                .content
                                .iter()
                                .filter_map(|c| match c {
                                    ToolResultContent::Text { text } => Some(text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            return Err(error_text);
                        }

                        if result.content.len() == 1 {
                            match &result.content[0] {
                                ToolResultContent::Text { text } => {
                                    return Ok(serde_json::Value::String(text.clone()));
                                }
                                ToolResultContent::Image { data, mime_type } => {
                                    return Ok(serde_json::json!({
                                        "type": "image",
                                        "data": data,
                                        "mime_type": mime_type
                                    }));
                                }
                                ToolResultContent::Resource { resource, text } => {
                                    return Ok(serde_json::json!({
                                        "type": "resource",
                                        "uri": resource.uri,
                                        "text": text
                                    }));
                                }
                            }
                        } else {
                            let items: Vec<serde_json::Value> = result
                                .content
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
                            return Ok(serde_json::Value::Array(items));
                        }
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
    }

    Err(format!("Tool not found: {}", call.function.name))
}

/// Execute an agent-configured tool (builtin or function type).
async fn execute_agent_tool(
    tool: &ToolConfig,
    call: &ToolCall,
) -> Result<serde_json::Value, String> {
    match tool.tool_type {
        ToolType::Builtin => crate::builtin_tools::execute(tool, call).await,
        ToolType::Function => Err(format!(
            "The '{}' tool is declared as a function type which requires custom implementation. \
                 Function tools are not yet supported in the agent runtime.",
            tool.name
        )),
        ToolType::Mcp => Err(format!(
            "The '{}' tool is declared as MCP type but no MCP server provides it.",
            tool.name
        )),
    }
}

/// Try to recover a tool call that a model emitted as JSON-in-content
/// instead of structured `tool_calls`. Common with Ollama `:cloud` /
/// `-cloud` reasoning models. Recognised shapes (with or without
/// ```json fences):
///   {"name": "<tool>", "arguments": { ... }}
///   {"name": "<tool>", "parameters": { ... }}
/// Returns None if the content does not look like a tool call.
pub fn parse_text_leaked_tool_call(content: &str) -> Option<ToolCall> {
    let mut text = content.trim();
    if let Some(stripped) = text.strip_prefix("```json") {
        text = stripped.trim();
    } else if let Some(stripped) = text.strip_prefix("```") {
        text = stripped.trim();
    }
    if let Some(stripped) = text.strip_suffix("```") {
        text = stripped.trim();
    }

    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object()?;
    let name = obj.get("name").and_then(|n| n.as_str())?;
    let args = obj
        .get("arguments")
        .or_else(|| obj.get("parameters"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let args_json = serde_json::to_string(&args).ok()?;

    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Some(ToolCall::new(
        format!("call_leaked_{:x}", nanos),
        name,
        args_json,
    ))
}
