//! Builtin tools for agent runtime.

pub mod web_search;

use crate::agents::ToolConfig;
use liteforge::ToolCall;

/// Execute a builtin tool by name.
pub async fn execute(tool: &ToolConfig, call: &ToolCall) -> Result<serde_json::Value, String> {
    match tool.name.as_str() {
        "web_search" => web_search::execute(call).await,
        _ => Err(format!("Unknown builtin tool: {}", tool.name)),
    }
}
