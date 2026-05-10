//! MCP server trait and common functionality.
//!
//! This module defines the common interface for MCP server connections.

use super::config::McpServerConfig;
use super::types::{
    CallToolParams, CallToolResult, GetPromptResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, McpPrompt, McpResource, McpTool, ReadResourceResult, ServerCapabilities,
};
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur when interacting with MCP servers.
#[derive(Debug, Error)]
pub enum McpError {
    /// Server connection failed.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Server disconnected.
    #[error("Server disconnected")]
    Disconnected,

    /// Request timed out.
    #[error("Request timed out")]
    Timeout,

    /// Invalid response from server.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Protocol error.
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Tool execution error.
    #[error("Tool error: {0}")]
    ToolError(String),

    /// Server returned an error.
    #[error("Server error [{code}]: {message}")]
    ServerError { code: i32, message: String },

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type for MCP operations.
pub type McpResult<T> = Result<T, McpError>;

/// State of an MCP server connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    /// Not connected.
    Disconnected,
    /// Connecting to server.
    Connecting,
    /// Connected and initialized.
    Connected,
    /// Connection failed.
    Failed,
    /// Shutting down.
    ShuttingDown,
}

impl std::fmt::Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "disconnected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Failed => write!(f, "failed"),
            Self::ShuttingDown => write!(f, "shutting_down"),
        }
    }
}

/// Information about a connected MCP server.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Server name from config.
    pub name: String,
    /// Server's reported name.
    pub server_name: Option<String>,
    /// Server's reported version.
    pub server_version: Option<String>,
    /// Server capabilities.
    pub capabilities: ServerCapabilities,
    /// Current state.
    pub state: ServerState,
}

/// Trait for MCP server connections.
///
/// This trait defines the common interface for all MCP transport types
/// (stdio, SSE, HTTP).
#[async_trait]
pub trait McpServer: Send + Sync {
    /// Get the server name.
    fn name(&self) -> &str;

    /// Get the server configuration.
    fn config(&self) -> &McpServerConfig;

    /// Get the current connection state.
    fn state(&self) -> ServerState;

    /// Get server info (after connection).
    fn info(&self) -> Option<&ServerInfo>;

    /// Connect to the server.
    async fn connect(&mut self) -> McpResult<()>;

    /// Disconnect from the server.
    async fn disconnect(&mut self) -> McpResult<()>;

    /// Check if connected.
    fn is_connected(&self) -> bool {
        self.state() == ServerState::Connected
    }

    /// List available tools.
    async fn list_tools(&self) -> McpResult<ListToolsResult>;

    /// Call a tool.
    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult>;

    /// List available resources.
    async fn list_resources(&self) -> McpResult<ListResourcesResult>;

    /// Read a resource.
    async fn read_resource(&self, uri: &str) -> McpResult<ReadResourceResult>;

    /// List available prompts.
    async fn list_prompts(&self) -> McpResult<ListPromptsResult>;

    /// Get a prompt.
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> McpResult<GetPromptResult>;
}

/// A manager for multiple MCP server connections.
#[derive(Default)]
pub struct McpServerManager {
    servers: HashMap<String, Box<dyn McpServer>>,
}

impl McpServerManager {
    /// Create a new server manager.
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Add a server to the manager.
    pub fn add_server(&mut self, server: Box<dyn McpServer>) {
        let name = server.name().to_string();
        self.servers.insert(name, server);
    }

    /// Remove a server by name.
    pub fn remove_server(&mut self, name: &str) -> Option<Box<dyn McpServer>> {
        self.servers.remove(name)
    }

    /// Get a server by name.
    pub fn get(&self, name: &str) -> Option<&dyn McpServer> {
        self.servers.get(name).map(|s| s.as_ref())
    }

    /// Get a mutable reference to a server.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn McpServer>> {
        self.servers.get_mut(name)
    }

    /// Get all server names.
    pub fn names(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }

    /// Get all connected servers.
    pub fn connected(&self) -> Vec<&dyn McpServer> {
        self.servers
            .values()
            .filter(|s| s.is_connected())
            .map(|s| s.as_ref())
            .collect()
    }

    /// Connect all servers.
    pub async fn connect_all(&mut self) -> Vec<McpResult<()>> {
        let mut results = Vec::new();
        for server in self.servers.values_mut() {
            results.push(server.connect().await);
        }
        results
    }

    /// Disconnect all servers.
    pub async fn disconnect_all(&mut self) -> Vec<McpResult<()>> {
        let mut results = Vec::new();
        for server in self.servers.values_mut() {
            results.push(server.disconnect().await);
        }
        results
    }

    /// Get all tools from all connected servers.
    pub async fn list_all_tools(&self) -> Vec<(String, Vec<McpTool>)> {
        let mut all_tools = Vec::new();
        for (name, server) in &self.servers {
            if server.is_connected() {
                if let Ok(result) = server.list_tools().await {
                    all_tools.push((name.clone(), result.tools));
                }
            }
        }
        all_tools
    }

    /// Get all resources from all connected servers.
    pub async fn list_all_resources(&self) -> Vec<(String, Vec<McpResource>)> {
        let mut all_resources = Vec::new();
        for (name, server) in &self.servers {
            if server.is_connected() {
                if let Ok(result) = server.list_resources().await {
                    all_resources.push((name.clone(), result.resources));
                }
            }
        }
        all_resources
    }

    /// Get all prompts from all connected servers.
    pub async fn list_all_prompts(&self) -> Vec<(String, Vec<McpPrompt>)> {
        let mut all_prompts = Vec::new();
        for (name, server) in &self.servers {
            if server.is_connected() {
                if let Ok(result) = server.list_prompts().await {
                    all_prompts.push((name.clone(), result.prompts));
                }
            }
        }
        all_prompts
    }
}

impl std::fmt::Debug for McpServerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServerManager")
            .field("servers", &self.names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_state_display() {
        assert_eq!(ServerState::Disconnected.to_string(), "disconnected");
        assert_eq!(ServerState::Connected.to_string(), "connected");
        assert_eq!(ServerState::Failed.to_string(), "failed");
    }

    #[test]
    fn test_mcp_error_display() {
        let err = McpError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));

        let err = McpError::ServerError {
            code: -32601,
            message: "Method not found".to_string(),
        };
        assert!(err.to_string().contains("-32601"));
        assert!(err.to_string().contains("Method not found"));
    }

    #[test]
    fn test_server_manager_new() {
        let manager = McpServerManager::new();
        assert!(manager.names().is_empty());
    }
}
