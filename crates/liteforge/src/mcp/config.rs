//! MCP configuration types.
//!
//! This module provides configuration structures for MCP servers
//! and clients.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Transport type for MCP server communication.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    /// Standard I/O (subprocess).
    #[default]
    Stdio,
    /// Server-Sent Events.
    Sse,
    /// HTTP/REST.
    Http,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::Sse => write!(f, "sse"),
            Self::Http => write!(f, "http"),
        }
    }
}

/// Authentication configuration for MCP servers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    /// No authentication.
    #[default]
    None,
    /// Bearer token authentication.
    Bearer {
        /// The bearer token.
        token: String,
    },
    /// OAuth2 authentication.
    OAuth {
        /// Client ID.
        client_id: String,
        /// Client secret.
        #[serde(skip_serializing_if = "Option::is_none")]
        client_secret: Option<String>,
        /// Token endpoint URL.
        token_url: String,
        /// OAuth scopes.
        #[serde(skip_serializing_if = "Option::is_none")]
        scopes: Option<Vec<String>>,
    },
    /// API key authentication.
    ApiKey {
        /// Header name for the API key.
        header: String,
        /// The API key value.
        key: String,
    },
}

/// Configuration for an individual MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (unique identifier).
    pub name: String,

    /// Transport type.
    #[serde(default)]
    pub transport: TransportType,

    /// Command to run for stdio transport.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Arguments for the command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Environment variables to set.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Working directory for stdio transport.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,

    /// URL for SSE/HTTP transport.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Authentication configuration.
    #[serde(default, skip_serializing_if = "is_auth_none")]
    pub auth: AuthConfig,

    /// Connection timeout.
    #[serde(
        default = "default_timeout",
        with = "humantime_serde",
        skip_serializing_if = "is_default_timeout"
    )]
    pub timeout: Duration,

    /// Whether to auto-reconnect on disconnect.
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    /// Maximum reconnection attempts.
    #[serde(default = "default_max_reconnects")]
    pub max_reconnects: u32,

    /// Capabilities to request from the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<RequestedCapabilities>,

    /// Additional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_true() -> bool {
    true
}

fn default_max_reconnects() -> u32 {
    3
}

fn is_default_timeout(d: &Duration) -> bool {
    *d == default_timeout()
}

fn is_auth_none(auth: &AuthConfig) -> bool {
    matches!(auth, AuthConfig::None)
}

impl McpServerConfig {
    /// Create a new stdio server configuration.
    pub fn stdio(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Stdio,
            command: Some(command.into()),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            url: None,
            auth: AuthConfig::None,
            timeout: default_timeout(),
            auto_reconnect: true,
            max_reconnects: default_max_reconnects(),
            capabilities: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new SSE server configuration.
    pub fn sse(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Sse,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            url: Some(url.into()),
            auth: AuthConfig::None,
            timeout: default_timeout(),
            auto_reconnect: true,
            max_reconnects: default_max_reconnects(),
            capabilities: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new HTTP server configuration.
    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Http,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            url: Some(url.into()),
            auth: AuthConfig::None,
            timeout: default_timeout(),
            auto_reconnect: true,
            max_reconnects: default_max_reconnects(),
            capabilities: None,
            metadata: HashMap::new(),
        }
    }

    /// Add command arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Add a single argument.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set environment variables.
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Add an environment variable.
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set authentication.
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }

    /// Set bearer token authentication.
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = AuthConfig::Bearer {
            token: token.into(),
        };
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set auto-reconnect behavior.
    pub fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.auto_reconnect = auto_reconnect;
        self
    }

    /// Set max reconnection attempts.
    pub fn with_max_reconnects(mut self, max_reconnects: u32) -> Self {
        self.max_reconnects = max_reconnects;
        self
    }
}

