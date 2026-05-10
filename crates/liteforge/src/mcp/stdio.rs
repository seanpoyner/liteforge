//! MCP stdio transport implementation.
//!
//! This module implements MCP communication over stdin/stdout with a subprocess.

use super::config::McpServerConfig;
use super::server::{McpError, McpResult, McpServer, ServerInfo, ServerState};
use super::types::{
    CallToolParams, CallToolResult, ClientInfo, GetPromptResult, InitializeParams,
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, JSONRPC_VERSION, MCP_VERSION,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

/// A pending request waiting for a response.
struct PendingRequest {
    sender: oneshot::Sender<McpResult<Value>>,
}

/// MCP server connection over stdio.
pub struct McpStdioServer {
    config: McpServerConfig,
    state: ServerState,
    info: Option<ServerInfo>,
    child: Option<Child>,
    request_id: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, PendingRequest>>>,
    write_tx: Option<mpsc::Sender<String>>,
}

impl McpStdioServer {
    /// Create a new stdio server connection.
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: ServerState::Disconnected,
            info: None,
            child: None,
            request_id: AtomicI64::new(1),
            pending: Arc::new(Mutex::new(HashMap::new())),
            write_tx: None,
        }
    }

    /// Get the next request ID.
    fn next_request_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a request and wait for response.
    async fn request(&self, method: &str, params: Option<Value>) -> McpResult<Value> {
        let write_tx = self.write_tx.as_ref().ok_or(McpError::Disconnected)?;

        let id = self.next_request_id();
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;

        // Create channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, PendingRequest { sender: tx });
        }

        // Send request
        write_tx
            .send(request_json)
            .await
            .map_err(|_| McpError::Disconnected)?;

        // Wait for response with timeout
        let timeout = self.config.timeout;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::Disconnected),
            Err(_) => {
                // Remove pending request on timeout
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(McpError::Timeout)
            }
        }
    }

    /// Initialize the connection.
    async fn initialize(&mut self) -> McpResult<InitializeResult> {
        let params = InitializeParams {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: Default::default(),
            client_info: ClientInfo::default(),
        };

        let result: Value = self
            .request("initialize", Some(serde_json::to_value(&params)?))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;

        // Send initialized notification
        let notification = serde_json::json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": "notifications/initialized"
        });

        if let Some(write_tx) = &self.write_tx {
            let _ = write_tx.send(serde_json::to_string(&notification)?).await;
        }

        // Update server info
        self.info = Some(ServerInfo {
            name: self.config.name.clone(),
            server_name: Some(init_result.server_info.name.clone()),
            server_version: init_result.server_info.version.clone(),
            capabilities: init_result.capabilities.clone(),
            state: ServerState::Connected,
        });

        Ok(init_result)
    }
}

#[async_trait]
impl McpServer for McpStdioServer {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &McpServerConfig {
        &self.config
    }

    fn state(&self) -> ServerState {
        self.state
    }

    fn info(&self) -> Option<&ServerInfo> {
        self.info.as_ref()
    }

    async fn connect(&mut self) -> McpResult<()> {
        if self.state == ServerState::Connected {
            return Ok(());
        }

        self.state = ServerState::Connecting;

        // Get command and args
        let command = self
            .config
            .command
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("No command specified".to_string()))?;

        // Build command
        let mut cmd = Command::new(command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &self.config.cwd {
            cmd.current_dir(cwd);
        }

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            self.state = ServerState::Failed;
            McpError::ConnectionFailed(format!("Failed to spawn process: {}", e))
        })?;

        // Get stdin and stdout
        let stdin = child.stdin.take().ok_or_else(|| {
            self.state = ServerState::Failed;
            McpError::ConnectionFailed("Failed to get stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            self.state = ServerState::Failed;
            McpError::ConnectionFailed("Failed to get stdout".to_string())
        })?;

        self.child = Some(child);

        // Create write channel
        let (write_tx, mut write_rx) = mpsc::channel::<String>(32);
        self.write_tx = Some(write_tx);

        // Spawn write task
        let mut stdin = stdin;
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if stdin.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Spawn read task
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }

                // Try to parse as response
                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    let id = match &response.id {
                        super::types::RequestId::Number(n) => *n,
                        super::types::RequestId::String(s) => s.parse().unwrap_or(-1),
                    };

                    let mut pending_guard = pending.lock().await;
                    if let Some(request) = pending_guard.remove(&id) {
                        let result = if let Some(error) = response.error {
                            Err(McpError::ServerError {
                                code: error.code,
                                message: error.message,
                            })
                        } else {
                            Ok(response.result.unwrap_or(Value::Null))
                        };
                        let _ = request.sender.send(result);
                    }
                }
                // TODO: Handle notifications
            }
        });

        // Initialize
        self.initialize().await?;

        self.state = ServerState::Connected;
        Ok(())
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        self.state = ServerState::ShuttingDown;
        self.write_tx = None;

        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }

        self.state = ServerState::Disconnected;
        self.info = None;
        Ok(())
    }

    async fn list_tools(&self) -> McpResult<ListToolsResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let result = self.request("tools/list", None).await?;
        let list: ListToolsResult = serde_json::from_value(result)?;
        Ok(list)
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let result = self
            .request("tools/call", Some(serde_json::to_value(&params)?))
            .await?;
        let call_result: CallToolResult = serde_json::from_value(result)?;
        Ok(call_result)
    }

    async fn list_resources(&self) -> McpResult<ListResourcesResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let result = self.request("resources/list", None).await?;
        let list: ListResourcesResult = serde_json::from_value(result)?;
        Ok(list)
    }

    async fn read_resource(&self, uri: &str) -> McpResult<ReadResourceResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let params = serde_json::json!({ "uri": uri });
        let result = self.request("resources/read", Some(params)).await?;
        let read_result: ReadResourceResult = serde_json::from_value(result)?;
        Ok(read_result)
    }

    async fn list_prompts(&self) -> McpResult<ListPromptsResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let result = self.request("prompts/list", None).await?;
        let list: ListPromptsResult = serde_json::from_value(result)?;
        Ok(list)
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> McpResult<GetPromptResult> {
        if self.state != ServerState::Connected {
            return Err(McpError::Disconnected);
        }

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        let result = self.request("prompts/get", Some(params)).await?;
        let prompt_result: GetPromptResult = serde_json::from_value(result)?;
        Ok(prompt_result)
    }
}

impl std::fmt::Debug for McpStdioServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpStdioServer")
            .field("name", &self.config.name)
            .field("state", &self.state)
            .field("command", &self.config.command)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_server_creation() {
        let config = McpServerConfig::stdio("test", "echo");
        let server = McpStdioServer::new(config);

        assert_eq!(server.name(), "test");
        assert_eq!(server.state(), ServerState::Disconnected);
        assert!(server.info().is_none());
    }

    #[test]
    fn test_stdio_server_request_id() {
        let config = McpServerConfig::stdio("test", "echo");
        let server = McpStdioServer::new(config);

        let id1 = server.next_request_id();
        let id2 = server.next_request_id();
        let id3 = server.next_request_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[tokio::test]
    async fn test_stdio_server_disconnect_when_not_connected() {
        let config = McpServerConfig::stdio("test", "echo");
        let mut server = McpStdioServer::new(config);

        // Should not fail when not connected
        let result = server.disconnect().await;
        assert!(result.is_ok());
    }
}
