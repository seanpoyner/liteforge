# MCP Integration

LiteForge includes a Model Context Protocol (MCP) client for connecting to MCP servers via stdio, SSE, or HTTP.

## Configuration

```rust
use liteforge::mcp::{McpConfig, McpServerConfig, TransportType};

let config = McpConfig {
    servers: vec![
        McpServerConfig {
            name: "filesystem".to_string(),
            transport: TransportType::Stdio,
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/home/user/documents".to_string(),
            ]),
            url: None,
            auth: None,
            capabilities: None,
        },
        McpServerConfig {
            name: "remote-tools".to_string(),
            transport: TransportType::Sse,
            command: None,
            args: None,
            url: Some("http://localhost:3001/sse".to_string()),
            auth: None,
            capabilities: None,
        },
    ],
};
```

## Transport Types

| Transport | Use Case |
|-----------|----------|
| `Stdio` | Local process (spawned via command) |
| `Sse` | Remote server via Server-Sent Events |
| `Http` | Remote server via HTTP |

## Server Management

```rust
use liteforge::mcp::McpServerManager;

let mut manager = McpServerManager::new();

// Add servers from config
for server_config in config.servers {
    manager.add_server(server_config).await?;
}

// List connected servers
let servers = manager.list_servers();

// Get server info
let info: ServerInfo = manager.get_server_info("filesystem")?;
```

## Tool Discovery

MCP servers expose tools that can be integrated with the SDK's tool framework:

```rust
use liteforge::mcp::McpToolRegistry;

let mcp_tools = McpToolRegistry::new(&manager);

// Discover tools from all connected servers
let discovery = mcp_tools.discover_tools().await?;

for tool in &discovery.tools {
    println!("{}: {}", tool.name, tool.description);
}
```

## JSON-RPC Types

The MCP module exposes the full JSON-RPC 2.0 type set:

- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`
- `InitializeParams`, `InitializeResult`
- `ListToolsResult`, `CallToolParams`, `CallToolResult`
- `ListResourcesResult`, `ReadResourceResult`
- `ListPromptsResult`, `GetPromptResult`
- `McpTool`, `McpResource`, `McpResourceTemplate`, `McpPrompt`

## Python Usage

```python
from liteforge import McpConfig, McpServerConfig

config = McpConfig(servers=[
    McpServerConfig(
        name="filesystem",
        transport="stdio",
        command="npx",
        args=["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    )
])
```

## JavaScript / TypeScript Usage

The JS bindings provide builder-style MCP configuration:

```javascript
import { McpServerConfig, McpConfig } from '@seanpoyner/liteforge';

// Stdio server
const fsServer = McpServerConfig.stdio('filesystem', 'npx');
fsServer.withArg('-y');
fsServer.withArg('@modelcontextprotocol/server-filesystem');
fsServer.withArg('/tmp');
fsServer.withTimeout(30);

// SSE server with auth
const sseServer = McpServerConfig.sse('remote-tools', 'https://mcp.example.com/sse');
sseServer.withBearerToken('my-token');

// HTTP server
const httpServer = McpServerConfig.http('api-tools', 'https://api.example.com/mcp');
httpServer.withApiKey('my-api-key');

// Combine into config
const config = new McpConfig();
config.withServer(fsServer);
config.withServer(sseServer);
config.withServer(httpServer);

console.log(config.serverNames()); // ['filesystem', 'remote-tools', 'api-tools']
```

## CLI Usage

```bash
# List configured MCP servers
forge mcp list

# Add an MCP server
forge mcp add --name my-server --transport stdio --command "npx -y @mcp/server"

# Launch Claude Code with MCP servers
forge claude
```
