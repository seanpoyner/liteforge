# MCP API

Model Context Protocol client types and server management.

## McpConfig

```rust
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
}
```

## McpServerConfig

```rust
pub struct McpServerConfig {
    pub name: String,
    pub transport: TransportType,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub auth: Option<AuthConfig>,
    pub capabilities: Option<RequestedCapabilities>,
}
```

## TransportType

| Variant | Description |
|---------|-------------|
| `Stdio` | Local process via stdin/stdout |
| `Sse` | Remote via Server-Sent Events |
| `Http` | Remote via HTTP |

## Server Management

### McpServerManager

| Method | Description |
|--------|-------------|
| `new()` | Create manager |
| `add_server(config)` | Connect to a server |
| `list_servers()` | List connected servers |
| `get_server_info(name)` | Get server details |

### McpServer Trait

```rust
pub trait McpServer: Send + Sync {
    async fn initialize(&mut self) -> McpResult<InitializeResult>;
    async fn list_tools(&self) -> McpResult<ListToolsResult>;
    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult>;
    async fn list_resources(&self) -> McpResult<ListResourcesResult>;
    async fn read_resource(&self, uri: &str) -> McpResult<ReadResourceResult>;
    async fn list_prompts(&self) -> McpResult<ListPromptsResult>;
    async fn get_prompt(&self, name: &str, args: Value) -> McpResult<GetPromptResult>;
}
```

Implementations: `McpStdioServer`, `McpSseServer`, `McpHttpServer`.

## Tool Discovery

### McpToolRegistry

| Method | Description |
|--------|-------------|
| `new(manager)` | Create from server manager |
| `discover_tools()` | List all tools from all servers |

### McpToolWrapper

Wraps an MCP tool as a `Tool` trait object for use with `ToolRegistry`.

## JSON-RPC Types

| Type | Description |
|------|-------------|
| `JsonRpcRequest` | JSON-RPC 2.0 request |
| `JsonRpcResponse` | JSON-RPC 2.0 response |
| `JsonRpcError` | Error with code and message |
| `JsonRpcNotification` | One-way notification |

## MCP Primitives

| Type | Description |
|------|-------------|
| `McpTool` | Tool definition from server |
| `McpResource` | Resource definition |
| `McpResourceTemplate` | URI template for resources |
| `McpPrompt` | Prompt template |
| `McpPromptArgument` | Prompt argument definition |
| `ToolResultContent` | Tool call result content |

## Constants

- `JSONRPC_VERSION` = `"2.0"`
- `MCP_VERSION` = `"2024-11-05"`
