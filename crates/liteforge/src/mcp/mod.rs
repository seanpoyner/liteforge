//! MCP (Model Context Protocol) support.
//!
//! This module provides integration with the Model Context Protocol (MCP),
//! an open protocol for connecting AI assistants to external tools and context.
//!
//! # Overview
//!
//! MCP allows AI assistants to:
//! - Call tools provided by MCP servers
//! - Access resources (files, databases, etc.)
//! - Use pre-defined prompts
//!
//! # Example
//!
//! ```no_run
//! use liteforge::mcp::{McpConfig, McpServerConfig};
//!
//! // Configure an MCP server
//! let config = McpConfig::new()
//!     .with_server(
//!         McpServerConfig::stdio("filesystem", "npx")
//!             .with_arg("-y")
//!             .with_arg("@modelcontextprotocol/server-filesystem")
//!             .with_arg("/tmp")
//!     );
//!
//! // Get server names
//! for name in config.server_names() {
//!     println!("Server: {}", name);
//! }
//! ```
//!
//! # Transport Types
//!
//! MCP supports three transport types:
//!
//! - **Stdio**: Communication via stdin/stdout with a subprocess
//! - **SSE**: Server-Sent Events over HTTP
//! - **HTTP**: Standard HTTP/REST endpoints
//!
//! # Authentication
//!
//! Several authentication methods are supported:
//!
//! - Bearer token
//! - OAuth2
//! - API key

mod config;
mod http;
mod server;
mod sse;
mod stdio;
mod tool;
mod types;

pub use config::{AuthConfig, McpConfig, McpServerConfig, RequestedCapabilities, TransportType};
pub use http::McpHttpServer;
pub use server::{McpError, McpResult, McpServer, McpServerManager, ServerInfo, ServerState};
pub use sse::McpSseServer;
pub use stdio::McpStdioServer;
pub use tool::{McpToolRegistry, McpToolWrapper, ToolDiscoveryResult};
pub use types::{
    // Message types
    CallToolParams,
    CallToolResult,
    // Capabilities
    ClientCapabilities,
    ClientInfo,
    GetPromptResult,
    InitializeParams,
    InitializeResult,
    // JSON-RPC types
    JsonRpcError,
    JsonRpcNotification,
    JsonRpcRequest,
    JsonRpcResponse,
    ListPromptsResult,
    ListResourcesResult,
    ListToolsResult,
    LoggingCapabilities,
    // MCP primitives
    McpPrompt,
    McpPromptArgument,
    McpResource,
    McpResourceTemplate,
    McpTool,
    PromptCapabilities,
    PromptContent,
    PromptMessage,
    ReadResourceResult,
    RequestId,
    ResourceCapabilities,
    ResourceContent,
    RootCapabilities,
    SamplingCapabilities,
    ServerCapabilities,
    ServerInfo as McpServerInfo,
    ToolCapabilities,
    ToolResultContent,
    // Constants
    JSONRPC_VERSION,
    MCP_VERSION,
};
