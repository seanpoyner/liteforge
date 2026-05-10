use crate::error::forge_error_to_napi;
use crate::streaming::CompletionStream;
use crate::types::*;
use napi::bindgen_prelude::*;
use std::collections::HashMap;
use liteforge::{
    AsyncForgeClient as RustAsyncClient, ChatCompletionRequest as RustChatCompletionRequest,
    OtelConfig as RustOtelConfig, ForgeConfig as RustForgeConfig,
};

#[napi]
pub struct ForgeClientBuilder {
    api_key: Option<String>,
    default_model: Option<String>,
    base_url: Option<String>,
    timeout_secs: Option<u32>,
    default_headers: Option<HashMap<String, String>>,
    default_metadata: Option<HashMap<String, serde_json::Value>>,
    otel_endpoint: Option<String>,
    otel_headers: Option<HashMap<String, String>>,
    otel_service_name: Option<String>,
    otel_resource_attributes: Option<HashMap<String, String>>,
    otel_capture_prompts: Option<bool>,
}

#[napi]
impl ForgeClientBuilder {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            api_key: None,
            default_model: None,
            base_url: None,
            timeout_secs: None,
            default_headers: None,
            default_metadata: None,
            otel_endpoint: None,
            otel_headers: None,
            otel_service_name: None,
            otel_resource_attributes: None,
            otel_capture_prompts: None,
        }
    }

    #[napi]
    pub fn api_key(&mut self, key: String) -> &Self {
        self.api_key = Some(key);
        self
    }

    #[napi]
    pub fn default_model(&mut self, model: String) -> &Self {
        self.default_model = Some(model);
        self
    }

    #[napi]
    pub fn base_url(&mut self, url: String) -> &Self {
        self.base_url = Some(url);
        self
    }

    #[napi]
    pub fn timeout_secs(&mut self, secs: u32) -> &Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Static headers attached to every outbound request.
    #[napi]
    pub fn default_headers(&mut self, headers: HashMap<String, String>) -> &Self {
        self.default_headers = Some(headers);
        self
    }

    /// Sticky metadata merged into every request body's top-level `metadata`.
    #[napi]
    pub fn default_metadata(&mut self, metadata: HashMap<String, serde_json::Value>) -> &Self {
        self.default_metadata = Some(metadata);
        self
    }

    /// OTel exporter endpoint (e.g. Dynatrace tenant URL).
    #[napi]
    pub fn otel_endpoint(&mut self, endpoint: String) -> &Self {
        self.otel_endpoint = Some(endpoint);
        self
    }

    /// OTel exporter headers (e.g. {"Authorization": "Api-Token …"}).
    #[napi]
    pub fn otel_headers(&mut self, headers: HashMap<String, String>) -> &Self {
        self.otel_headers = Some(headers);
        self
    }

    /// `service.name` resource attribute.
    #[napi]
    pub fn otel_service_name(&mut self, name: String) -> &Self {
        self.otel_service_name = Some(name);
        self
    }

    /// Additional OTel resource attributes (e.g. `deployment.environment`).
    #[napi]
    pub fn otel_resource_attributes(&mut self, attrs: HashMap<String, String>) -> &Self {
        self.otel_resource_attributes = Some(attrs);
        self
    }

    /// Capture prompt/completion content on LLM spans (off by default;
    /// non-prod debug only).
    #[napi]
    pub fn otel_capture_prompts(&mut self, enabled: bool) -> &Self {
        self.otel_capture_prompts = Some(enabled);
        self
    }

    #[napi]
    pub fn build(&self) -> AsyncForgeClient {
        let mut builder = liteforge::ForgeConfig::builder();
        if let Some(ref key) = self.api_key {
            builder = builder.api_key(key.clone());
        }
        if let Some(ref model) = self.default_model {
            builder = builder.default_model(model.clone());
        }
        if let Some(ref url) = self.base_url {
            builder = builder.base_url(url.clone());
        }
        if let Some(secs) = self.timeout_secs {
            builder = builder.timeout_secs(secs as u64);
        }
        if let Some(ref headers) = self.default_headers {
            builder = builder.default_headers(headers.clone());
        }
        if let Some(ref meta) = self.default_metadata {
            builder = builder.default_metadata(meta.clone());
        }

        let any_otel = self.otel_endpoint.is_some()
            || self.otel_headers.is_some()
            || self.otel_service_name.is_some()
            || self.otel_resource_attributes.is_some()
            || self.otel_capture_prompts.is_some();
        if any_otel {
            let otel = RustOtelConfig {
                endpoint: self.otel_endpoint.clone(),
                headers: self.otel_headers.clone().unwrap_or_default(),
                service_name: self.otel_service_name.clone(),
                resource_attributes: self.otel_resource_attributes.clone().unwrap_or_default(),
                capture_prompts: self.otel_capture_prompts.unwrap_or(false),
            };
            builder = builder.otel(otel);
        }

        AsyncForgeClient {
            inner: RustAsyncClient::with_config(builder.build()),
        }
    }
}

