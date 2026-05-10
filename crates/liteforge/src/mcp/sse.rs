//! MCP SSE (Server-Sent Events) transport implementation.
//!
//! This module implements MCP communication over Server-Sent Events.

use super::config::{AuthConfig, McpServerConfig};
use super::server::{McpError, McpResult, McpServer, ServerInfo, ServerState};
use super::types::{
    CallToolParams, CallToolResult, ClientInfo, GetPromptResult, InitializeParams,
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, JSONRPC_VERSION, MCP_VERSION,
};
use async_trait::async_trait;
use reqwest::{header, Client};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// A pending request waiting for a response.
struct PendingRequest {
    sender: oneshot::Sender<McpResult<Value>>,
}

/// MCP server connection over SSE.
///
/// SSE connections maintain a persistent event stream from the server,
/// while sending requests via HTTP POST.
pub struct McpSseServer {
    config: McpServerConfig,
    state: ServerState,
    info: Option<ServerInfo>,
    client: Client,
    request_id: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, PendingRequest>>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
}

impl McpSseServer {
    /// Create a new SSE server connection.
    pub fn new(config: McpServerConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();

        Self {
            config,
            state: ServerState::Disconnected,
            info: None,
            client,
            request_id: AtomicI64::new(1),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_task: None,
        }
    }

    /// Get the next request ID.
    fn next_request_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Get the base URL.
    fn base_url(&self) -> McpResult<&str> {
        self.config
            .url
            .as_deref()
            .ok_or_else(|| McpError::ConnectionFailed("No URL specified".to_string()))
    }

    /// Get the messages endpoint URL.
    fn messages_url(&self) -> McpResult<String> {
        let base = self.base_url()?;
        Ok(format!("{}/messages", base.trim_end_matches('/')))
    }

    /// Add authentication headers.
    fn auth_header(&self) -> Option<(String, String)> {
        match &self.config.auth {
            AuthConfig::None => None,
            AuthConfig::Bearer { token } => {
                Some(("Authorization".to_string(), format!("Bearer {}", token)))
            }
            AuthConfig::ApiKey { header, key } => Some((header.clone(), key.clone())),
            AuthConfig::OAuth { .. } => None,
        }
    }

    /// Send a JSON-RPC request.
    async fn request(&self, method: &str, params: Option<Value>) -> McpResult<Value> {
        let messages_url = self.messages_url()?;

        let id = self.next_request_id();
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, PendingRequest { sender: tx });
        }

        // Build and send request
        let mut builder = self
            .client
            .post(&messages_url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request);

        if let Some((key, value)) = self.auth_header() {
            builder = builder.header(key, value);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| McpError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let mut pending = self.pending.lock().await;
            pending.remove(&id);
            return Err(McpError::ConnectionFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // For SSE, responses come through the event stream
        // Wait for response with timeout
        let timeout = self.config.timeout;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::Disconnected),
            Err(_) => {
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

        let result = self
            .request("initialize", Some(serde_json::to_value(&params)?))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;

        // Send initialized notification
        let _ = self.request("notifications/initialized", None).await;

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

    /// Start the SSE event listener.
    async fn start_event_listener(&mut self) -> McpResult<()> {
        let base_url = self.base_url()?.to_string();
        let pending = self.pending.clone();
        let auth = self.auth_header();
        let client = self.client.clone();

        let task = tokio::spawn(async move {
            let events_url = format!("{}/events", base_url.trim_end_matches('/'));

            let mut builder = client
                .get(&events_url)
                .header(header::ACCEPT, "text/event-stream");

            if let Some((key, value)) = auth {
                builder = builder.header(key, value);
            }

            if let Ok(response) = builder.send().await {
                if response.status().is_success() {
                    // Read SSE events
                    let mut bytes = response.bytes_stream();
                    use futures::StreamExt;
                    let mut buffer = String::new();

                    while let Some(chunk) = bytes.next().await {
                        if let Ok(data) = chunk {
                            buffer.push_str(&String::from_utf8_lossy(&data));

                            // Process complete events
                            while let Some(idx) = buffer.find("\n\n") {
                                let event = buffer[..idx].to_string();
                                buffer = buffer[idx + 2..].to_string();

                                // Parse SSE event
                                if let Some(data_line) =
                                    event.lines().find(|l| l.starts_with("data:"))
                                {
                                    let data = data_line.trim_start_matches("data:").trim();
                                    if let Ok(response) =
                                        serde_json::from_str::<JsonRpcResponse>(data)
                                    {
                                        let id = match &response.id {
                                            super::types::RequestId::Number(n) => *n,
                                            super::types::RequestId::String(s) => {
                                                s.parse().unwrap_or(-1)
                                            }
                                        };

                                        let mut pending_guard = pending.lock().await;
                                        if let Some(req) = pending_guard.remove(&id) {
                                            let result = if let Some(error) = response.error {
                                                Err(McpError::ServerError {
                                                    code: error.code,
                                                    message: error.message,
                                                })
                                            } else {
                                                Ok(response.result.unwrap_or(Value::Null))
                                            };
                                            let _ = req.sender.send(result);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        self.event_task = Some(task);
        Ok(())
    }
}

#[async_trait]
impl McpServer for McpSseServer {
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

        // Verify URL is set
        let _ = self.base_url()?;

        // Start event listener
        self.start_event_listener().await?;

        // Give the event listener a moment to establish
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Try to initialize
        match self.initialize().await {
            Ok(_) => {
                self.state = ServerState::Connected;
                Ok(())
            }
            Err(e) => {
                self.state = ServerState::Failed;
                if let Some(task) = self.event_task.take() {
                    task.abort();
                }
                Err(e)
            }
        }
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        self.state = ServerState::ShuttingDown;

        if let Some(task) = self.event_task.take() {
            task.abort();
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

impl std::fmt::Debug for McpSseServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpSseServer")
            .field("name", &self.config.name)
            .field("state", &self.state)
            .field("url", &self.config.url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_server_creation() {
        let config = McpServerConfig::sse("test", "https://example.com/mcp");
        let server = McpSseServer::new(config);

        assert_eq!(server.name(), "test");
        assert_eq!(server.state(), ServerState::Disconnected);
        assert!(server.info().is_none());
    }

    #[test]
    fn test_sse_server_urls() {
        let config = McpServerConfig::sse("test", "https://example.com/mcp");
        let server = McpSseServer::new(config);

        assert_eq!(server.base_url().unwrap(), "https://example.com/mcp");
        assert_eq!(
            server.messages_url().unwrap(),
            "https://example.com/mcp/messages"
        );
    }

    #[test]
    fn test_sse_server_request_id() {
        let config = McpServerConfig::sse("test", "https://example.com/mcp");
        let server = McpSseServer::new(config);

        let id1 = server.next_request_id();
        let id2 = server.next_request_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_sse_server_auth_header() {
        let config =
            McpServerConfig::sse("test", "https://example.com/mcp").with_bearer_token("secret123");
        let server = McpSseServer::new(config);

        let auth = server.auth_header();
        assert!(auth.is_some());
        let (header, value) = auth.unwrap();
        assert_eq!(header, "Authorization");
        assert_eq!(value, "Bearer secret123");
    }

    #[tokio::test]
    async fn test_sse_server_disconnect() {
        let config = McpServerConfig::sse("test", "https://example.com/mcp");
        let mut server = McpSseServer::new(config);

        let result = server.disconnect().await;
        assert!(result.is_ok());
        assert_eq!(server.state(), ServerState::Disconnected);
    }
}
