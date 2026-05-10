//! MCP HTTP transport implementation.
//!
//! This module implements MCP communication over HTTP/REST endpoints.

use super::config::{AuthConfig, McpServerConfig};
use super::server::{McpError, McpResult, McpServer, ServerInfo, ServerState};
use super::types::{
    CallToolParams, CallToolResult, ClientInfo, GetPromptResult, InitializeParams,
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, JSONRPC_VERSION, MCP_VERSION,
};
use async_trait::async_trait;
use reqwest::{header, Client, RequestBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

/// MCP server connection over HTTP.
pub struct McpHttpServer {
    config: McpServerConfig,
    state: ServerState,
    info: Option<ServerInfo>,
    client: Client,
    request_id: AtomicI64,
}

impl McpHttpServer {
    /// Create a new HTTP server connection.
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

    /// Add authentication to a request.
    fn add_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.config.auth {
            AuthConfig::None => builder,
            AuthConfig::Bearer { token } => {
                builder.header(header::AUTHORIZATION, format!("Bearer {}", token))
            }
            AuthConfig::ApiKey { header: h, key } => builder.header(h, key),
            AuthConfig::OAuth { .. } => {
                // OAuth would need a token manager - for now, just return as-is
                builder
            }
        }
    }

    /// Send a JSON-RPC request.
    async fn request(&self, method: &str, params: Option<Value>) -> McpResult<Value> {
        let base_url = self.base_url()?;

        let id = self.next_request_id();
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.to_string(),
            params,
        };

        let builder = self
            .client
            .post(base_url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request);

        let builder = self.add_auth(builder);

        let response = builder
            .send()
            .await
            .map_err(|e| McpError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(McpError::ConnectionFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let json_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::InvalidResponse(e.to_string()))?;

        if let Some(error) = json_response.error {
            return Err(McpError::ServerError {
                code: error.code,
                message: error.message,
            });
        }

        Ok(json_response.result.unwrap_or(Value::Null))
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

        // Send initialized notification (fire and forget)
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
}

#[async_trait]
impl McpServer for McpHttpServer {
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

        // Try to initialize
        match self.initialize().await {
            Ok(_) => {
                self.state = ServerState::Connected;
                Ok(())
            }
            Err(e) => {
                self.state = ServerState::Failed;
                Err(e)
            }
        }
    }

    async fn disconnect(&mut self) -> McpResult<()> {
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

impl std::fmt::Debug for McpHttpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpHttpServer")
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
    fn test_http_server_creation() {
        let config = McpServerConfig::http("test", "https://example.com/mcp");
        let server = McpHttpServer::new(config);

        assert_eq!(server.name(), "test");
        assert_eq!(server.state(), ServerState::Disconnected);
        assert!(server.info().is_none());
    }

    #[test]
    fn test_http_server_base_url() {
        let config = McpServerConfig::http("test", "https://example.com/mcp");
        let server = McpHttpServer::new(config);

        assert_eq!(server.base_url().unwrap(), "https://example.com/mcp");
    }

    #[test]
    fn test_http_server_no_url() {
        let mut config = McpServerConfig::http("test", "https://example.com/mcp");
        config.url = None;
        let server = McpHttpServer::new(config);

        assert!(server.base_url().is_err());
    }

    #[test]
    fn test_http_server_request_id() {
        let config = McpServerConfig::http("test", "https://example.com/mcp");
        let server = McpHttpServer::new(config);

        let id1 = server.next_request_id();
        let id2 = server.next_request_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_http_server_disconnect() {
        let config = McpServerConfig::http("test", "https://example.com/mcp");
        let mut server = McpHttpServer::new(config);

        let result = server.disconnect().await;
        assert!(result.is_ok());
        assert_eq!(server.state(), ServerState::Disconnected);
    }
}