/// Capabilities to request from an MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestedCapabilities {
    /// Request tool capabilities.
    #[serde(default)]
    pub tools: bool,
    /// Request resource capabilities.
    #[serde(default)]
    pub resources: bool,
    /// Request prompt capabilities.
    #[serde(default)]
    pub prompts: bool,
}

/// Overall MCP configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// Configured MCP servers.
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,

    /// Default timeout for all servers.
    #[serde(
        default = "default_timeout",
        with = "humantime_serde",
        skip_serializing_if = "is_default_timeout"
    )]
    pub default_timeout: Duration,

    /// Whether to enable all servers by default.
    #[serde(default = "default_true")]
    pub enable_all: bool,
}

impl McpConfig {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a server configuration.
    pub fn add_server(&mut self, config: McpServerConfig) -> &mut Self {
        self.servers.push(config);
        self
    }

    /// Add a server configuration (builder pattern).
    pub fn with_server(mut self, config: McpServerConfig) -> Self {
        self.servers.push(config);
        self
    }

    /// Get a server configuration by name.
    pub fn get_server(&self, name: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// Get all server names.
    pub fn server_names(&self) -> Vec<&str> {
        self.servers.iter().map(|s| s.name.as_str()).collect()
    }

    /// Set default timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_display() {
        assert_eq!(TransportType::Stdio.to_string(), "stdio");
        assert_eq!(TransportType::Sse.to_string(), "sse");
        assert_eq!(TransportType::Http.to_string(), "http");
    }

    #[test]
    fn test_server_config_stdio() {
        let config = McpServerConfig::stdio("filesystem", "npx")
            .with_arg("-y")
            .with_arg("@modelcontextprotocol/server-filesystem")
            .with_arg("/tmp");

        assert_eq!(config.name, "filesystem");
        assert_eq!(config.transport, TransportType::Stdio);
        assert_eq!(config.command, Some("npx".to_string()));
        assert_eq!(config.args.len(), 3);
    }

    #[test]
    fn test_server_config_sse() {
        let config = McpServerConfig::sse("remote", "https://example.com/mcp")
            .with_bearer_token("secret123");

        assert_eq!(config.transport, TransportType::Sse);
        assert_eq!(config.url, Some("https://example.com/mcp".to_string()));
        assert!(matches!(config.auth, AuthConfig::Bearer { .. }));
    }

    #[test]
    fn test_server_config_http() {
        let config = McpServerConfig::http("api", "https://api.example.com/v1")
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.transport, TransportType::Http);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_mcp_config() {
        let config = McpConfig::new()
            .with_server(McpServerConfig::stdio("fs", "mcp-server-filesystem"))
            .with_server(McpServerConfig::sse("remote", "https://example.com"));

        assert_eq!(config.servers.len(), 2);
        assert!(config.get_server("fs").is_some());
        assert!(config.get_server("remote").is_some());
        assert!(config.get_server("nonexistent").is_none());
    }

    #[test]
    fn test_server_names() {
        let config = McpConfig::new()
            .with_server(McpServerConfig::stdio("a", "cmd"))
            .with_server(McpServerConfig::stdio("b", "cmd"));

        let names = config.server_names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_auth_config_serialization() {
        let bearer = AuthConfig::Bearer {
            token: "secret".to_string(),
        };
        let json = serde_json::to_string(&bearer).unwrap();
        assert!(json.contains("bearer"));
        assert!(json.contains("secret"));

        let api_key = AuthConfig::ApiKey {
            header: "X-API-Key".to_string(),
            key: "mykey".to_string(),
        };
        let json = serde_json::to_string(&api_key).unwrap();
        assert!(json.contains("apikey"));
    }

    #[test]
    fn test_server_config_env() {
        let config = McpServerConfig::stdio("test", "cmd")
            .with_env_var("PATH", "/usr/bin")
            .with_env_var("HOME", "/home/user");

        assert_eq!(config.env.len(), 2);
        assert_eq!(config.env.get("PATH"), Some(&"/usr/bin".to_string()));
    }
}
