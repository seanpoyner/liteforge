//! LiteForge Client implementation.

use crate::config::ForgeConfig;
use crate::error::Result;
use crate::transport;
use crate::types::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionRequest, EmbeddingRequest,
    EmbeddingResponse, Message, ModelList,
};
use futures::Stream;
use reqwest::Method;
use std::pin::Pin;

/// Synchronous LiteForge client.
///
/// Wraps [`AsyncForgeClient`] with a stored Tokio runtime so each call
/// does **not** spin up a fresh runtime.
///
/// # Example
///
/// ```no_run
/// use liteforge::{ForgeClient, Message};
///
/// let client = ForgeClient::new();
/// let response = client.complete(vec![Message::user("Hello!")]).unwrap();
/// println!("{}", response.content().unwrap_or(""));
/// ```
#[derive(Debug)]
pub struct ForgeClient {
    inner: AsyncForgeClient,
    runtime: tokio::runtime::Runtime,
}

impl ForgeClient {
    /// Create a new client with default configuration from environment.
    pub fn new() -> Self {
        Self::with_config(ForgeConfig::from_env())
    }

    /// Create a new client with the given configuration.
    pub fn with_config(config: ForgeConfig) -> Self {
        let inner = AsyncForgeClient::with_config(config);
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        Self { inner, runtime }
    }

    /// Create a new client builder.
    pub fn builder() -> ForgeClientBuilder {
        ForgeClientBuilder::default()
    }

    /// Get the configured model.
    pub fn model(&self) -> &str {
        self.inner.model()
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    /// Create a chat completion (synchronous, blocking).
    ///
    /// This method blocks the current thread. For async usage, use [`AsyncForgeClient`].
    pub fn complete(&self, messages: Vec<Message>) -> Result<ChatCompletion> {
        self.runtime.block_on(self.inner.complete(messages))
    }

    /// Create a chat completion with a specific model.
    pub fn complete_with_model(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<ChatCompletion> {
        self.runtime
            .block_on(self.inner.complete_with_model(model, messages))
    }

    /// Create a chat completion with full request options.
    pub fn chat_completions(&self, request: ChatCompletionRequest) -> Result<ChatCompletion> {
        self.runtime.block_on(self.inner.chat_completions(request))
    }

    /// List available models.
    pub fn list_models(&self) -> Result<ModelList> {
        self.runtime.block_on(self.inner.list_models())
    }

    /// Create an embedding for a single text.
    pub fn embed(&self, text: impl Into<String>) -> Result<EmbeddingResponse> {
        self.runtime.block_on(self.inner.embed(text))
    }

    /// Create embeddings for multiple texts.
    pub fn embed_batch(&self, texts: Vec<String>) -> Result<EmbeddingResponse> {
        self.runtime.block_on(self.inner.embed_batch(texts))
    }

    /// Create embeddings with full request options.
    pub fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        self.runtime.block_on(self.inner.embeddings(request))
    }
}

impl Default for ForgeClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Asynchronous LiteForge client.
///
/// Provides async OpenAI-style access to chat completions.
/// All HTTP logic is delegated to the [`transport`](crate::transport) module.
///
/// # Example
///
/// ```no_run
/// use liteforge::{AsyncForgeClient, Message};
///
/// #[tokio::main]
/// async fn main() {
///     let client = AsyncForgeClient::new();
///     let response = client.complete(vec![Message::user("Hello!")]).await.unwrap();
///     println!("{}", response.content().unwrap_or(""));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AsyncForgeClient {
    config: ForgeConfig,
    http: reqwest::Client,
}

impl AsyncForgeClient {
    /// Create a new async client with default configuration from environment.
    pub fn new() -> Self {
        Self::with_config(ForgeConfig::from_env())
    }

    /// Create a new async client with the given configuration.
    pub fn with_config(config: ForgeConfig) -> Self {
        let mut builder = reqwest::Client::builder().timeout(config.timeout);
        if let Ok(path) = std::env::var("LITEFORGE_EXTRA_CA_FILE") {
            match std::fs::read(&path) {
                Ok(pem) => match reqwest::Certificate::from_pem_bundle(&pem) {
                    Ok(certs) => {
                        for c in certs {
                            builder = builder.add_root_certificate(c);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "LITEFORGE_EXTRA_CA_FILE={} could not be parsed as a PEM bundle: {}",
                            path,
                            e
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "LITEFORGE_EXTRA_CA_FILE={} could not be read: {}",
                        path,
                        e
                    );
                }
            }
        }
        let http = builder.build().expect("Failed to build HTTP client");

        Self { config, http }
    }

    /// Get the configured model.
    pub fn model(&self) -> &str {
        &self.config.default_model
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Create a chat completion.
    pub async fn complete(&self, messages: Vec<Message>) -> Result<ChatCompletion> {
        self.complete_with_model(&self.config.default_model, messages)
            .await
    }

    /// Create a chat completion with a specific model.
    pub async fn complete_with_model(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<ChatCompletion> {
        let request = ChatCompletionRequest::new(model, messages);
        self.chat_completions(request).await
    }

    /// Create a chat completion with full request options.
    pub async fn chat_completions(&self, request: ChatCompletionRequest) -> Result<ChatCompletion> {
        transport::request_with_body(
            &self.http,
            &self.config,
            Method::POST,
            "/chat/completions",
            &request,
        )
        .await
    }

    /// Create a streaming chat completion.
    pub async fn chat_completions_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>> {
        let mut request = request;
        request.stream = Some(true);
        transport::request_stream(&self.http, &self.config, "/chat/completions", &request).await
    }

    /// Create a streaming completion with convenience method.
    pub async fn complete_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>> {
        let request = ChatCompletionRequest::new(&self.config.default_model, messages);
        self.chat_completions_stream(request).await
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<ModelList> {
        transport::request_no_body(&self.http, &self.config, Method::GET, "/models").await
    }

    /// Create an embedding for a single text.
    pub async fn embed(&self, text: impl Into<String>) -> Result<EmbeddingResponse> {
        let request = EmbeddingRequest::new(&self.config.default_model, text);
        self.embeddings(request).await
    }

    /// Create embeddings for multiple texts.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<EmbeddingResponse> {
        let request = EmbeddingRequest::batch(&self.config.default_model, texts);
        self.embeddings(request).await
    }

    /// Create embeddings with full request options.
    pub async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        transport::request_with_body(
            &self.http,
            &self.config,
            Method::POST,
            "/embeddings",
            &request,
        )
        .await
    }

    /// Make a generic POST request to an endpoint.
    ///
    /// This is useful for calling endpoints not directly exposed by the client.
    pub async fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        transport::request_with_body(&self.http, &self.config, Method::POST, &path, body).await
    }
}

impl Default for AsyncForgeClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for [`ForgeClient`].
#[derive(Debug, Default)]
pub struct ForgeClientBuilder {
    config_builder: crate::config::ForgeConfigBuilder,
}

impl ForgeClientBuilder {
    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.api_key(api_key);
        self
    }

    /// Set the default model.
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.default_model(model);
        self
    }

    /// Set the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.base_url(url);
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.config_builder = self.config_builder.timeout_secs(secs);
        self
    }

    /// Build the synchronous client.
    pub fn build(self) -> ForgeClient {
        ForgeClient::with_config(self.config_builder.build())
    }

    /// Build the async client.
    pub fn build_async(self) -> AsyncForgeClient {
        AsyncForgeClient::with_config(self.config_builder.build())
    }
}