#[napi]
pub struct AsyncForgeClient {
    pub(crate) inner: RustAsyncClient,
}

#[napi]
impl AsyncForgeClient {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustAsyncClient::new(),
        }
    }

    #[napi(factory)]
    pub fn with_config(
        api_key: Option<String>,
        default_model: Option<String>,
        base_url: Option<String>,
        timeout_secs: Option<u32>,
    ) -> Self {
        let mut builder = RustForgeConfig::builder();
        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }
        if let Some(model) = default_model {
            builder = builder.default_model(model);
        }
        if let Some(url) = base_url {
            builder = builder.base_url(url);
        }
        if let Some(secs) = timeout_secs {
            builder = builder.timeout_secs(secs as u64);
        }
        Self {
            inner: RustAsyncClient::with_config(builder.build()),
        }
    }

    #[napi(factory)]
    pub fn builder() -> ForgeClientBuilder {
        ForgeClientBuilder::new()
    }

    #[napi(getter)]
    pub fn model(&self) -> String {
        self.inner.model().to_string()
    }

    #[napi(getter)]
    pub fn base_url(&self) -> String {
        self.inner.base_url().to_string()
    }

    #[napi]
    pub async fn complete(&self, messages: Vec<Message>) -> Result<ChatCompletion> {
        let rust_msgs: Vec<_> = messages.iter().map(js_message_to_rust).collect();
        let result = self
            .inner
            .complete(rust_msgs)
            .await
            .map_err(forge_error_to_napi)?;
        Ok(rust_completion_to_js(&result))
    }

    #[napi]
    pub async fn complete_with_model(
        &self,
        model: String,
        messages: Vec<Message>,
    ) -> Result<ChatCompletion> {
        let rust_msgs: Vec<_> = messages.iter().map(js_message_to_rust).collect();
        let result = self
            .inner
            .complete_with_model(&model, rust_msgs)
            .await
            .map_err(forge_error_to_napi)?;
        Ok(rust_completion_to_js(&result))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub async fn chat_completions(
        &self,
        model: String,
        messages: Vec<Message>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        tools: Option<Vec<JsToolDefinition>>,
        top_p: Option<f64>,
        stop: Option<Vec<String>>,
        presence_penalty: Option<f64>,
        frequency_penalty: Option<f64>,
        user: Option<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<ChatCompletion> {
        let rust_msgs: Vec<_> = messages.iter().map(js_message_to_rust).collect();
        let mut request = RustChatCompletionRequest::new(model, rust_msgs);
        if let Some(t) = temperature {
            request = request.temperature(t as f32);
        }
        if let Some(mt) = max_tokens {
            request = request.max_tokens(mt);
        }
        if let Some(t) = tools {
            let rust_tools: Vec<_> = t.iter().map(js_tool_def_to_rust).collect();
            request = request.tools(rust_tools);
        }
        if let Some(tp) = top_p {
            request.top_p = Some(tp as f32);
        }
        if let Some(s) = stop {
            request.stop = Some(s);
        }
        if let Some(pp) = presence_penalty {
            request.presence_penalty = Some(pp as f32);
        }
        if let Some(fp) = frequency_penalty {
            request.frequency_penalty = Some(fp as f32);
        }
        if let Some(u) = user {
            request.user = Some(u);
        }
        if let Some(m) = metadata {
            request = request.metadata(m);
        }
        let result = self
            .inner
            .chat_completions(request)
            .await
            .map_err(forge_error_to_napi)?;
        Ok(rust_completion_to_js(&result))
    }

    #[napi]
    pub async fn complete_stream(&self, messages: Vec<Message>) -> Result<CompletionStream> {
        let rust_msgs: Vec<_> = messages.iter().map(js_message_to_rust).collect();
        let stream = self
            .inner
            .complete_stream(rust_msgs)
            .await
            .map_err(forge_error_to_napi)?;
        Ok(CompletionStream::new(stream))
    }

    #[napi]
    pub async fn list_models(&self) -> Result<JsModelList> {
        let result = self.inner.list_models().await.map_err(forge_error_to_napi)?;
        Ok(rust_model_list_to_js(&result))
    }

    #[napi]
    pub async fn embed(&self, text: String) -> Result<JsEmbeddingResponse> {
        let result = self.inner.embed(text).await.map_err(forge_error_to_napi)?;
        Ok(rust_embedding_response_to_js(&result))
    }

    #[napi]
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<JsEmbeddingResponse> {
        let result = self
            .inner
            .embed_batch(texts)
            .await
            .map_err(forge_error_to_napi)?;
        Ok(rust_embedding_response_to_js(&result))
    }
}
