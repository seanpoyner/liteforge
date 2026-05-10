/**
 * MCP Server Configuration Example
 *
 * Demonstrates configuring MCP (Model Context Protocol) servers
 * with different transport types and authentication methods.
 *
 * Run: node examples/javascript/mcp_server.mjs
 */

import {
  McpServerConfig,
  McpConfig,
} from '@forge/sdk';

// --- Stdio Server ---
console.log('=== MCP Server Configuration ===\n');

const filesystemServer = McpServerConfig.stdio('filesystem', 'npx');
filesystemServer.withArg('-y');
filesystemServer.withArg('@modelcontextprotocol/server-filesystem');
filesystemServer.withArg('/tmp');
filesystemServer.withTimeout(30);

console.log(`Server: ${filesystemServer.name}`);
console.log(`Transport: ${filesystemServer.transport}`);

// --- SSE Server ---
const sseServer = McpServerConfig.sse('remote-tools', 'https://mcp.example.com/sse');
sseServer.withBearerToken('my-token');
sseServer.withTimeout(60);

console.log(`\nSSE Server: ${sseServer.name}`);
console.log(`Transport: ${sseServer.transport}`);

// --- HTTP Server ---
const httpServer = McpServerConfig.http('api-tools', 'https://api.example.com/mcp');
httpServer.withApiKey('my-api-key');
httpServer.withEnv('API_VERSION', 'v2');

console.log(`\nHTTP Server: ${httpServer.name}`);
console.log(`Transport: ${httpServer.transport}`);

// --- MCP Config ---
console.log('\n=== MCP Configuration ===\n');

const config = new McpConfig();
config.withServer(filesystemServer);
config.withServer(sseServer);
config.withServer(httpServer);

console.log(`Configured servers: ${config.serverNames().join(', ')}`);
console.log(`Total servers: ${config.len()}`);

const serverInfo = config.getServer('filesystem');
if (serverInfo) {
  console.log(`\nFilesystem server info:`, JSON.stringify(serverInfo, null, 2));
}
