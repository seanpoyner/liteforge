#!/usr/bin/env python3
"""
MCP Server example - configuration and tool integration.

Run with: python examples/python/mcp_server.py

This example demonstrates:
- McpServerConfig for configuring MCP servers
- Different transport types (stdio, SSE, HTTP)
- McpConfig for managing multiple servers
- Tool definitions in MCP format

Note: This example demonstrates MCP configuration patterns without
actually connecting to external MCP servers.
"""

from liteforge import (
    McpServerConfig,
    McpConfig,
    ToolRegistry,
    create_tool,
)


def main():
    print("=== LiteForge MCP Server Example (Python) ===\n")

    # 1. Create MCP server configurations
    print("1. Creating MCP server configurations...\n")

    # Stdio transport (subprocess)
    filesystem_server = (
        McpServerConfig.stdio("filesystem", "npx")
        .with_arg("-y")
        .with_arg("@modelcontextprotocol/server-filesystem")
        .with_arg("/tmp")
        .with_timeout_secs(30)
        .with_auto_reconnect(True)
    )

    print("   Filesystem server (stdio):")
    print(f"     Name: {filesystem_server.name}")
    print(f"     Transport: {filesystem_server.transport}")
    print(f"     Command: {filesystem_server.command}")
    print(f"     Args: {filesystem_server.args}")

    # SSE transport (remote server)
    remote_server = (
        McpServerConfig.sse("remote-api", "https://api.example.com/mcp/sse")
        .with_bearer_token("your-api-token")
        .with_timeout_secs(60)
        .with_max_reconnects(5)
    )

    print("\n   Remote API server (SSE):")
    print(f"     Name: {remote_server.name}")
    print(f"     Transport: {remote_server.transport}")
    print(f"     URL: {remote_server.url}")
    print(f"     Auto-reconnect: {remote_server.auto_reconnect}")

    # HTTP transport
    http_server = (
        McpServerConfig.http("rest-api", "https://api.example.com/mcp")
        .with_timeout_secs(30)
    )

    print("\n   REST API server (HTTP):")
    print(f"     Name: {http_server.name}")
    print(f"     Transport: {http_server.transport}")
    print(f"     URL: {http_server.url}")

    # Stdio with environment variables
    python_server = (
        McpServerConfig.stdio("python-tools", "python")
        .with_arg("-m")
        .with_arg("mcp_server")
        .with_env_var("PYTHONPATH", "/opt/mcp-tools")
        .with_env_var("DEBUG", "1")
    )

    print("\n   Python tools server (stdio with env):")
    print(f"     Name: {python_server.name}")
    print(f"     Args: {python_server.args}")

    # 2. Build MCP configuration
    print("\n2. Building MCP configuration...")

    config = (
        McpConfig()
        .with_server(filesystem_server)
        .with_server(remote_server)
        .with_server(http_server)
        .with_server(python_server)
    )

    print(f"   Configured servers: {config.server_names()}")

    # Get specific server config
    fs_config = config.get_server("filesystem")
    if fs_config:
        print(f"   Retrieved 'filesystem' config: transport={fs_config.transport}")

    # 3. Define tools that an MCP server might provide
    print("\n3. Defining MCP-style tools...\n")

    # These are tools that would typically be provided by an MCP server
    # We can register them directly for local use

    def read_file(args: dict) -> dict:
        """Mock file read."""
        path = args.get("path", "")
        return {"content": f"Mock contents of {path}", "size": 42}

    def write_file(args: dict) -> dict:
        """Mock file write."""
        path = args.get("path", "")
        content = args.get("content", "")
        return {"success": True, "path": path, "bytes_written": len(content)}

    def list_directory(args: dict) -> dict:
        """Mock directory listing."""
        path = args.get("path", "/")
        return {
            "path": path,
            "entries": [
                {"name": "file1.txt", "type": "file"},
                {"name": "file2.txt", "type": "file"},
                {"name": "subdir", "type": "directory"},
            ],
        }

    # Create tools with MCP-like schemas
    tools = [
        create_tool(
            name="read_file",
            description="Read the contents of a file",
            parameters={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read",
                    }
                },
                "required": ["path"],
            },
            func=read_file,
            requires_confirmation=False,
        ),
        create_tool(
            name="write_file",
            description="Write content to a file",
            parameters={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write",
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write",
                    },
                },
                "required": ["path", "content"],
            },
            func=write_file,
            requires_confirmation=True,  # Writing requires confirmation
        ),
        create_tool(
            name="list_directory",
            description="List contents of a directory",
            parameters={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path",
                    }
                },
                "required": ["path"],
            },
            func=list_directory,
            requires_confirmation=False,
        ),
    ]

    for tool in tools:
        print(f"   Tool: {tool}")

    # 4. Register tools in registry
    print("\n4. Registering tools in ToolRegistry...")

    registry = ToolRegistry()
    for tool in tools:
        registry.register(tool)

    print(f"   Registered tools: {registry.names()}")
    print(f"   Contains 'read_file': {registry.contains('read_file')}")

    # 5. Execute tools (simulating MCP tool calls)
    print("\n5. Executing tools (simulating MCP calls)...")

    # Read file
    result = read_file({"path": "/etc/hosts"})
    print(f"   read_file('/etc/hosts') -> {result}")

    # Write file
    result = write_file({"path": "/tmp/test.txt", "content": "Hello, MCP!"})
    print(f"   write_file('/tmp/test.txt') -> {result}")

    # List directory
    result = list_directory({"path": "/home"})
    print(f"   list_directory('/home') -> {result}")

    # 6. Typical MCP server workflow
    print("\n6. Typical MCP workflow...")
    print("""
   1. Configure servers:
      config = McpConfig().with_server(McpServerConfig.stdio(...))

   2. Start MCP manager (connects to servers):
      manager = McpServerManager(config)
      await manager.start()

   3. Discover available tools:
      tools = await manager.list_tools("server-name")

   4. Execute tool calls:
      result = await manager.call_tool("server-name", "tool-name", args)

   5. Shut down:
      await manager.stop()
    """)

    # 7. Different server patterns
    print("7. Common MCP server patterns...\n")

    patterns = [
        ("File System", "npx @modelcontextprotocol/server-filesystem /path"),
        ("GitHub", "npx @modelcontextprotocol/server-github"),
        ("Database", "npx @modelcontextprotocol/server-postgres"),
        ("Slack", "npx @modelcontextprotocol/server-slack"),
        ("Custom Python", "python -m my_mcp_server"),
        ("Custom Node", "node my-mcp-server.js"),
    ]

    for name, cmd in patterns:
        print(f"   {name}: {cmd}")

    print("\n=== Example Complete ===")
    print("\nNote: To actually use MCP servers, you would need:")
    print("  - An MCP server process (for stdio) or")
    print("  - An MCP server endpoint (for SSE/HTTP)")
    print("  - The McpServerManager to handle connections")


if __name__ == "__main__":
    main()
