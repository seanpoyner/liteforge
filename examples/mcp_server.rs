//! MCP Server example - configuration and tool integration.
//!
//! Run with: cargo run --example mcp_server
//!
//! This example demonstrates:
//! - McpServerConfig for configuring MCP servers
//! - Different transport types (stdio, SSE, HTTP)
//! - McpConfig for managing multiple servers
//! - McpToolWrapper for integrating MCP tools with ToolRegistry
//! - ToolDiscoveryResult for handling discovered tools
//!
//! Note: This example demonstrates MCP configuration patterns without
//! actually connecting to external MCP servers.

use serde_json::json;
use std::time::Duration;
use liteforge::mcp::{
    AuthConfig, McpConfig, McpServerConfig, McpTool, McpToolRegistry, McpToolWrapper,
    ToolDiscoveryResult, TransportType,
};
use liteforge::tools::{Tool, ToolRegistry};

fn main() {
    println!("=== LiteForge MCP Server Example ===\n");

    // 1. Create MCP server configurations
    println!("1. Creating MCP server configurations...\n");

    // Stdio transport (subprocess)
    let filesystem_server = McpServerConfig::stdio("filesystem", "npx")
        .with_arg("-y")
        .with_arg("@modelcontextprotocol/server-filesystem")
        .with_arg("/tmp")
        .with_timeout(Duration::from_secs(30))
        .with_auto_reconnect(true);

    println!("   Filesystem server (stdio):");
    println!("     Name: {}", filesystem_server.name);
    println!("     Transport: {}", filesystem_server.transport);
    println!("     Command: {:?}", filesystem_server.command);
    println!("     Args: {:?}", filesystem_server.args);

    // SSE transport (remote server)
    let remote_server = McpServerConfig::sse("remote-api", "https://api.example.com/mcp/sse")
        .with_bearer_token("your-api-token")
        .with_timeout(Duration::from_secs(60))
        .with_max_reconnects(5);

    println!("\n   Remote API server (SSE):");
    println!("     Name: {}", remote_server.name);
    println!("     Transport: {}", remote_server.transport);
    println!("     URL: {:?}", remote_server.url);
    println!("     Auto-reconnect: {}", remote_server.auto_reconnect);

    // HTTP transport
    let http_server = McpServerConfig::http("rest-api", "https://api.example.com/mcp")
        .with_timeout(Duration::from_secs(30));

    println!("\n   REST API server (HTTP):");
    println!("     Name: {}", http_server.name);
    println!("     Transport: {}", http_server.transport);
    println!("     URL: {:?}", http_server.url);

    // Stdio with environment variables
    let python_server = McpServerConfig::stdio("python-tools", "python")
        .with_arg("-m")
        .with_arg("mcp_server")
        .with_env_var("PYTHONPATH", "/opt/mcp-tools")
        .with_env_var("DEBUG", "1");

    println!("\n   Python tools server (stdio with env):");
    println!("     Name: {}", python_server.name);
    println!("     Args: {:?}", python_server.args);
    println!("     Env vars: {:?}", python_server.env);

    // 2. Build MCP configuration
    println!("\n2. Building MCP configuration...");

    let mut config = McpConfig::new();
    config.add_server(filesystem_server);
    config.add_server(remote_server);
    config.add_server(http_server);
    config.add_server(python_server);

    println!("   Configured servers: {:?}", config.server_names());

    // Get specific server config
    if let Some(fs_config) = config.get_server("filesystem") {
        println!("   Retrieved 'filesystem' config: transport={}", fs_config.transport);
    }

    // 3. Create mock MCP tools
    println!("\n3. Creating MCP tool definitions...");

    let read_file_tool = McpTool {
        name: "read_file".to_string(),
        description: Some("Read the contents of a file".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        }),
    };

    let write_file_tool = McpTool {
        name: "write_file".to_string(),
        description: Some("Write content to a file".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        }),
    };

    let list_dir_tool = McpTool {
        name: "list_directory".to_string(),
        description: Some("List contents of a directory".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path"
                }
            },
            "required": ["path"]
        }),
    };

    for tool in [&read_file_tool, &write_file_tool, &list_dir_tool] {
        println!("   Tool: {} - {}", tool.name, tool.description.as_deref().unwrap_or(""));
    }

    // 4. Create McpToolWrapper instances
    println!("\n4. Wrapping MCP tools for ToolRegistry...");

    // Create wrapper with mock executor
    let wrapper = McpToolWrapper::new(read_file_tool.clone(), "filesystem")
        .with_executor(|params| {
            // Mock executor for demonstration
            Ok(liteforge::mcp::CallToolResult {
                content: vec![liteforge::mcp::ToolResultContent::Text {
                    text: format!("Mock file contents for: {}", params.name),
                }],
                is_error: None,
            })
        });

    println!("   Created wrapper for: {}", wrapper.name());
    println!("   Server: {}", wrapper.server_name());
    println!("   Description: {}", wrapper.description());

    // Test the wrapper (implements Tool trait)
    let result = wrapper.execute(json!({"path": "/tmp/test.txt"}));
    println!("   Mock execution result: {:?}", result);

    // 5. Tool discovery simulation
    println!("\n5. Simulating tool discovery...");

    // Successful discovery
    let discovery_result = ToolDiscoveryResult::success(
        "filesystem",
        vec![read_file_tool, write_file_tool, list_dir_tool],
    );

    println!("   Discovery from '{}' server:", discovery_result.server_name);
    println!("     Success: {}", discovery_result.is_success());
    println!("     Tools found: {}", discovery_result.tool_count());
    for tool in &discovery_result.tools {
        println!("       - {}", tool.name);
    }

    // Failed discovery
    let failed_discovery = ToolDiscoveryResult::failure(
        "unavailable-server",
        "Connection timeout after 30 seconds",
    );
    println!("\n   Discovery from '{}' server:", failed_discovery.server_name);
    println!("     Success: {}", failed_discovery.is_success());
    println!("     Errors: {:?}", failed_discovery.errors);

    // 6. Register MCP tools with ToolRegistry
    println!("\n6. Registering MCP tools with ToolRegistry...");

    let mut registry = ToolRegistry::new();
    let count = registry.register_mcp_tools(&discovery_result);
    println!("   Registered {} tools from MCP server", count);
    println!("   Registry contains: {:?}", registry.names());

    // Get tool definitions for LLM
    let definitions = registry.definitions();
    println!("   {} tool definitions ready for LLM API", definitions.len());

    // 7. Authentication configurations
    println!("\n7. Authentication options...");

    let bearer_auth = AuthConfig::Bearer {
        token: "secret-token-123".to_string(),
    };
    println!("   Bearer auth: {:?}", bearer_auth);

    let api_key_auth = AuthConfig::ApiKey {
        header: "X-API-Key".to_string(),
        key: "my-api-key".to_string(),
    };
    println!("   API key auth: {:?}", api_key_auth);

    let oauth_auth = AuthConfig::OAuth {
        client_id: "my-client-id".to_string(),
        client_secret: Some("my-secret".to_string()),
        token_url: "https://auth.example.com/token".to_string(),
        scopes: Some(vec!["read".to_string(), "write".to_string()]),
    };
    println!("   OAuth2 auth: client_id={}", match &oauth_auth {
        AuthConfig::OAuth { client_id, .. } => client_id,
        _ => "",
    });

    // 8. Transport types
    println!("\n8. Available transport types...");
    println!("   {} - Subprocess communication via stdin/stdout", TransportType::Stdio);
    println!("   {} - Server-Sent Events for real-time updates", TransportType::Sse);
    println!("   {} - Standard HTTP/REST endpoints", TransportType::Http);

    println!("\n=== Example Complete ===");
    println!("\nNote: To actually use MCP servers, you would need:");
    println!("  - An MCP server process (for stdio) or");
    println!("  - An MCP server endpoint (for SSE/HTTP)");
    println!("  - The mcp_server.run() or similar to start communication");
}
