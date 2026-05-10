//! Agent configuration parsing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Agent configuration loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name/identifier.
    pub name: String,

    /// Agent description.
    #[serde(default)]
    pub description: String,

    /// Model to use.
    #[serde(default)]
    pub model: Option<String>,

    /// System prompt for the agent.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Temperature setting.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Tools available to the agent.
    #[serde(default)]
    pub tools: Vec<ToolConfig>,

    /// MCP servers to connect to.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,

    /// Custom metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_yaml::Value>,
}

/// Tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool name.
    pub name: String,

    /// Tool description.
    #[serde(default)]
    pub description: String,

    /// Tool type (builtin, mcp, function).
    #[serde(default = "default_tool_type")]
    pub tool_type: ToolType,

    /// Tool parameters schema (JSON Schema).
    #[serde(default)]
    pub parameters: Option<serde_yaml::Value>,
}

fn default_tool_type() -> ToolType {
    ToolType::Builtin
}

/// Tool type.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Builtin,
    Mcp,
    Function,
}

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name.
    pub name: String,

    /// Command to run the server.
    pub command: String,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl AgentConfig {
    /// Load an agent config from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AgentConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| AgentConfigError::Io(e.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parse an agent config from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, AgentConfigError> {
        serde_yaml::from_str(yaml).map_err(|e| AgentConfigError::Parse(e.to_string()))
    }

    /// Get tool count.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get MCP server count.
    pub fn mcp_server_count(&self) -> usize {
        self.mcp_servers.len()
    }
}

/// Error type for agent config operations.
#[derive(Debug)]
pub enum AgentConfigError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for AgentConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for AgentConfigError {}

/// Discover agent configs in a directory.
pub fn discover_agents(dir: impl AsRef<Path>) -> Result<Vec<AgentConfig>, AgentConfigError> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err(AgentConfigError::Io(format!(
            "Not a directory: {}",
            dir.display()
        )));
    }

    let mut agents = Vec::new();

    for entry in std::fs::read_dir(dir).map_err(|e| AgentConfigError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| AgentConfigError::Io(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    match AgentConfig::from_file(&path) {
                        Ok(config) => agents.push(config),
                        Err(e) => {
                            eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    agents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(agents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_config() {
        let yaml = r#"
name: test-agent
description: A test agent
model: gpt-4o
system_prompt: You are a helpful assistant.
temperature: 0.7
tools:
  - name: search
    description: Search the web
    tool_type: builtin
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem"]
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "test-agent");
        assert_eq!(config.model, Some("gpt-4o".to_string()));
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.mcp_servers.len(), 1);
    }

    #[test]
    fn test_minimal_config() {
        let yaml = "name: minimal";
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "minimal");
        assert!(config.description.is_empty());
        assert!(config.tools.is_empty());
    }
}
