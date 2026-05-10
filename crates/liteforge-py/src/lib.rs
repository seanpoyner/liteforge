//! Python bindings for LiteForge.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value as JsonValue;
use std::pin::Pin;
use std::sync::Arc;
use liteforge_core::futures::{Stream, StreamExt};
use liteforge_core::{
    automation::{
        AutomationConfig as RustAutomationConfig, ExecutionRecord as RustExecutionRecord,
        ScheduleConfig as RustScheduleConfig, TaskContext as RustTaskContext,
        TaskOutput as RustTaskOutput, TaskStatus as RustTaskStatus,
    },
    chunk as rust_chunk,
    conversation::{
        CompactingConversation as RustCompactingConversation,
        ConversationConfig as RustConversationConfig,
        ManagedConversation as RustManagedConversation,
        SummarizationStrategy as RustSummarizationStrategy,
    },
    detect_injection as rust_detect_injection, detect_pii as rust_detect_pii,
    evals::{
        EvalResult as RustEvalResult, SuiteResult as RustSuiteResult, SuiteStats as RustSuiteStats,
        TestCase as RustTestCase,
    },
    events::{
        Event as RustEvent, EventBus as RustEventBus, EventData as RustEventData,
        EventType as RustEventType,
    },
    find_pii as rust_find_pii,
    hitl::{
        ApprovalRequest as RustApprovalRequest, ApprovalResult as RustApprovalResult,
        ApprovalStatus as RustApprovalStatus, RiskLevel as RustRiskLevel,
    },
    hooks::{
        Hook as RustHook, HookContext as RustHookContext, HookEvent as RustHookEvent,
        HookManager as RustHookManager, HookResult as RustHookResult,
    },
    images::{
        ImageData as RustImageData, ImageQuality as RustImageQuality,
        ImageRequest as RustImageRequest, ImageResponse as RustImageResponse,
        ImageSize as RustImageSize, ImageStyle as RustImageStyle,
        ResponseFormat as RustResponseFormat,
    },
    knowledge::{
        Document as RustDocument, KnowledgeClient, KnowledgeStats as RustKnowledgeStats,
        ListOptions as RustListOptions, LocalKnowledgeBackend as RustLocalKnowledgeBackend,
        SearchOptions as RustSearchOptions, SearchResult as RustSearchResult,
    },
    mcp::{
        McpConfig as RustMcpConfig, McpHttpServer as RustMcpHttpServer, McpServer,
        McpServerConfig as RustMcpServerConfig, McpSseServer as RustMcpSseServer,
        McpStdioServer as RustMcpStdioServer, ServerState as RustServerState,
        TransportType as RustTransportType,
    },
    observability::{
        HistogramData as RustHistogramData, MetricValue as RustMetricValue,
        MetricsCollector as RustMetricsCollector, Span as RustSpan, SpanContext as RustSpanContext,
        SpanKind as RustSpanKind, SpanStatus as RustSpanStatus, Tracer as RustTracer,
    },
    orchestration::{
        CommonIntents as RustCommonIntents, Intent as RustIntent, IntentRoute as RustIntentRoute,
        IntentRouter as RustIntentRouter, OrchestrationResult as RustOrchestrationResult,
        OrchestrationStrategy as RustOrchestrationStrategy,
        OrchestratorConfig as RustOrchestratorConfig, RoutingDecision as RustRoutingDecision,
        Session as RustSession, SessionMessage as RustSessionMessage,
        SessionStore as RustSessionStore, StepStatus as RustStepStatus, Workflow as RustWorkflow,
        WorkflowStep as RustWorkflowStep,
    },
    pipelines::{PipelineContext as RustPipelineContext, StepOutput as RustStepOutput},
    prompts::{
        CommonPrompts as RustCommonPrompts, PromptConfig as RustPromptConfig,
        PromptLibrary as RustPromptLibrary, PromptTemplate as RustPromptTemplate,
    },
    rag::{
        cosine_similarity as rust_cosine_similarity, dot_product as rust_dot_product,
        euclidean_distance as rust_euclidean_distance, normalize as rust_normalize,
        EmbeddedDocument as RustEmbeddedDocument, VectorIndex as RustVectorIndex,
        VectorSearchResult as RustVectorSearchResult,
    },
    redact_pii as rust_redact_pii,
    scheduler::{
        CronSchedule as RustCronSchedule, IntervalSchedule as RustIntervalSchedule,
        JobStatus as RustJobStatus, OnceSchedule as RustOnceSchedule, Schedule as RustSchedule,
        ScheduleType as RustScheduleType,
    },
    skills::{
        extract_skill, qa_skill, rewrite_skill, summarize_skill, translate_skill,
        PromptSkill as RustPromptSkill, Skill, SkillConfig as RustSkillConfig,
        SkillInput as RustSkillInput, SkillOutput as RustSkillOutput,
    },
    tools::{
        validate_json_schema as rust_validate_json_schema, Tool as RustTool,
        ToolExecutor as RustToolExecutor, ToolRegistry as RustToolRegistry,
        ToolResult as RustToolResult,
    },
    AsyncForgeClient as RustAsyncClient, ChatCompletionChunk, ChatCompletionRequest,
    Chunk as RustChunk, ChunkingStrategy as RustChunkingStrategy, EmbeddingRequest,
    GuardrailResult as RustGuardrailResult, Message as RustMessage, PiiType as RustPiiType,
    ForgeClient as RustClient, ForgeConfig,
};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

/// Convert a Python dict to a Rust Message.
fn dict_to_message(dict: &Bound<'_, PyDict>) -> PyResult<RustMessage> {
    let role: String = dict
        .get_item("role")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'role' key"))?
        .extract()?;

    let content: Option<String> = dict.get_item("content")?.map(|v| v.extract()).transpose()?;

    Ok(RustMessage {
        role,
        content,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    })
}

/// Convert a list of Python dicts to Rust Messages.
fn list_to_messages(list: &Bound<'_, PyList>) -> PyResult<Vec<RustMessage>> {
    list.iter()
        .map(|item| {
            let dict = item.downcast::<PyDict>()?;
            dict_to_message(dict)
        })
        .collect()
}

/// Convert a Rust ChatCompletion to a Python dict.
fn completion_to_dict(
    py: Python<'_>,
    completion: liteforge_core::ChatCompletion,
) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("id", &completion.id)?;
    dict.set_item("object", &completion.object)?;
    dict.set_item("created", completion.created)?;
    dict.set_item("model", &completion.model)?;

    // Choices
    let choices = PyList::empty_bound(py);
    for choice in &completion.choices {
        let choice_dict = PyDict::new_bound(py);
        choice_dict.set_item("index", choice.index)?;

        let message_dict = PyDict::new_bound(py);
        message_dict.set_item("role", &choice.message.role)?;
        message_dict.set_item("content", &choice.message.content)?;
        choice_dict.set_item("message", message_dict)?;

        choice_dict.set_item("finish_reason", &choice.finish_reason)?;
        choices.append(choice_dict)?;
    }
    dict.set_item("choices", choices)?;

    // Usage
    if let Some(usage) = &completion.usage {
        let usage_dict = PyDict::new_bound(py);
        usage_dict.set_item("prompt_tokens", usage.prompt_tokens)?;
        usage_dict.set_item("completion_tokens", usage.completion_tokens)?;
        usage_dict.set_item("total_tokens", usage.total_tokens)?;
        dict.set_item("usage", usage_dict)?;
    }

    Ok(dict.into())
}

/// Convert a Rust EmbeddingResponse to a Python dict.
fn embedding_response_to_dict(
    py: Python<'_>,
    response: liteforge_core::EmbeddingResponse,
) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("object", &response.object)?;
    dict.set_item("model", &response.model)?;

    // Data (embeddings)
    let data = PyList::empty_bound(py);
    for embedding in &response.data {
        let emb_dict = PyDict::new_bound(py);
        emb_dict.set_item("object", &embedding.object)?;
        emb_dict.set_item("index", embedding.index)?;
        emb_dict.set_item("embedding", embedding.embedding.clone())?;
        data.append(emb_dict)?;
    }
    dict.set_item("data", data)?;

    // Usage
    let usage_dict = PyDict::new_bound(py);
    usage_dict.set_item("prompt_tokens", response.usage.prompt_tokens)?;
    usage_dict.set_item("total_tokens", response.usage.total_tokens)?;
    dict.set_item("usage", usage_dict)?;

    Ok(dict.into())
}

/// Convert a Rust ChatCompletionChunk to a Python dict.
fn chunk_to_dict(py: Python<'_>, chunk: ChatCompletionChunk) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("id", &chunk.id)?;
    dict.set_item("object", &chunk.object)?;
    dict.set_item("created", chunk.created)?;
    dict.set_item("model", &chunk.model)?;

    let choices = PyList::empty_bound(py);
    for choice in &chunk.choices {
        let choice_dict = PyDict::new_bound(py);
        choice_dict.set_item("index", choice.index)?;

        let delta_dict = PyDict::new_bound(py);
        delta_dict.set_item("role", &choice.delta.role)?;
        delta_dict.set_item("content", &choice.delta.content)?;
        choice_dict.set_item("delta", delta_dict)?;

        choice_dict.set_item("finish_reason", &choice.finish_reason)?;
        choices.append(choice_dict)?;
    }
    dict.set_item("choices", choices)?;

    Ok(dict.into())
}

/// LiteForge Client for Python.
///
/// Provides high-performance chat completions powered by Rust.
///
/// Example:
///     ```python
///     from liteforge import ForgeClient
///
///     client = ForgeClient()
///     response = client.complete([{"role": "user", "content": "Hello!"}])
///     print(response["choices"][0]["message"]["content"])
///     ```
#[pyclass]
pub struct ForgeClient {
    inner: RustClient,
    #[allow(dead_code)]
    runtime: Runtime,
}

#[pymethods]
impl ForgeClient {
    /// Create a new LiteForge client.
    ///
    /// Args:
    ///     api_key: Optional API key (defaults to LITEFORGE_API_KEY env var)
    ///     base_url: Optional base URL for the API
    ///     default_model: Optional default model to use
    ///     timeout: Optional timeout in seconds
    #[new]
    #[pyo3(signature = (api_key=None, base_url=None, default_model=None, timeout=None))]
    fn new(
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: Option<String>,
        timeout: Option<u64>,
    ) -> PyResult<Self> {
        let mut builder = ForgeConfig::builder();

        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }
        if let Some(url) = base_url {
            builder = builder.base_url(url);
        }
        if let Some(model) = default_model {
            builder = builder.default_model(model);
        }
        if let Some(secs) = timeout {
            builder = builder.timeout_secs(secs);
        }

        let config = builder.build();
        let inner = RustClient::with_config(config);

        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self { inner, runtime })
    }

    /// Create a chat completion.
    ///
    /// Args:
    ///     messages: List of message dicts with 'role' and 'content' keys
    ///     model: Optional model override
    ///     temperature: Optional sampling temperature (0.0 to 2.0)
    ///     max_tokens: Optional max tokens to generate
    ///
    /// Returns:
    ///     A dict containing the completion response
    #[pyo3(signature = (messages, model=None, temperature=None, max_tokens=None))]
    fn complete(
        &self,
        py: Python<'_>,
        messages: &Bound<'_, PyList>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<PyObject> {
        let messages = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| self.inner.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        if let Some(temp) = temperature {
            request = request.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request = request.max_tokens(max);
        }

        let result = self
            .inner
            .chat_completions(request)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        completion_to_dict(py, result)
    }

    /// List available models.
    ///
    /// Returns:
    ///     A dict containing the model list
    fn list_models(&self, py: Python<'_>) -> PyResult<PyObject> {
        let result = self
            .inner
            .list_models()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        dict.set_item("object", &result.object)?;

        let models = PyList::empty_bound(py);
        for model in &result.data {
            let model_dict = PyDict::new_bound(py);
            model_dict.set_item("id", &model.id)?;
            model_dict.set_item("object", &model.object)?;
            model_dict.set_item("created", model.created)?;
            model_dict.set_item("owned_by", &model.owned_by)?;
            models.append(model_dict)?;
        }
        dict.set_item("data", models)?;

        Ok(dict.into())
    }

    /// Create an embedding for a single text.
    ///
    /// Args:
    ///     text: The text to embed
    ///     model: Optional model override
    ///
    /// Returns:
    ///     A dict containing the embedding response
    #[pyo3(signature = (text, model=None))]
    fn embed(&self, py: Python<'_>, text: &str, model: Option<String>) -> PyResult<PyObject> {
        let request = if let Some(m) = model {
            EmbeddingRequest::new(m, text.to_string())
        } else {
            EmbeddingRequest::new(self.inner.model(), text.to_string())
        };

        let result = self
            .inner
            .embeddings(request)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        embedding_response_to_dict(py, result)
    }

    /// Create embeddings for multiple texts.
    ///
    /// Args:
    ///     texts: List of texts to embed
    ///     model: Optional model override
    ///
    /// Returns:
    ///     A dict containing the embedding response with multiple embeddings
    #[pyo3(signature = (texts, model=None))]
    fn embed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
        model: Option<String>,
    ) -> PyResult<PyObject> {
        let request = if let Some(m) = model {
            EmbeddingRequest::batch(m, texts)
        } else {
            EmbeddingRequest::batch(self.inner.model(), texts)
        };

        let result = self
            .inner
            .embeddings(request)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        embedding_response_to_dict(py, result)
    }

    /// Create a chat completion with automatic retry on transient errors.
    ///
    /// Automatically retries on rate limits (429), server errors (5xx),
    /// network errors, and timeouts.
    ///
    /// Args:
    ///     messages: List of message dicts with 'role' and 'content' keys
    ///     retry_config: Optional retry configuration (default: 3 retries)
    ///     model: Optional model override
    ///     temperature: Optional sampling temperature (0.0 to 2.0)
    ///     max_tokens: Optional max tokens to generate
    ///
    /// Returns:
    ///     A dict containing the completion response
    ///
    /// Example:
    ///     ```python
    ///     from liteforge import ForgeClient, RetryConfig
    ///
    ///     client = ForgeClient()
    ///     config = RetryConfig(max_retries=5, initial_delay_ms=1000)
    ///     response = client.complete_with_retry(
    ///         [{"role": "user", "content": "Hello!"}],
    ///         retry_config=config
    ///     )
    ///     ```
    #[pyo3(signature = (messages, retry_config=None, model=None, temperature=None, max_tokens=None))]
    fn complete_with_retry(
        &self,
        py: Python<'_>,
        messages: &Bound<'_, PyList>,
        retry_config: Option<RetryConfig>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<PyObject> {
        let messages = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| self.inner.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        if let Some(temp) = temperature {
            request = request.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request = request.max_tokens(max);
        }

        let config = retry_config.map(|c| c.inner).unwrap_or_default();
        let result =
            liteforge_core::with_retry(&config, || self.inner.chat_completions(request.clone()))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        completion_to_dict(py, result)
    }

    /// Get the configured model name.
    #[getter]
    fn model(&self) -> &str {
        self.inner.model()
    }

    /// Get the base URL.
    #[getter]
    fn base_url(&self) -> &str {
        self.inner.base_url()
    }
}

/// Async LiteForge Client for Python.
///
/// Provides async high-performance chat completions powered by Rust.
///
/// Example:
///     ```python
///     import asyncio
///     from liteforge import AsyncForgeClient
///
///     async def main():
///         client = AsyncForgeClient()
///         response = await client.complete([{"role": "user", "content": "Hello!"}])
///         print(response["choices"][0]["message"]["content"])
///
///     asyncio.run(main())
///     ```
#[pyclass]
pub struct AsyncForgeClient {
    inner: RustAsyncClient,
}

#[pymethods]
impl AsyncForgeClient {
    /// Create a new async LiteForge client.
    ///
    /// New kwargs (all optional, fall back to the matching env vars):
    ///   default_headers: dict[str, str], static headers per request
    ///   default_metadata: dict[str, Any], sticky body metadata
    ///                                       (LITEFORGE_DEFAULT_METADATA env)
    ///   otel_endpoint, otel_headers, otel_service_name,
    ///   otel_resource_attributes, otel_capture_prompts:
    ///     OTel exporter config (only honoured when the wheel was built
    ///     with --features otel; no-op otherwise). Falls back to
    ///     OTEL_EXPORTER_OTLP_*, OTEL_SERVICE_NAME, OTEL_RESOURCE_ATTRIBUTES,
    ///     LITEFORGE_OTEL_CAPTURE_PROMPTS.
    #[new]
    #[pyo3(signature = (
        api_key=None,
        base_url=None,
        default_model=None,
        timeout=None,
        default_headers=None,
        default_metadata=None,
        otel_endpoint=None,
        otel_headers=None,
        otel_service_name=None,
        otel_resource_attributes=None,
        otel_capture_prompts=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new<'py>(
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: Option<String>,
        timeout: Option<u64>,
        default_headers: Option<&Bound<'py, PyDict>>,
        default_metadata: Option<&Bound<'py, PyDict>>,
        otel_endpoint: Option<String>,
        otel_headers: Option<&Bound<'py, PyDict>>,
        otel_service_name: Option<String>,
        otel_resource_attributes: Option<&Bound<'py, PyDict>>,
        otel_capture_prompts: Option<bool>,
    ) -> PyResult<Self> {
        let mut builder = ForgeConfig::builder();

        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }
        if let Some(url) = base_url {
            builder = builder.base_url(url);
        }
        if let Some(model) = default_model {
            builder = builder.default_model(model);
        }
        if let Some(secs) = timeout {
            builder = builder.timeout_secs(secs);
        }
        if let Some(d) = default_headers {
            builder = builder.default_headers(pydict_to_string_map(d)?);
        }
        if let Some(d) = default_metadata {
            builder = builder.default_metadata(pydict_to_metadata_map(d)?);
        }

        // Build OTel config from any explicit kwargs; remaining fields
        // pick up env-var defaults inside `ForgeConfig::from_env` when the
        // builder doesn't set `.otel(...)`.
        let any_otel_kwarg = otel_endpoint.is_some()
            || otel_headers.is_some()
            || otel_service_name.is_some()
            || otel_resource_attributes.is_some()
            || otel_capture_prompts.is_some();
        if any_otel_kwarg {
            let headers = match otel_headers {
                Some(h) => pydict_to_string_map(h)?,
                None => Default::default(),
            };
            let resource_attributes = match otel_resource_attributes {
                Some(r) => pydict_to_string_map(r)?,
                None => Default::default(),
            };
            let otel = liteforge_core::OtelConfig {
                endpoint: otel_endpoint,
                headers,
                service_name: otel_service_name,
                resource_attributes,
                capture_prompts: otel_capture_prompts.unwrap_or(false),
            };
            builder = builder.otel(otel);
        }

        let config = builder.build();
        let inner = RustAsyncClient::with_config(config);

        Ok(Self { inner })
    }

    /// Create a chat completion (async).
    #[pyo3(signature = (messages, model=None, temperature=None, max_tokens=None, metadata=None))]
    fn complete<'py>(
        &self,
        py: Python<'py>,
        messages: &Bound<'py, PyList>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        metadata: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let messages = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| self.inner.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        if let Some(temp) = temperature {
            request = request.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request = request.max_tokens(max);
        }
        if let Some(meta) = metadata {
            request = request.metadata(pydict_to_metadata_map(meta)?);
        }

        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let result = inner
                .chat_completions(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| completion_to_dict(py, result))
        })
    }

    /// List available models (async).
    fn list_models<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let result = inner
                .list_models()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| -> PyResult<PyObject> {
                let dict = PyDict::new_bound(py);
                dict.set_item("object", &result.object)?;

                let models = PyList::empty_bound(py);
                for model in &result.data {
                    let model_dict = PyDict::new_bound(py);
                    model_dict.set_item("id", &model.id)?;
                    model_dict.set_item("object", &model.object)?;
                    model_dict.set_item("created", model.created)?;
                    model_dict.set_item("owned_by", &model.owned_by)?;
                    models.append(model_dict)?;
                }
                dict.set_item("data", models)?;

                Ok(dict.into())
            })
        })
    }

    /// Create a streaming chat completion (async).
    ///
    /// Returns an async iterator that yields completion chunks.
    ///
    /// Args:
    ///     messages: List of message dicts with 'role' and 'content' keys
    ///     model: Optional model override
    ///     temperature: Optional sampling temperature (0.0 to 2.0)
    ///     max_tokens: Optional max tokens to generate
    ///
    /// Returns:
    ///     An async iterator of completion chunks
    ///
    /// Example:
    ///     ```python
    ///     async for chunk in await client.complete_stream([{"role": "user", "content": "Tell me a story"}]):
    ///         if chunk["choices"][0]["delta"].get("content"):
    ///             print(chunk["choices"][0]["delta"]["content"], end="")
    ///     ```
    #[pyo3(signature = (messages, model=None, temperature=None, max_tokens=None))]
    fn complete_stream<'py>(
        &self,
        py: Python<'py>,
        messages: &Bound<'py, PyList>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let messages = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| self.inner.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        if let Some(temp) = temperature {
            request = request.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request = request.max_tokens(max);
        }

        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let stream = inner
                .chat_completions_stream(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Ok(CompletionStream {
                stream: Arc::new(Mutex::new(Some(stream))),
            })
        })
    }

    /// Create a chat completion with automatic retry on transient errors (async).
    ///
    /// Automatically retries on rate limits (429), server errors (5xx),
    /// network errors, and timeouts.
    ///
    /// Args:
    ///     messages: List of message dicts with 'role' and 'content' keys
    ///     retry_config: Optional retry configuration (default: 3 retries)
    ///     model: Optional model override
    ///     temperature: Optional sampling temperature (0.0 to 2.0)
    ///     max_tokens: Optional max tokens to generate
    ///
    /// Returns:
    ///     A dict containing the completion response
    #[pyo3(signature = (messages, retry_config=None, model=None, temperature=None, max_tokens=None))]
    fn complete_with_retry<'py>(
        &self,
        py: Python<'py>,
        messages: &Bound<'py, PyList>,
        retry_config: Option<RetryConfig>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let messages = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| self.inner.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        if let Some(temp) = temperature {
            request = request.temperature(temp);
        }
        if let Some(max) = max_tokens {
            request = request.max_tokens(max);
        }

        let inner = self.inner.clone();
        let config = retry_config.map(|c| c.inner).unwrap_or_default();

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let result = liteforge_core::with_retry_async(&config, || {
                let inner = inner.clone();
                let request = request.clone();
                async move { inner.chat_completions(request).await }
            })
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| completion_to_dict(py, result))
        })
    }

    /// Create an embedding for a single text (async).
    ///
    /// Args:
    ///     text: The text to embed
    ///     model: Optional model override
    ///
    /// Returns:
    ///     A dict containing the embedding response
    #[pyo3(signature = (text, model=None))]
    fn embed<'py>(
        &self,
        py: Python<'py>,
        text: String,
        model: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let model = model.unwrap_or_else(|| inner.model().to_string());

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let request = EmbeddingRequest::new(model, text);
            let result = inner
                .embeddings(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| embedding_response_to_dict(py, result))
        })
    }

    /// Create embeddings for multiple texts (async).
    ///
    /// Args:
    ///     texts: List of texts to embed
    ///     model: Optional model override
    ///
    /// Returns:
    ///     A dict containing the embedding response with multiple embeddings
    #[pyo3(signature = (texts, model=None))]
    fn embed_batch<'py>(
        &self,
        py: Python<'py>,
        texts: Vec<String>,
        model: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let model = model.unwrap_or_else(|| inner.model().to_string());

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let request = EmbeddingRequest::batch(model, texts);
            let result = inner
                .embeddings(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| embedding_response_to_dict(py, result))
        })
    }

    /// Get the configured model name.
    #[getter]
    fn model(&self) -> &str {
        self.inner.model()
    }

    /// Get the base URL.
    #[getter]
    fn base_url(&self) -> &str {
        self.inner.base_url()
    }
}

// ============================================================================
// Streaming
// ============================================================================

type BoxedStream = Pin<Box<dyn Stream<Item = liteforge_core::Result<ChatCompletionChunk>> + Send>>;

/// Async iterator for streaming completions.
#[pyclass]
pub struct CompletionStream {
    stream: Arc<Mutex<Option<BoxedStream>>>,
}

#[pymethods]
impl CompletionStream {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let stream = self.stream.clone();

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = stream.lock().await;
            let stream_opt = guard.as_mut();

            match stream_opt {
                None => Err(PyErr::new::<pyo3::exceptions::PyStopAsyncIteration, _>(
                    "Stream exhausted",
                )),
                Some(s) => match s.next().await {
                    Some(Ok(chunk)) => Python::with_gil(|py| chunk_to_dict(py, chunk)),
                    Some(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        e.to_string(),
                    )),
                    None => {
                        *guard = None;
                        Err(PyErr::new::<pyo3::exceptions::PyStopAsyncIteration, _>(
                            "Stream exhausted",
                        ))
                    }
                },
            }
        })
    }
}

// ============================================================================
// Chunking
// ============================================================================

/// A chunk of text with position metadata.
#[pyclass]
#[derive(Clone)]
pub struct Chunk {
    /// The chunk content
    #[pyo3(get)]
    pub text: String,
    /// Position in the original document (0-indexed)
    #[pyo3(get)]
    pub index: usize,
    /// Starting character position in original text
    #[pyo3(get)]
    pub start_char: usize,
    /// Ending character position in original text
    #[pyo3(get)]
    pub end_char: usize,
}

#[pymethods]
impl Chunk {
    fn __repr__(&self) -> String {
        format!(
            "Chunk(index={}, start={}, end={}, text={:?})",
            self.index,
            self.start_char,
            self.end_char,
            if self.text.len() > 50 {
                format!("{}...", &self.text[..50])
            } else {
                self.text.clone()
            }
        )
    }

    fn __len__(&self) -> usize {
        self.text.len()
    }
}

impl From<RustChunk> for Chunk {
    fn from(c: RustChunk) -> Self {
        Chunk {
            text: c.text,
            index: c.index,
            start_char: c.start_char,
            end_char: c.end_char,
        }
    }
}

/// Split text into chunks for RAG retrieval.
///
/// Args:
///     text: The text to chunk
///     size: Target size for each chunk in characters (default: 500)
///     overlap: Number of overlapping characters between chunks (default: 50)
///     strategy: Chunking strategy - "fixed", "recursive", "sentence", or "paragraph" (default: "recursive")
///
/// Returns:
///     A list of Chunk objects with text and position metadata.
///
/// Example:
///     ```python
///     from liteforge import chunk
///
///     chunks = chunk("Long document text...", size=500, overlap=50, strategy="recursive")
///     for c in chunks:
///         print(f"Chunk {c.index}: {c.text[:50]}...")
///     ```
#[pyfunction]
#[pyo3(signature = (text, size=500, overlap=50, strategy="recursive"))]
fn chunk(text: &str, size: usize, overlap: usize, strategy: &str) -> PyResult<Vec<Chunk>> {
    let rust_strategy = match strategy.to_lowercase().as_str() {
        "fixed" => RustChunkingStrategy::Fixed,
        "recursive" => RustChunkingStrategy::Recursive,
        "sentence" => RustChunkingStrategy::Sentence,
        "paragraph" => RustChunkingStrategy::Paragraph,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown strategy: {}. Use 'fixed', 'recursive', 'sentence', or 'paragraph'",
                strategy
            )))
        }
    };

    let chunks = rust_chunk(text, size, overlap, rust_strategy);
    Ok(chunks.into_iter().map(Chunk::from).collect())
}

// ============================================================================
// Guardrails
// ============================================================================

/// Result from a guardrail check.
#[pyclass]
#[derive(Clone)]
pub struct GuardrailResult {
    /// Whether the guardrail check passed
    #[pyo3(get)]
    pub passed: bool,
    /// The (possibly modified) value
    #[pyo3(get)]
    pub value: String,
    /// Human-readable message about the result
    #[pyo3(get)]
    pub message: String,
    /// Name of the guardrail that produced this result
    #[pyo3(get)]
    pub guardrail_name: String,
}

#[pymethods]
impl GuardrailResult {
    fn __repr__(&self) -> String {
        if self.passed {
            format!(
                "GuardrailResult(passed=True, guardrail='{}')",
                self.guardrail_name
            )
        } else {
            format!(
                "GuardrailResult(passed=False, guardrail='{}', message={:?})",
                self.guardrail_name, self.message
            )
        }
    }

    fn __bool__(&self) -> bool {
        self.passed
    }
}

impl From<RustGuardrailResult> for GuardrailResult {
    fn from(r: RustGuardrailResult) -> Self {
        GuardrailResult {
            passed: r.passed,
            value: r.value,
            message: r.message,
            guardrail_name: r.guardrail_name,
        }
    }
}

/// Detect PII (Personally Identifiable Information) in text.
///
/// Checks for SSN, phone numbers, email addresses, credit card numbers, and IP addresses.
///
/// Args:
///     text: The text to check for PII
///
/// Returns:
///     A GuardrailResult with passed=False if PII was detected
///
/// Example:
///     ```python
///     from liteforge import detect_pii
///
///     result = detect_pii("Contact me at test@example.com")
///     if not result.passed:
///         print(f"PII detected: {result.message}")
///     ```
#[pyfunction]
fn detect_pii(text: &str) -> GuardrailResult {
    rust_detect_pii(text).into()
}

/// Redact PII from text by replacing with [REDACTED].
///
/// Args:
///     text: The text containing PII to redact
///
/// Returns:
///     The text with all PII replaced by [REDACTED]
///
/// Example:
///     ```python
///     from liteforge import redact_pii
///
///     clean = redact_pii("My SSN is 123-45-6789")
///     print(clean)  # "My SSN is [REDACTED]"
///     ```
#[pyfunction]
fn redact_pii(text: &str) -> String {
    rust_redact_pii(text)
}

/// Find all PII in text with their types.
///
/// Args:
///     text: The text to search for PII
///
/// Returns:
///     A list of (type, value) tuples for all PII found
///
/// Example:
///     ```python
///     from liteforge import find_pii
///
///     found = find_pii("Email: test@example.com, Phone: 555-123-4567")
///     for pii_type, value in found:
///         print(f"{pii_type}: {value}")
///     ```
#[pyfunction]
fn find_pii(text: &str) -> Vec<(String, String)> {
    rust_find_pii(text)
        .into_iter()
        .map(|(pii_type, value)| (pii_type_to_string(pii_type), value))
        .collect()
}

fn pii_type_to_string(pii_type: RustPiiType) -> String {
    match pii_type {
        RustPiiType::Ssn => "ssn".to_string(),
        RustPiiType::Phone => "phone".to_string(),
        RustPiiType::Email => "email".to_string(),
        RustPiiType::CreditCard => "credit_card".to_string(),
        RustPiiType::IpAddress => "ip_address".to_string(),
    }
}

/// Detect potential prompt injection attacks.
///
/// Checks for common injection patterns like "ignore previous instructions",
/// role manipulation, jailbreak attempts, etc.
///
/// Args:
///     text: The text to check for injection attempts
///
/// Returns:
///     A GuardrailResult with passed=False if injection was detected
///
/// Example:
///     ```python
///     from liteforge import detect_injection
///
///     result = detect_injection("ignore all previous instructions")
///     if not result.passed:
///         print(f"Injection detected: {result.message}")
///     ```
#[pyfunction]
fn detect_injection(text: &str) -> GuardrailResult {
    rust_detect_injection(text).into()
}

/// Run all guardrails (PII and injection detection) on text.
///
/// Returns the first failure encountered, or a pass result if all checks pass.
///
/// Args:
///     text: The text to check
///
/// Returns:
///     A GuardrailResult with the outcome
///
/// Example:
///     ```python
///     from liteforge import check_all
///
///     result = check_all("Normal user message")
///     if result.passed:
///         print("All guardrails passed!")
///     ```
#[pyfunction]
fn check_all(text: &str) -> GuardrailResult {
    liteforge_core::check_all(text).into()
}

// ============================================================================
// Retry Utilities
// ============================================================================

/// Configuration for retry behavior.
///
/// Example:
///     ```python
///     from liteforge import RetryConfig
///
///     # Default config (3 retries, 500ms initial delay, 2x backoff)
///     config = RetryConfig()
///
///     # Custom config
///     config = RetryConfig(
///         max_retries=5,
///         initial_delay_ms=1000,
///         max_delay_ms=30000,
///         backoff_multiplier=2.0
///     )
///     ```
#[pyclass]
#[derive(Clone)]
pub struct RetryConfig {
    inner: liteforge_core::RetryConfig,
}

#[pymethods]
impl RetryConfig {
    /// Create a new retry configuration.
    ///
    /// Args:
    ///     max_retries: Maximum retry attempts (default: 3)
    ///     initial_delay_ms: Initial delay in milliseconds (default: 500)
    ///     max_delay_ms: Maximum delay in milliseconds (default: 30000)
    ///     backoff_multiplier: Exponential backoff multiplier (default: 2.0)
    #[new]
    #[pyo3(signature = (max_retries=3, initial_delay_ms=500, max_delay_ms=30000, backoff_multiplier=2.0))]
    fn new(
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            inner: liteforge_core::RetryConfig {
                max_retries,
                initial_delay: std::time::Duration::from_millis(initial_delay_ms),
                max_delay: std::time::Duration::from_millis(max_delay_ms),
                backoff_multiplier,
            },
        }
    }

    /// Get the max retries setting.
    #[getter]
    fn max_retries(&self) -> u32 {
        self.inner.max_retries
    }

    /// Get the initial delay in milliseconds.
    #[getter]
    fn initial_delay_ms(&self) -> u64 {
        self.inner.initial_delay.as_millis() as u64
    }

    /// Get the max delay in milliseconds.
    #[getter]
    fn max_delay_ms(&self) -> u64 {
        self.inner.max_delay.as_millis() as u64
    }

    /// Get the backoff multiplier.
    #[getter]
    fn backoff_multiplier(&self) -> f64 {
        self.inner.backoff_multiplier
    }

    fn __repr__(&self) -> String {
        format!(
            "RetryConfig(max_retries={}, initial_delay_ms={}, max_delay_ms={}, backoff_multiplier={})",
            self.inner.max_retries,
            self.inner.initial_delay.as_millis(),
            self.inner.max_delay.as_millis(),
            self.inner.backoff_multiplier
        )
    }
}

// ============================================================================
// Tools Framework
// ============================================================================

/// A Python callable wrapped as a Rust Tool.
struct PyCallableTool {
    name: String,
    description: String,
    parameters: JsonValue,
    func: PyObject,
    requires_confirmation: bool,
}

impl RustTool for PyCallableTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> JsonValue {
        self.parameters.clone()
    }

    fn execute(&self, args: JsonValue) -> Result<JsonValue, String> {
        Python::with_gil(|py| {
            // Convert args to Python dict
            let py_args =
                json_to_py(py, &args).map_err(|e| format!("Failed to convert args: {}", e))?;

            // Call the Python function
            let result = self
                .func
                .call1(py, (py_args,))
                .map_err(|e| format!("Tool execution failed: {}", e))?;

            // Convert result back to JSON
            py_to_json(result.bind(py)).map_err(|e| format!("Failed to convert result: {}", e))
        })
    }

    fn requires_confirmation(&self) -> bool {
        self.requires_confirmation
    }
}

/// Convert serde_json::Value to Python object.
fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(b) => Ok(b.into_py(py)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(py.None())
            }
        }
        JsonValue::String(s) => Ok(s.into_py(py)),
        JsonValue::Array(arr) => {
            let list = PyList::empty_bound(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        JsonValue::Object(obj) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

/// Convert Python object to serde_json::Value.
fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if obj.is_none() {
        return Ok(JsonValue::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(JsonValue::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(JsonValue::Number(i.into()));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(JsonValue::String(s));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        let arr: Result<Vec<_>, _> = list.iter().map(|item| py_to_json(&item)).collect();
        return Ok(JsonValue::Array(arr?));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, py_to_json(&v)?);
        }
        return Ok(JsonValue::Object(map));
    }
    // Fallback: try to convert to string
    let s = obj.str()?.extract::<String>()?;
    Ok(JsonValue::String(s))
}

/// Convert a Python `dict[str, str]` into a Rust `HashMap<String, String>`.
/// Used for `default_headers`, `otel_headers`, `otel_resource_attributes`.
fn pydict_to_string_map(
    d: &Bound<'_, PyDict>,
) -> PyResult<std::collections::HashMap<String, String>> {
    let mut out = std::collections::HashMap::with_capacity(d.len());
    for (k, v) in d.iter() {
        let key: String = k.extract()?;
        let val: String = v.extract().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "value for key '{}' must be a string",
                key
            ))
        })?;
        out.insert(key, val);
    }
    Ok(out)
}

/// Convert a Python dict into the metadata map used for sticky/per-call
/// metadata. Values are JSON-serialised via the existing `py_to_json`
/// helper so we accept any JSON-coercible Python value.
fn pydict_to_metadata_map(
    d: &Bound<'_, PyDict>,
) -> PyResult<std::collections::HashMap<String, JsonValue>> {
    let mut out = std::collections::HashMap::with_capacity(d.len());
    for (k, v) in d.iter() {
        let key: String = k.extract()?;
        out.insert(key, py_to_json(&v)?);
    }
    Ok(out)
}

/// Initialise the OTel tracer provider + W3C propagator from Python.
///
/// All kwargs are optional; omitted values fall back to the matching
/// `OTEL_*` env var. Idempotent, safe to call multiple times.
///
/// When the wheel was built without `--features otel`, this is a no-op
/// that succeeds silently. The `otel_feature_enabled()` helper lets
/// callers detect this case if they want to log a warning.
#[pyfunction]
#[pyo3(signature = (
    endpoint=None,
    headers=None,
    service_name=None,
    resource_attributes=None,
    capture_prompts=None,
))]
fn init_otel<'py>(
    endpoint: Option<String>,
    headers: Option<&Bound<'py, PyDict>>,
    service_name: Option<String>,
    resource_attributes: Option<&Bound<'py, PyDict>>,
    capture_prompts: Option<bool>,
) -> PyResult<()> {
    let headers = match headers {
        Some(h) => pydict_to_string_map(h)?,
        None => Default::default(),
    };
    let resource_attributes = match resource_attributes {
        Some(r) => pydict_to_string_map(r)?,
        None => Default::default(),
    };
    let otel = liteforge_core::OtelConfig {
        endpoint,
        headers,
        service_name,
        resource_attributes,
        capture_prompts: capture_prompts.unwrap_or(false),
    };

    liteforge_core::init_otel(&otel)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Returns True if this wheel was built with `--features otel`. When
/// False, `init_otel()` is a no-op.
#[pyfunction]
fn otel_feature_enabled() -> bool {
    liteforge_core::otel_feature_enabled()
}

/// Result of a tool execution.
///
/// Attributes:
///     tool_call_id: The ID of the tool call (for correlating with the original call)
///     name: Name of the tool that was executed
///     success: Whether the execution was successful
///     result: The result value (if successful), as a dict
///     error: Error message (if failed)
///     execution_time_ms: Execution time in milliseconds
#[pyclass]
#[derive(Clone)]
pub struct ToolResult {
    #[pyo3(get)]
    pub tool_call_id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub result: Option<PyObject>,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub execution_time_ms: Option<u64>,
}

#[pymethods]
impl ToolResult {
    fn __repr__(&self) -> String {
        if self.success {
            format!("ToolResult(name='{}', success=True)", self.name)
        } else {
            format!(
                "ToolResult(name='{}', success=False, error={:?})",
                self.name,
                self.error.as_deref().unwrap_or("Unknown")
            )
        }
    }

    fn __bool__(&self) -> bool {
        self.success
    }

    /// Convert to a message content string for including in conversation.
    fn to_message_content(&self) -> String {
        if self.success {
            Python::with_gil(|py| {
                if let Some(ref result) = self.result {
                    if let Ok(json) = py_to_json(result.bind(py)) {
                        return serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string());
                    }
                }
                "{}".to_string()
            })
        } else {
            format!(
                r#"{{"error": "{}"}}"#,
                self.error.as_deref().unwrap_or("Unknown error")
            )
        }
    }
}

fn rust_tool_result_to_py(py: Python<'_>, result: RustToolResult) -> PyResult<ToolResult> {
    let py_result = if let Some(value) = result.result {
        Some(json_to_py(py, &value)?)
    } else {
        None
    };

    Ok(ToolResult {
        tool_call_id: result.tool_call_id,
        name: result.name,
        success: result.success,
        result: py_result,
        error: result.error,
        execution_time_ms: result.execution_time_ms,
    })
}

/// Registry for managing collections of tools.
///
/// Example:
///     ```python
///     from liteforge import ToolRegistry, create_tool
///
///     def add(a: float, b: float) -> dict:
///         return {"result": a + b}
///
///     registry = ToolRegistry()
///     tool = create_tool(
///         name="add",
///         description="Add two numbers",
///         parameters={"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}},
///         func=add
///     )
///     registry.register(tool)
///     ```
#[pyclass]
pub struct ToolRegistry {
    inner: RustToolRegistry,
}

#[pymethods]
impl ToolRegistry {
    /// Create a new empty registry.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustToolRegistry::new(),
        }
    }

    /// Register a tool in the registry.
    ///
    /// Args:
    ///     tool: A tool created with create_tool()
    fn register(&mut self, tool: &PyTool) -> PyResult<()> {
        let rust_tool = PyCallableTool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.parameters.clone(),
            func: tool.func.clone(),
            requires_confirmation: tool.requires_confirmation,
        };
        self.inner.register(Box::new(rust_tool));
        Ok(())
    }

    /// Unregister a tool by name.
    ///
    /// Returns True if the tool was found and removed.
    fn unregister(&mut self, name: &str) -> bool {
        self.inner.unregister(name).is_some()
    }

    /// Check if a tool exists in the registry.
    fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    /// Get the number of registered tools.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Check if the registry is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get all tool names.
    fn names(&self) -> Vec<String> {
        self.inner.names().iter().map(|s| s.to_string()).collect()
    }

    /// Get tool definitions for all registered tools.
    ///
    /// Returns definitions in the OpenAI API format.
    fn definitions(&self, py: Python<'_>) -> PyResult<PyObject> {
        let defs = self.inner.definitions();
        let list = PyList::empty_bound(py);

        for def in defs {
            let dict = PyDict::new_bound(py);
            dict.set_item("type", &def.tool_type)?;

            let func_dict = PyDict::new_bound(py);
            func_dict.set_item("name", &def.function.name)?;
            func_dict.set_item("description", &def.function.description)?;

            if let Some(params) = &def.function.parameters {
                let params_dict = PyDict::new_bound(py);
                params_dict.set_item("type", &params.schema_type)?;

                // Convert properties map to dict
                let props_dict = PyDict::new_bound(py);
                for (k, v) in &params.properties {
                    props_dict.set_item(k, json_to_py(py, v)?)?;
                }
                params_dict.set_item("properties", props_dict)?;

                if let Some(required) = &params.required {
                    params_dict.set_item("required", required.clone())?;
                }

                func_dict.set_item("parameters", params_dict)?;
            }

            dict.set_item("function", func_dict)?;
            list.append(dict)?;
        }

        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("ToolRegistry(tools={:?})", self.names())
    }
}

/// Executor for running tools with validation.
///
/// Example:
///     ```python
///     from liteforge import ToolRegistry, ToolExecutor, create_tool
///
///     registry = ToolRegistry()
///     # ... register tools ...
///
///     executor = ToolExecutor(registry)
///     result = executor.execute("add", {"a": 1, "b": 2})
///     print(result.result)  # {"result": 3}
///     ```
#[pyclass]
pub struct ToolExecutor {
    inner: RustToolExecutor,
}

#[pymethods]
impl ToolExecutor {
    /// Create a new executor with the given registry.
    ///
    /// Args:
    ///     registry: The ToolRegistry containing tools to execute
    ///     validate_args: Whether to validate arguments against schemas (default: True)
    #[new]
    #[pyo3(signature = (registry, validate_args=true))]
    fn new(registry: &ToolRegistry, validate_args: bool) -> Self {
        // Clone the inner registry's tools
        let mut new_registry = RustToolRegistry::new();
        for name in registry.inner.names() {
            if let Some(tool) = registry.inner.get(name) {
                new_registry.register_arc(tool);
            }
        }

        let executor = RustToolExecutor::new(new_registry).validate_args(validate_args);
        Self { inner: executor }
    }

    /// Execute a tool by name with the given arguments.
    ///
    /// Args:
    ///     name: Name of the tool to execute
    ///     args: Arguments as a dict
    ///
    /// Returns:
    ///     A ToolResult with the outcome
    #[pyo3(signature = (name, args))]
    fn execute(
        &self,
        py: Python<'_>,
        name: &str,
        args: &Bound<'_, PyDict>,
    ) -> PyResult<ToolResult> {
        let json_args = py_to_json(args.as_any())?;
        let result = self.inner.execute(name, json_args);
        rust_tool_result_to_py(py, result)
    }

    /// Execute a tool with a call ID for correlation.
    #[pyo3(signature = (call_id, name, args))]
    fn execute_with_id(
        &self,
        py: Python<'_>,
        call_id: &str,
        name: &str,
        args: &Bound<'_, PyDict>,
    ) -> PyResult<ToolResult> {
        let json_args = py_to_json(args.as_any())?;
        let result = self.inner.execute_with_id(call_id, name, json_args);
        rust_tool_result_to_py(py, result)
    }

    /// Execute a tool call from an LLM response.
    ///
    /// Args:
    ///     tool_call: A dict with 'id', 'type', and 'function' keys
    fn execute_call(&self, py: Python<'_>, tool_call: &Bound<'_, PyDict>) -> PyResult<ToolResult> {
        let id: String = tool_call
            .get_item("id")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'id'"))?
            .extract()?;

        let function = tool_call
            .get_item("function")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'function'"))?;
        let function = function.downcast::<PyDict>()?;

        let name: String = function
            .get_item("name")?
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'function.name'")
            })?
            .extract()?;

        let arguments: String = function
            .get_item("arguments")?
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'function.arguments'")
            })?
            .extract()?;

        // Parse arguments JSON
        let json_args: JsonValue = serde_json::from_str(&arguments).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to parse arguments JSON: {}",
                e
            ))
        })?;

        let result = self.inner.execute_with_id(&id, &name, json_args);
        rust_tool_result_to_py(py, result)
    }

    /// Execute multiple tool calls.
    fn execute_calls(
        &self,
        py: Python<'_>,
        tool_calls: &Bound<'_, PyList>,
    ) -> PyResult<Vec<ToolResult>> {
        let mut results = Vec::new();
        for call in tool_calls.iter() {
            let dict = call.downcast::<PyDict>()?;
            results.push(self.execute_call(py, dict)?);
        }
        Ok(results)
    }

    /// Check if a tool exists.
    fn has_tool(&self, name: &str) -> bool {
        self.inner.has_tool(name)
    }

    fn __repr__(&self) -> String {
        format!("ToolExecutor(validate_args={})", true)
    }
}

/// A tool definition that can be registered with a ToolRegistry.
///
/// Create with create_tool() function.
#[pyclass]
#[derive(Clone)]
pub struct PyTool {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: String,
    parameters: JsonValue,
    func: PyObject,
    #[pyo3(get)]
    pub requires_confirmation: bool,
}

#[pymethods]
impl PyTool {
    /// Get the parameters schema as a dict.
    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        json_to_py(py, &self.parameters)
    }

    /// Execute the tool with the given arguments.
    fn execute(&self, py: Python<'_>, args: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let result = self.func.call1(py, (args,))?;
        Ok(result)
    }

    fn __repr__(&self) -> String {
        format!(
            "PyTool(name='{}', description='{}')",
            self.name, self.description
        )
    }
}

/// Create a tool from a Python callable.
///
/// Args:
///     name: The unique name of the tool
///     description: A description of what the tool does
///     parameters: JSON schema for the tool's parameters (as a dict)
///     func: The Python function to call when the tool is executed
///     requires_confirmation: Whether this tool requires user confirmation (default: False)
///
/// Returns:
///     A PyTool that can be registered with a ToolRegistry
///
/// Example:
///     ```python
///     from liteforge import create_tool
///
///     def get_weather(location: str) -> dict:
///         return {"temperature": 72, "location": location}
///
///     tool = create_tool(
///         name="get_weather",
///         description="Get weather for a location",
///         parameters={
///             "type": "object",
///             "properties": {
///                 "location": {"type": "string", "description": "City name"}
///             },
///             "required": ["location"]
///         },
///         func=get_weather
///     )
///     ```
#[pyfunction]
#[pyo3(signature = (name, description, parameters, func, requires_confirmation=false))]
fn create_tool(
    name: String,
    description: String,
    parameters: &Bound<'_, PyDict>,
    func: PyObject,
    requires_confirmation: bool,
) -> PyResult<PyTool> {
    let params_json = py_to_json(parameters.as_any())?;

    Ok(PyTool {
        name,
        description,
        parameters: params_json,
        func,
        requires_confirmation,
    })
}

/// Validate a JSON value against a JSON schema.
///
/// Args:
///     value: The value to validate (as a dict)
///     schema: The JSON schema to validate against (as a dict)
///
/// Returns:
///     True if valid
///
/// Raises:
///     ValueError: If validation fails, with details about what failed
///
/// Example:
///     ```python
///     from liteforge import validate_json_schema
///
///     schema = {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}
///     validate_json_schema({"name": "Alice"}, schema)  # True
///     validate_json_schema({}, schema)  # Raises ValueError
///     ```
#[pyfunction]
fn validate_json_schema(value: &Bound<'_, PyDict>, schema: &Bound<'_, PyDict>) -> PyResult<bool> {
    let json_value = py_to_json(value.as_any())?;
    let json_schema = py_to_json(schema.as_any())?;

    match rust_validate_json_schema(&json_value, &json_schema) {
        Ok(()) => Ok(true),
        Err(errors) => {
            let error_msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Schema validation failed: {}",
                error_msg
            )))
        }
    }
}

// ============================================================================
// Knowledge API
// ============================================================================

/// A document in the knowledge base.
///
/// Example:
///     ```python
///     from liteforge import Document
///
///     doc = Document("doc1", "This is the document content")
///     doc = Document("doc2", "More content", namespace="tech")
///     doc = doc.with_metadata("author", "Alice")
///     ```
#[pyclass]
#[derive(Clone)]
pub struct Document {
    inner: RustDocument,
}

#[pymethods]
impl Document {
    /// Create a new document.
    ///
    /// Args:
    ///     id: Unique document identifier
    ///     content: Document content/text
    ///     namespace: Optional namespace for organizing documents
    #[new]
    #[pyo3(signature = (id, content, namespace=None))]
    fn new(id: String, content: String, namespace: Option<String>) -> Self {
        let mut doc = RustDocument::new(id, content);
        if let Some(ns) = namespace {
            doc = doc.namespace(ns);
        }
        Self { inner: doc }
    }

    /// Get the document ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the document content.
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    /// Get the document namespace.
    #[getter]
    fn namespace(&self) -> Option<String> {
        self.inner.namespace.clone()
    }

    /// Get metadata as a dict.
    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<PyObject> {
        json_to_py(
            py,
            &serde_json::Value::Object(
                self.inner
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
        )
    }

    /// Get the created timestamp.
    #[getter]
    fn created_at(&self) -> Option<i64> {
        self.inner.created_at
    }

    /// Get the updated timestamp.
    #[getter]
    fn updated_at(&self) -> Option<i64> {
        self.inner.updated_at
    }

    /// Add metadata to the document.
    ///
    /// Args:
    ///     key: Metadata key
    ///     value: Metadata value (will be converted to JSON)
    ///
    /// Returns:
    ///     A new Document with the metadata added
    fn with_metadata(&self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(value)?;
        let mut new_inner = self.inner.clone();
        new_inner.metadata.insert(key, json_value);
        Ok(Self { inner: new_inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "Document(id='{}', content='{}...', namespace={:?})",
            self.inner.id,
            &self.inner.content.chars().take(30).collect::<String>(),
            self.inner.namespace
        )
    }
}

/// A search result from the knowledge base.
#[pyclass]
#[derive(Clone)]
pub struct SearchResult {
    inner: RustSearchResult,
}

#[pymethods]
impl SearchResult {
    /// Get the matched document.
    #[getter]
    fn document(&self) -> Document {
        Document {
            inner: self.inner.document.clone(),
        }
    }

    /// Get the relevance score (0.0 to 1.0).
    #[getter]
    fn score(&self) -> f32 {
        self.inner.score
    }

    /// Get highlighted snippets from the document.
    #[getter]
    fn highlights(&self) -> Vec<String> {
        self.inner.highlights.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SearchResult(doc_id='{}', score={:.3})",
            self.inner.document.id, self.inner.score
        )
    }
}

/// Options for searching the knowledge base.
///
/// Example:
///     ```python
///     from liteforge import SearchOptions
///
///     opts = SearchOptions()
///     opts = opts.limit(10).namespace("tech").min_score(0.5)
///     ```
#[pyclass]
#[derive(Clone)]
pub struct SearchOptions {
    inner: RustSearchOptions,
}

#[pymethods]
impl SearchOptions {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustSearchOptions::new(),
        }
    }

    /// Set the maximum number of results.
    fn limit(&self, limit: usize) -> Self {
        Self {
            inner: self.inner.clone().limit(limit),
        }
    }

    /// Filter by namespace.
    fn namespace(&self, namespace: String) -> Self {
        Self {
            inner: self.inner.clone().namespace(namespace),
        }
    }

    /// Set minimum relevance score.
    fn min_score(&self, score: f32) -> Self {
        Self {
            inner: self.inner.clone().min_score(score),
        }
    }

    /// Include highlighted snippets in results.
    fn include_highlights(&self, include: bool) -> Self {
        Self {
            inner: self.inner.clone().include_highlights(include),
        }
    }

    /// Add a metadata filter.
    fn filter(&self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(value)?;
        Ok(Self {
            inner: self.inner.clone().filter(key, json_value),
        })
    }
}

/// Options for listing documents.
///
/// Example:
///     ```python
///     from liteforge import ListOptions
///
///     opts = ListOptions().limit(20).offset(10).namespace("tech")
///     ```
#[pyclass]
#[derive(Clone)]
pub struct ListOptions {
    inner: RustListOptions,
}

#[pymethods]
impl ListOptions {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustListOptions::new(),
        }
    }

    /// Set the maximum number of results.
    fn limit(&self, limit: usize) -> Self {
        Self {
            inner: self.inner.clone().limit(limit),
        }
    }

    /// Set the offset for pagination.
    fn offset(&self, offset: usize) -> Self {
        Self {
            inner: self.inner.clone().offset(offset),
        }
    }

    /// Filter by namespace.
    fn namespace(&self, namespace: String) -> Self {
        Self {
            inner: self.inner.clone().namespace(namespace),
        }
    }
}

/// Statistics about the knowledge base.
#[pyclass]
pub struct KnowledgeStats {
    inner: RustKnowledgeStats,
}

#[pymethods]
impl KnowledgeStats {
    /// Total number of documents.
    #[getter]
    fn document_count(&self) -> usize {
        self.inner.document_count
    }

    /// Number of namespaces.
    #[getter]
    fn namespace_count(&self) -> usize {
        self.inner.namespace_count
    }

    /// List of namespace names.
    #[getter]
    fn namespaces(&self) -> Vec<String> {
        self.inner.namespaces.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "KnowledgeStats(documents={}, namespaces={})",
            self.inner.document_count, self.inner.namespace_count
        )
    }
}

/// An in-memory knowledge backend for development and testing.
///
/// Uses simple text matching for search (not production-grade).
///
/// Example:
///     ```python
///     from liteforge import LocalKnowledgeBackend, Document, SearchOptions
///
///     backend = LocalKnowledgeBackend()
///
///     # Upload documents
///     docs = [
///         Document("1", "Rust is a systems programming language"),
///         Document("2", "Python is great for data science"),
///     ]
///     backend.upload(docs)
///
///     # Search
///     results = backend.search("rust programming", SearchOptions().limit(5))
///     for r in results:
///         print(f"{r.document.id}: {r.score}")
///     ```
#[pyclass]
pub struct LocalKnowledgeBackend {
    inner: Arc<RustLocalKnowledgeBackend>,
    runtime: Runtime,
}

#[pymethods]
impl LocalKnowledgeBackend {
    /// Create a new empty local backend.
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(RustLocalKnowledgeBackend::new()),
            runtime,
        })
    }

    /// Create a local backend with initial documents.
    #[staticmethod]
    fn with_documents(documents: Vec<Document>) -> PyResult<Self> {
        let rust_docs: Vec<RustDocument> = documents.into_iter().map(|d| d.inner).collect();
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(RustLocalKnowledgeBackend::with_documents(rust_docs)),
            runtime,
        })
    }

    /// Search for documents matching a query.
    ///
    /// Args:
    ///     query: Search query string
    ///     options: Search options (limit, namespace, filters)
    ///
    /// Returns:
    ///     List of SearchResult objects sorted by relevance
    fn search(&self, query: String, options: SearchOptions) -> PyResult<Vec<SearchResult>> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .search(&query, options.inner)
                .await
                .map(|results| {
                    results
                        .into_iter()
                        .map(|r| SearchResult { inner: r })
                        .collect()
                })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Upload documents to the knowledge base.
    ///
    /// Args:
    ///     documents: List of Document objects to upload
    ///
    /// Returns:
    ///     List of uploaded document IDs
    fn upload(&self, documents: Vec<Document>) -> PyResult<Vec<String>> {
        let rust_docs: Vec<RustDocument> = documents.into_iter().map(|d| d.inner).collect();
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .upload(rust_docs)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Get a document by ID.
    ///
    /// Args:
    ///     id: Document ID
    ///
    /// Returns:
    ///     Document if found, None otherwise
    fn get(&self, id: String) -> PyResult<Option<Document>> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .get(&id)
                .await
                .map(|opt| opt.map(|doc| Document { inner: doc }))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// List documents in the knowledge base.
    ///
    /// Args:
    ///     options: List options (limit, offset, namespace)
    ///
    /// Returns:
    ///     List of Document objects
    fn list(&self, options: ListOptions) -> PyResult<Vec<Document>> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .list(options.inner)
                .await
                .map(|docs| docs.into_iter().map(|d| Document { inner: d }).collect())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Delete a document by ID.
    ///
    /// Args:
    ///     id: Document ID
    ///
    /// Returns:
    ///     True if deleted, False if not found
    fn delete(&self, id: String) -> PyResult<bool> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .delete(&id)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Update an existing document.
    ///
    /// Args:
    ///     document: Updated document (must have existing ID)
    ///
    /// Returns:
    ///     True if updated, False if not found
    fn update(&self, document: Document) -> PyResult<bool> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .update(document.inner)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Get statistics about the knowledge base.
    ///
    /// Returns:
    ///     KnowledgeStats object
    fn stats(&self) -> PyResult<KnowledgeStats> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .stats()
                .await
                .map(|s| KnowledgeStats { inner: s })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    /// Clear documents in a namespace or all documents.
    ///
    /// Args:
    ///     namespace: Namespace to clear (None clears all)
    ///
    /// Returns:
    ///     Number of documents deleted
    #[pyo3(signature = (namespace=None))]
    fn clear(&self, namespace: Option<String>) -> PyResult<usize> {
        let inner = self.inner.clone();
        self.runtime.block_on(async move {
            inner
                .clear(namespace.as_deref())
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }
}

// ============================================================================
// RAG (Retrieval-Augmented Generation)
// ============================================================================

/// Compute cosine similarity between two vectors.
///
/// Returns a value between -1.0 and 1.0.
///
/// Example:
///     ```python
///     from liteforge import cosine_similarity
///
///     sim = cosine_similarity([1.0, 0.0], [1.0, 0.0])  # 1.0
///     sim = cosine_similarity([1.0, 0.0], [0.0, 1.0])  # 0.0 (orthogonal)
///     ```
#[pyfunction]
fn cosine_similarity(a: Vec<f32>, b: Vec<f32>) -> f32 {
    rust_cosine_similarity(&a, &b)
}

/// Compute dot product of two vectors.
#[pyfunction]
fn dot_product(a: Vec<f32>, b: Vec<f32>) -> f32 {
    rust_dot_product(&a, &b)
}

/// Compute Euclidean distance between two vectors.
#[pyfunction]
fn euclidean_distance(a: Vec<f32>, b: Vec<f32>) -> f32 {
    rust_euclidean_distance(&a, &b)
}

/// Normalize a vector to unit length.
#[pyfunction]
fn normalize(v: Vec<f32>) -> Vec<f32> {
    rust_normalize(&v)
}

/// A document with its embedding vector for similarity search.
///
/// Example:
///     ```python
///     from liteforge import EmbeddedDocument
///
///     doc = EmbeddedDocument("doc1", "Hello world", [0.1, 0.2, 0.3])
///     doc = doc.with_metadata("source", "web")
///     ```
#[pyclass]
#[derive(Clone)]
pub struct EmbeddedDocument {
    inner: RustEmbeddedDocument,
}

#[pymethods]
impl EmbeddedDocument {
    /// Create a new embedded document.
    #[new]
    fn new(id: String, content: String, embedding: Vec<f32>) -> Self {
        Self {
            inner: RustEmbeddedDocument::new(id, content, embedding),
        }
    }

    /// Get the document ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the document content.
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    /// Get the embedding vector.
    #[getter]
    fn embedding(&self) -> Vec<f32> {
        self.inner.embedding.clone()
    }

    /// Get metadata as a dict.
    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<PyObject> {
        json_to_py(
            py,
            &serde_json::Value::Object(
                self.inner
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
        )
    }

    /// Add metadata to the document.
    fn with_metadata(&self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(value)?;
        let mut new_inner = self.inner.clone();
        new_inner.metadata.insert(key, json_value);
        Ok(Self { inner: new_inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "EmbeddedDocument(id='{}', embedding_dim={})",
            self.inner.id,
            self.inner.embedding.len()
        )
    }
}

/// Result from a vector similarity search.
#[pyclass]
#[derive(Clone)]
pub struct VectorSearchResult {
    inner: RustVectorSearchResult,
}

#[pymethods]
impl VectorSearchResult {
    /// Get the matched document.
    #[getter]
    fn document(&self) -> EmbeddedDocument {
        EmbeddedDocument {
            inner: self.inner.document.clone(),
        }
    }

    /// Get the similarity score.
    #[getter]
    fn score(&self) -> f32 {
        self.inner.score
    }

    fn __repr__(&self) -> String {
        format!(
            "VectorSearchResult(id='{}', score={:.3})",
            self.inner.document.id, self.inner.score
        )
    }
}

/// An in-memory vector index for similarity search.
///
/// Uses brute-force cosine similarity. For production, use a vector database.
///
/// Example:
///     ```python
///     from liteforge import VectorIndex, EmbeddedDocument
///
///     index = VectorIndex()
///     index.add(EmbeddedDocument("1", "Hello", [1.0, 0.0, 0.0]))
///     index.add(EmbeddedDocument("2", "World", [0.0, 1.0, 0.0]))
///
///     results = index.search([0.9, 0.1, 0.0], limit=1)
///     print(results[0].document.id)  # "1"
///     ```
#[pyclass]
pub struct VectorIndex {
    inner: RustVectorIndex,
}

#[pymethods]
impl VectorIndex {
    /// Create a new empty vector index.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustVectorIndex::new(),
        }
    }

    /// Create an index with initial documents.
    #[staticmethod]
    fn with_documents(documents: Vec<EmbeddedDocument>) -> Self {
        let rust_docs: Vec<RustEmbeddedDocument> = documents.into_iter().map(|d| d.inner).collect();
        Self {
            inner: RustVectorIndex::with_documents(rust_docs),
        }
    }

    /// Add a document to the index.
    fn add(&mut self, document: EmbeddedDocument) {
        self.inner.add(document.inner);
    }

    /// Add multiple documents to the index.
    fn add_batch(&mut self, documents: Vec<EmbeddedDocument>) {
        let rust_docs: Vec<RustEmbeddedDocument> = documents.into_iter().map(|d| d.inner).collect();
        self.inner.add_batch(rust_docs);
    }

    /// Remove a document from the index.
    fn remove(&mut self, id: String) -> bool {
        self.inner.remove(&id)
    }

    /// Get a document by ID.
    fn get(&self, id: String) -> Option<EmbeddedDocument> {
        self.inner
            .get(&id)
            .map(|d| EmbeddedDocument { inner: d.clone() })
    }

    /// Search for similar documents.
    ///
    /// Args:
    ///     query: Query embedding vector
    ///     limit: Maximum number of results
    ///
    /// Returns:
    ///     List of VectorSearchResult sorted by similarity (highest first)
    fn search(&self, query: Vec<f32>, limit: usize) -> Vec<VectorSearchResult> {
        self.inner
            .search(&query, limit)
            .into_iter()
            .map(|r| VectorSearchResult { inner: r })
            .collect()
    }

    /// Search with a minimum score threshold.
    fn search_with_threshold(
        &self,
        query: Vec<f32>,
        limit: usize,
        min_score: f32,
    ) -> Vec<VectorSearchResult> {
        self.inner
            .search_with_threshold(&query, limit, min_score)
            .into_iter()
            .map(|r| VectorSearchResult { inner: r })
            .collect()
    }

    /// Get the number of documents in the index.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Check if the index is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all documents.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get all document IDs.
    fn ids(&self) -> Vec<String> {
        self.inner
            .ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}

// ============================================================================
// Events
// ============================================================================

/// Event type enumeration.
#[pyclass]
#[derive(Clone)]
pub struct EventType {
    inner: RustEventType,
}

#[pymethods]
impl EventType {
    #[staticmethod]
    fn agent_start() -> Self {
        Self {
            inner: RustEventType::AgentStart,
        }
    }

    #[staticmethod]
    fn agent_end() -> Self {
        Self {
            inner: RustEventType::AgentEnd,
        }
    }

    #[staticmethod]
    fn agent_step() -> Self {
        Self {
            inner: RustEventType::AgentStep,
        }
    }

    #[staticmethod]
    fn agent_error() -> Self {
        Self {
            inner: RustEventType::AgentError,
        }
    }

    #[staticmethod]
    fn tool_call() -> Self {
        Self {
            inner: RustEventType::ToolCall,
        }
    }

    #[staticmethod]
    fn tool_result() -> Self {
        Self {
            inner: RustEventType::ToolResult,
        }
    }

    #[staticmethod]
    fn tool_error() -> Self {
        Self {
            inner: RustEventType::ToolError,
        }
    }

    #[staticmethod]
    fn llm_request() -> Self {
        Self {
            inner: RustEventType::LlmRequest,
        }
    }

    #[staticmethod]
    fn llm_response() -> Self {
        Self {
            inner: RustEventType::LlmResponse,
        }
    }

    #[staticmethod]
    fn llm_error() -> Self {
        Self {
            inner: RustEventType::LlmError,
        }
    }

    #[staticmethod]
    fn custom() -> Self {
        Self {
            inner: RustEventType::Custom,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// An event in the SDK event system.
#[pyclass]
#[derive(Clone)]
pub struct Event {
    inner: RustEvent,
}

#[pymethods]
impl Event {
    /// Create a new event.
    #[new]
    #[pyo3(signature = (event_type, data=None))]
    fn new(event_type: EventType, data: Option<&Bound<'_, pyo3::PyAny>>) -> PyResult<Self> {
        let event = if let Some(d) = data {
            RustEvent::with_data(event_type.inner, py_to_json(d)?)
        } else {
            RustEvent::new(event_type.inner)
        };
        Ok(Self { inner: event })
    }

    /// Create a tool call event.
    #[staticmethod]
    fn tool_call(tool_name: &str, arguments: &str) -> Self {
        Self {
            inner: RustEvent::tool_call(tool_name, arguments),
        }
    }

    /// Create a tool result event.
    #[staticmethod]
    fn tool_result(tool_name: &str, result: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: RustEvent::tool_result(tool_name, py_to_json(result)?),
        })
    }

    /// Create a tool error event.
    #[staticmethod]
    fn tool_error(tool_name: &str, error_msg: &str) -> Self {
        Self {
            inner: RustEvent::tool_error(tool_name, error_msg),
        }
    }

    /// Create an agent start event.
    #[staticmethod]
    fn agent_start(agent_id: &str) -> Self {
        Self {
            inner: RustEvent::agent_start(agent_id),
        }
    }

    /// Create an agent end event.
    #[staticmethod]
    fn agent_end(agent_id: &str, result: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: RustEvent::agent_end(agent_id, py_to_json(result)?),
        })
    }

    /// Create a custom event.
    #[staticmethod]
    fn custom(name: &str, data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: RustEvent::custom(name, py_to_json(data)?),
        })
    }

    /// Get the event type.
    #[getter]
    fn event_type(&self) -> EventType {
        EventType {
            inner: self.inner.event_type,
        }
    }

    /// Get the event data.
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.inner.data {
            RustEventData::None => Ok(py.None()),
            RustEventData::Json(v) => json_to_py(py, v),
            RustEventData::String(s) => Ok(s.clone().into_py(py)),
            RustEventData::Map(m) => {
                let dict = PyDict::new_bound(py);
                for (k, v) in m {
                    dict.set_item(k, json_to_py(py, v)?)?;
                }
                Ok(dict.into())
            }
        }
    }

    /// Get the event timestamp (milliseconds since epoch).
    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    /// Get the correlation ID if set.
    #[getter]
    fn correlation_id(&self) -> Option<String> {
        self.inner.correlation_id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Event(type={}, timestamp={})",
            self.inner.event_type, self.inner.timestamp
        )
    }
}

/// An event bus for publishing and subscribing to events.
///
/// Example:
///     ```python
///     from liteforge import EventBus, Event, EventType
///
///     bus = EventBus()
///
///     # Subscribe to all events
///     sub = bus.subscribe_all()
///
///     # Publish an event
///     bus.publish(Event.tool_call("calc", "{}"))
///
///     # Try to receive
///     event = sub.try_recv()
///     ```
#[pyclass]
pub struct EventBus {
    inner: RustEventBus,
}

#[pymethods]
impl EventBus {
    /// Create a new event bus.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustEventBus::new(),
        }
    }

    /// Create an event bus with custom capacity.
    #[staticmethod]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: RustEventBus::with_capacity(capacity),
        }
    }

    /// Publish an event to all subscribers.
    fn publish(&self, event: Event) {
        self.inner.publish(event.inner);
    }

    /// Publish multiple events.
    fn publish_batch(&self, events: Vec<Event>) {
        let rust_events: Vec<RustEvent> = events.into_iter().map(|e| e.inner).collect();
        self.inner.publish_batch(rust_events);
    }

    /// Get the number of subscribers.
    fn subscriber_count(&self) -> usize {
        self.inner.subscriber_count()
    }
}

// ============================================================================
// Hooks
// ============================================================================

/// Hook event type enumeration.
#[pyclass]
#[derive(Clone)]
pub struct HookEvent {
    inner: RustHookEvent,
}

#[pymethods]
impl HookEvent {
    #[staticmethod]
    fn before_agent_start() -> Self {
        Self {
            inner: RustHookEvent::BeforeAgentStart,
        }
    }

    #[staticmethod]
    fn after_agent_end() -> Self {
        Self {
            inner: RustHookEvent::AfterAgentEnd,
        }
    }

    #[staticmethod]
    fn before_agent_step() -> Self {
        Self {
            inner: RustHookEvent::BeforeAgentStep,
        }
    }

    #[staticmethod]
    fn after_agent_step() -> Self {
        Self {
            inner: RustHookEvent::AfterAgentStep,
        }
    }

    #[staticmethod]
    fn before_tool_call() -> Self {
        Self {
            inner: RustHookEvent::BeforeToolCall,
        }
    }

    #[staticmethod]
    fn after_tool_call() -> Self {
        Self {
            inner: RustHookEvent::AfterToolCall,
        }
    }

    #[staticmethod]
    fn before_llm_request() -> Self {
        Self {
            inner: RustHookEvent::BeforeLlmRequest,
        }
    }

    #[staticmethod]
    fn after_llm_response() -> Self {
        Self {
            inner: RustHookEvent::AfterLlmResponse,
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Context passed to hooks.
#[pyclass]
#[derive(Clone)]
pub struct HookContext {
    inner: RustHookContext,
}

#[pymethods]
impl HookContext {
    /// Create a new hook context.
    #[new]
    fn new(event: HookEvent) -> Self {
        Self {
            inner: RustHookContext::new(event.inner),
        }
    }

    /// Create a context for a tool call.
    #[staticmethod]
    fn tool_call(tool_name: &str, arguments: &str) -> Self {
        Self {
            inner: RustHookContext::tool_call(tool_name, arguments),
        }
    }

    /// Create a context for an agent start.
    #[staticmethod]
    fn agent_start(agent_id: &str) -> Self {
        Self {
            inner: RustHookContext::agent_start(agent_id),
        }
    }

    /// Get the event type.
    #[getter]
    fn event(&self) -> HookEvent {
        HookEvent {
            inner: self.inner.event,
        }
    }

    /// Get data as a Python object.
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        json_to_py(py, &self.inner.data)
    }

    /// Get a string value from the data.
    fn get_str(&self, key: &str) -> Option<String> {
        self.inner.get_str(key).map(|s| s.to_string())
    }

    /// Get the correlation ID if set.
    #[getter]
    fn correlation_id(&self) -> Option<String> {
        self.inner.correlation_id.clone()
    }

    fn __repr__(&self) -> String {
        format!("HookContext(event={})", self.inner.event)
    }
}

/// Result from a hook execution.
#[pyclass]
#[derive(Clone)]
pub struct HookResult {
    inner: RustHookResult,
}

#[pymethods]
impl HookResult {
    /// Create a Continue result (allow operation to proceed).
    #[staticmethod]
    fn continue_() -> Self {
        Self {
            inner: RustHookResult::Continue,
        }
    }

    /// Create a ContinueWith result (modify data and proceed).
    #[staticmethod]
    fn continue_with(data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: RustHookResult::ContinueWith(py_to_json(data)?),
        })
    }

    /// Create a Skip result (skip the operation).
    #[staticmethod]
    fn skip() -> Self {
        Self {
            inner: RustHookResult::Skip,
        }
    }

    /// Create a SkipWith result (skip and return this value).
    #[staticmethod]
    fn skip_with(data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: RustHookResult::SkipWith(py_to_json(data)?),
        })
    }

    /// Create an Abort result (abort with error).
    #[staticmethod]
    fn abort(message: String) -> Self {
        Self {
            inner: RustHookResult::Abort(message),
        }
    }

    /// Check if this result allows continuation.
    fn should_continue(&self) -> bool {
        self.inner.should_continue()
    }

    /// Check if this result skips the operation.
    fn should_skip(&self) -> bool {
        self.inner.should_skip()
    }

    /// Check if this result aborts.
    fn is_abort(&self) -> bool {
        self.inner.is_abort()
    }

    /// Get the error message if abort.
    fn error_message(&self) -> Option<String> {
        self.inner.error_message().map(|s| s.to_string())
    }

    fn __repr__(&self) -> String {
        if self.inner.should_continue() {
            "HookResult::Continue".to_string()
        } else if self.inner.should_skip() {
            "HookResult::Skip".to_string()
        } else {
            format!(
                "HookResult::Abort({})",
                self.inner.error_message().unwrap_or("")
            )
        }
    }
}

/// A Python-based hook implementation (internal, not exposed to Python).
struct PyHookImpl {
    name: String,
    priority: i32,
    callback: PyObject,
}

impl RustHook for PyHookImpl {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn on_event(&self, ctx: &RustHookContext) -> RustHookResult {
        Python::with_gil(|py| {
            let py_ctx = HookContext { inner: ctx.clone() };
            match self.callback.call1(py, (py_ctx,)) {
                Ok(result) => {
                    // Try to extract HookResult from Python return value
                    if let Ok(hook_result) = result.extract::<HookResult>(py) {
                        hook_result.inner
                    } else {
                        // If not a HookResult, treat as Continue
                        RustHookResult::Continue
                    }
                }
                Err(e) => {
                    // Log error and continue
                    eprintln!("Hook '{}' error: {}", self.name, e);
                    RustHookResult::Continue
                }
            }
        })
    }
}

/// Manages a collection of hooks and executes them in order.
///
/// Example:
///     ```python
///     from liteforge import HookManager, HookContext, HookResult
///
///     def my_hook(ctx):
///         print(f"Tool called: {ctx.get_str('tool')}")
///         return HookResult.continue_()
///
///     manager = HookManager()
///     manager.register("logging", my_hook)  # priority defaults to 100
///     manager.register("blocker", blocker_hook, priority=50)  # lower priority runs first
///
///     ctx = HookContext.tool_call("calc", "{}")
///     result = manager.run(ctx)
///     ```
#[pyclass]
pub struct HookManager {
    /// Public so other classes (e.g. `ToolCallingAgent::new`) can clone the
    /// `Arc` and pass it to `RustToolCallingAgent::with_hooks(...)`. The
    /// underlying `RustHookManager` is not `Clone` (RwLock<Vec<Box<dyn Hook>>>),
    /// so we share by Arc.
    pub inner: std::sync::Arc<RustHookManager>,
}

#[pymethods]
impl HookManager {
    /// Create a new empty hook manager.
    #[new]
    fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(RustHookManager::new()),
        }
    }

    /// Register a hook.
    ///
    /// Args:
    ///     name: Hook name for identification
    ///     callback: Python callable that receives HookContext and returns HookResult
    ///     priority: Priority (lower runs first, default 100)
    #[pyo3(signature = (name, callback, priority=100))]
    fn register(&self, name: String, callback: PyObject, priority: i32) {
        let hook = PyHookImpl {
            name,
            priority,
            callback,
        };
        self.inner.register(Box::new(hook));
    }

    /// Unregister a hook by name.
    fn unregister(&self, name: &str) -> bool {
        self.inner.unregister(name)
    }

    /// Run all hooks for an event.
    fn run(&self, ctx: HookContext) -> HookResult {
        HookResult {
            inner: self.inner.run(&ctx.inner),
        }
    }

    /// Get the number of registered hooks.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Check if no hooks are registered.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the names of all registered hooks.
    fn hook_names(&self) -> Vec<String> {
        self.inner.hook_names()
    }

    /// Clear all hooks.
    fn clear(&self) {
        self.inner.clear();
    }

    fn __repr__(&self) -> String {
        format!("HookManager(hooks={})", self.inner.len())
    }
}

// Agents
// ============================================================================

use liteforge_core::agents::{
    Agent, AgentConfig as RustAgentConfig, AgentMemory as RustAgentMemory,
    AgentState as RustAgentState, AgentStep as RustAgentStep, StepResult as RustStepResult,
    StepType as RustStepType, ToolCallingAgent as RustToolCallingAgent,
};

/// Agent state enumeration.
#[pyclass]
#[derive(Clone)]
pub struct AgentState {
    inner: RustAgentState,
}

#[pymethods]
impl AgentState {
    #[staticmethod]
    fn idle() -> Self {
        Self {
            inner: RustAgentState::Idle,
        }
    }

    #[staticmethod]
    fn thinking() -> Self {
        Self {
            inner: RustAgentState::Thinking,
        }
    }

    #[staticmethod]
    fn executing_tool() -> Self {
        Self {
            inner: RustAgentState::ExecutingTool,
        }
    }

    #[staticmethod]
    fn waiting_for_human() -> Self {
        Self {
            inner: RustAgentState::WaitingForHuman,
        }
    }

    #[staticmethod]
    fn completed() -> Self {
        Self {
            inner: RustAgentState::Completed,
        }
    }

    #[staticmethod]
    fn error() -> Self {
        Self {
            inner: RustAgentState::Error,
        }
    }

    #[staticmethod]
    fn stopped() -> Self {
        Self {
            inner: RustAgentState::Stopped,
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __eq__(&self, other: &AgentState) -> bool {
        self.inner == other.inner
    }
}

/// Agent memory for storing conversation history and facts.
#[pyclass]
#[derive(Clone)]
pub struct AgentMemory {
    inner: RustAgentMemory,
}

#[pymethods]
impl AgentMemory {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustAgentMemory::new(),
        }
    }

    /// Add a message to short-term memory.
    fn add_message(&mut self, role: &str, content: &str) {
        let msg = match role {
            "user" => RustMessage::user(content),
            "assistant" => RustMessage::assistant(content),
            "system" => RustMessage::system(content),
            _ => RustMessage::user(content), // Default to user
        };
        self.inner.add_message(msg);
    }

    /// Get all messages in short-term memory.
    fn messages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for msg in self.inner.messages() {
            let dict = PyDict::new_bound(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", msg.content.as_deref().unwrap_or(""))?;
            list.append(dict)?;
        }
        Ok(list.into())
    }

    /// Clear short-term memory.
    fn clear_short_term(&mut self) {
        self.inner.clear_short_term();
    }

    /// Store a value in long-term memory.
    fn remember(&mut self, key: &str, value: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        self.inner.remember(key, py_to_json(value)?);
        Ok(())
    }

    /// Retrieve a value from long-term memory.
    fn recall(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        match self.inner.recall(key) {
            Some(v) => json_to_py(py, v),
            None => Ok(py.None()),
        }
    }

    /// Forget a value from long-term memory.
    fn forget(&mut self, key: &str) {
        self.inner.forget(key);
    }

    /// Get message count.
    fn message_count(&self) -> usize {
        self.inner.message_count()
    }

    fn __repr__(&self) -> String {
        format!("AgentMemory(messages={})", self.inner.message_count())
    }
}

/// Configuration for an agent.
#[pyclass]
#[derive(Clone)]
pub struct AgentConfig {
    inner: RustAgentConfig,
}

#[pymethods]
impl AgentConfig {
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: RustAgentConfig::new(name),
        }
    }

    /// Set the system prompt (returns new config).
    fn with_system_prompt(&self, prompt: &str) -> Self {
        Self {
            inner: self.inner.clone().with_system_prompt(prompt),
        }
    }

    /// Set the model (returns new config).
    fn with_model(&self, model: &str) -> Self {
        Self {
            inner: self.inner.clone().with_model(model),
        }
    }

    /// Set the maximum steps (returns new config).
    fn with_max_steps(&self, max_steps: usize) -> Self {
        Self {
            inner: self.inner.clone().with_max_steps(max_steps),
        }
    }

    /// Set the temperature (returns new config).
    fn with_temperature(&self, temperature: f32) -> Self {
        Self {
            inner: self.inner.clone().with_temperature(temperature),
        }
    }

    /// Set the max tokens (returns new config).
    fn with_max_tokens(&self, max_tokens: u32) -> Self {
        Self {
            inner: self.inner.clone().with_max_tokens(max_tokens),
        }
    }

    /// Add a tool (returns new config).
    fn with_tool(&self, tool_name: &str) -> Self {
        Self {
            inner: self.inner.clone().with_tool(tool_name),
        }
    }

    /// Get the name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the system prompt.
    #[getter]
    fn system_prompt(&self) -> Option<String> {
        self.inner.system_prompt.clone()
    }

    /// Get the model.
    #[getter]
    fn model(&self) -> Option<String> {
        self.inner.model.clone()
    }

    /// Get the max steps.
    #[getter]
    fn max_steps(&self) -> usize {
        self.inner.max_steps
    }

    /// Get the tools.
    #[getter]
    fn tools(&self) -> Vec<String> {
        self.inner.tools.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "AgentConfig(name='{}', max_steps={})",
            self.inner.name, self.inner.max_steps
        )
    }
}

/// Step type enumeration.
#[pyclass]
#[derive(Clone)]
pub struct StepType {
    inner: RustStepType,
}

#[pymethods]
impl StepType {
    fn is_think(&self) -> bool {
        matches!(self.inner, RustStepType::Think)
    }

    fn is_tool_call(&self) -> bool {
        matches!(self.inner, RustStepType::ToolCall { .. })
    }

    fn is_tool_result(&self) -> bool {
        matches!(self.inner, RustStepType::ToolResult { .. })
    }

    fn is_response(&self) -> bool {
        matches!(self.inner, RustStepType::Response)
    }

    fn is_error(&self) -> bool {
        matches!(self.inner, RustStepType::Error)
    }

    /// Get the tool name if this is a tool call or result.
    fn tool_name(&self) -> Option<String> {
        match &self.inner {
            RustStepType::ToolCall { tool_name, .. } => Some(tool_name.clone()),
            RustStepType::ToolResult { tool_name, .. } => Some(tool_name.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Result of a single agent step.
#[pyclass]
#[derive(Clone)]
pub struct StepResult {
    inner: RustStepResult,
}

#[pymethods]
impl StepResult {
    fn is_done(&self) -> bool {
        self.inner.is_done()
    }

    fn has_tool_calls(&self) -> bool {
        self.inner.has_tool_calls()
    }

    fn is_error(&self) -> bool {
        self.inner.is_error()
    }

    /// Get the response text if this is a Done result.
    fn response(&self) -> Option<String> {
        self.inner.response().map(|s| s.to_string())
    }

    /// Get the error message if this is an Error result.
    fn error_message(&self) -> Option<String> {
        self.inner.error_message().map(|s| s.to_string())
    }

    fn __repr__(&self) -> String {
        if self.inner.is_done() {
            "StepResult(Done)".to_string()
        } else if self.inner.has_tool_calls() {
            "StepResult(ToolCalls)".to_string()
        } else if self.inner.is_error() {
            "StepResult(Error)".to_string()
        } else {
            "StepResult(Continue)".to_string()
        }
    }
}

/// A single step in agent execution.
#[pyclass]
#[derive(Clone)]
pub struct AgentStep {
    inner: RustAgentStep,
}

#[pymethods]
impl AgentStep {
    #[getter]
    fn step_number(&self) -> usize {
        self.inner.step_number
    }

    #[getter]
    fn step_type(&self) -> StepType {
        StepType {
            inner: self.inner.step_type.clone(),
        }
    }

    #[getter]
    fn result(&self) -> StepResult {
        StepResult {
            inner: self.inner.result.clone(),
        }
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration.as_millis() as u64
    }

    #[getter]
    fn prompt_tokens(&self) -> Option<u32> {
        self.inner.tokens.map(|t| t.prompt_tokens)
    }

    #[getter]
    fn completion_tokens(&self) -> Option<u32> {
        self.inner.tokens.map(|t| t.completion_tokens)
    }

    #[getter]
    fn total_tokens(&self) -> Option<u32> {
        self.inner.tokens.map(|t| t.total_tokens)
    }

    fn __repr__(&self) -> String {
        format!(
            "AgentStep(step={}, type={}, duration_ms={})",
            self.inner.step_number,
            self.inner.step_type,
            self.inner.duration.as_millis()
        )
    }
}

/// A tool-calling agent that can execute multi-step tasks.
///
/// Example:
///     ```python
///     from liteforge import ToolCallingAgent, AsyncForgeClient, ToolRegistry, AgentConfig
///
///     client = AsyncForgeClient()
///     tools = ToolRegistry()
///     config = AgentConfig("my-agent").with_system_prompt("You are helpful")
///
///     agent = ToolCallingAgent(client, tools, config)
///     result = await agent.run("What is 2 + 2?")
///     print(result)
///     ```
#[pyclass]
pub struct ToolCallingAgent {
    inner: std::sync::Arc<tokio::sync::Mutex<RustToolCallingAgent>>,
}

#[pymethods]
impl ToolCallingAgent {
    /// Create a new tool-calling agent.
    #[new]
    #[pyo3(signature = (client, tools, config=None, system_prompt=None, max_steps=None, hooks=None, preserve_history=None))]
    fn new(
        client: &AsyncForgeClient,
        tools: &ToolRegistry,
        config: Option<&AgentConfig>,
        system_prompt: Option<&str>,
        max_steps: Option<usize>,
        hooks: Option<&HookManager>,
        preserve_history: Option<bool>,
    ) -> Self {
        let mut agent = RustToolCallingAgent::new(client.inner.clone(), tools.inner.clone());

        if let Some(cfg) = config {
            agent = agent.with_config(cfg.inner.clone());
        }

        if let Some(prompt) = system_prompt {
            agent = agent.with_system_prompt(prompt);
        }

        if let Some(steps) = max_steps {
            agent = agent.with_max_steps(steps);
        }

        if let Some(h) = hooks {
            agent = agent.with_hooks(h.inner.clone());
        }

        if let Some(preserve) = preserve_history {
            agent = agent.with_preserve_history(preserve);
        }

        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(agent)),
        }
    }

    /// Get the agent's name.
    fn name(&self) -> String {
        // Use try_lock for sync access or block
        futures::executor::block_on(async { self.inner.lock().await.name().to_string() })
    }

    /// Get the agent's configuration.
    fn config(&self) -> AgentConfig {
        futures::executor::block_on(async {
            AgentConfig {
                inner: self.inner.lock().await.config().clone(),
            }
        })
    }

    /// Get the current state.
    fn state(&self) -> AgentState {
        futures::executor::block_on(async {
            AgentState {
                inner: self.inner.lock().await.context().state,
            }
        })
    }

    /// Get the execution history.
    fn history(&self) -> Vec<AgentStep> {
        futures::executor::block_on(async {
            self.inner
                .lock()
                .await
                .history()
                .iter()
                .map(|s| AgentStep { inner: s.clone() })
                .collect()
        })
    }

    /// Clear execution history.
    fn clear_history(&self) {
        futures::executor::block_on(async {
            self.inner.lock().await.clear_history();
        })
    }

    /// Run the agent with a user message (async).
    fn run<'py>(&self, py: Python<'py>, input: &str) -> PyResult<Bound<'py, PyAny>> {
        let input = input.to_string();
        let agent = self.inner.clone();

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = agent.lock().await;
            let result = guard
                .run(&input)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(result)
        })
    }

    /// Stop the agent.
    fn stop(&self) {
        futures::executor::block_on(async {
            self.inner.lock().await.stop();
        })
    }

    /// Reset the agent for a new task.
    fn reset(&self) {
        futures::executor::block_on(async {
            self.inner.lock().await.reset();
        })
    }

    /// Set whether to preserve conversation history across resets.
    ///
    /// When enabled, calling `reset()` or finishing a `run()` will not clear
    /// the short-term memory (conversation history). This enables multi-turn
    /// conversations where the agent retains context between `run()` calls.
    fn set_preserve_history(&self, preserve: bool) {
        futures::executor::block_on(async {
            self.inner.lock().await.set_preserve_history(preserve);
        })
    }

    /// Get the current conversation messages from the agent's memory.
    ///
    /// Returns a list of message dicts with 'role' and 'content' keys.
    /// This can be used to persist conversation state externally.
    fn conversation_messages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let messages = futures::executor::block_on(async {
            use liteforge_core::agents::Agent;
            self.inner.lock().await.conversation_messages()
        });

        let list = pyo3::types::PyList::empty_bound(py);
        for msg in messages {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", &msg.content)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }

    /// Load prior conversation history into the agent's memory.
    ///
    /// Args:
    ///     history: List of message dicts with 'role' and 'content' keys.
    ///              Roles should be 'user', 'assistant', or 'system'.
    fn load_history(&self, history: &Bound<'_, pyo3::types::PyList>) -> PyResult<()> {
        let messages = list_to_messages(history)?;
        futures::executor::block_on(async {
            use liteforge_core::agents::Agent;
            self.inner.lock().await.load_history(messages);
        });
        Ok(())
    }

    /// Run the agent with prior conversation history (async).
    ///
    /// This enables multi-turn conversations by injecting prior messages
    /// into the agent's memory before processing the current input.
    ///
    /// Args:
    ///     input: The current user message.
    ///     history: List of message dicts with 'role' and 'content' keys.
    #[pyo3(signature = (input, history))]
    fn run_with_history<'py>(
        &self,
        py: Python<'py>,
        input: &str,
        history: &Bound<'_, pyo3::types::PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let input = input.to_string();
        let messages = list_to_messages(history)?;
        let agent = self.inner.clone();

        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            use liteforge_core::agents::Agent;
            let mut guard = agent.lock().await;
            let result = guard
                .run_with_history(&input, messages)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(result)
        })
    }

    fn __repr__(&self) -> String {
        futures::executor::block_on(async {
            let guard = self.inner.lock().await;
            format!(
                "ToolCallingAgent(name='{}', state={})",
                guard.name(),
                guard.context().state
            )
        })
    }
}

// ============================================================================
// Orchestration Bindings
// ============================================================================

/// Intent classification result.
///
/// Represents a classified user intent with confidence score.
#[pyclass]
#[derive(Clone)]
pub struct Intent {
    inner: RustIntent,
}

#[pymethods]
impl Intent {
    /// Create a new intent.
    #[new]
    #[pyo3(signature = (name, input))]
    fn new(name: &str, input: &str) -> Self {
        Self {
            inner: RustIntent::new(name, input),
        }
    }

    /// Set confidence level.
    fn confidence(&self, confidence: f32) -> Self {
        Self {
            inner: self.inner.clone().confidence(confidence),
        }
    }

    /// Add metadata.
    fn metadata(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .metadata(key, serde_json::Value::String(value.to_string())),
        }
    }

    /// Get the intent name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the input text.
    #[getter]
    fn input(&self) -> String {
        self.inner.input.clone()
    }

    /// Get the confidence score.
    #[getter]
    fn confidence_score(&self) -> f32 {
        self.inner.confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "Intent(name='{}', confidence={})",
            self.inner.name, self.inner.confidence
        )
    }
}

/// Route definition for intent routing.
#[pyclass]
#[derive(Clone)]
pub struct IntentRoute {
    inner: RustIntentRoute,
}

#[pymethods]
impl IntentRoute {
    /// Create a new intent route for an agent.
    #[new]
    fn new(agent: &str) -> Self {
        Self {
            inner: RustIntentRoute::new(agent),
        }
    }

    /// Add a pattern to match.
    fn pattern(&self, pattern: &str) -> Self {
        Self {
            inner: self.inner.clone().pattern(pattern),
        }
    }

    /// Add multiple patterns.
    fn patterns(&self, patterns: Vec<String>) -> Self {
        Self {
            inner: self.inner.clone().patterns(patterns),
        }
    }

    /// Set priority (higher = more important).
    fn priority(&self, priority: i32) -> Self {
        Self {
            inner: self.inner.clone().priority(priority),
        }
    }

    /// Add a keyword that boosts confidence.
    fn keyword(&self, keyword: &str) -> Self {
        Self {
            inner: self.inner.clone().keyword(keyword),
        }
    }

    /// Add multiple keywords.
    fn keywords(&self, keywords: Vec<String>) -> Self {
        Self {
            inner: self.inner.clone().keywords(keywords),
        }
    }

    /// Get the agent name.
    #[getter]
    fn agent(&self) -> String {
        self.inner.agent.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "IntentRoute(agent='{}', patterns={:?})",
            self.inner.agent, self.inner.patterns
        )
    }
}

/// Common intent patterns builder.
#[pyclass]
pub struct CommonIntents;

#[pymethods]
impl CommonIntents {
    /// Create a route for greeting intents.
    #[staticmethod]
    fn greeting(agent: &str) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::greeting(agent),
        }
    }

    /// Create a route for question/help intents.
    #[staticmethod]
    fn question(agent: &str) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::question(agent),
        }
    }

    /// Create a route for code-related intents.
    #[staticmethod]
    fn code(agent: &str) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::code(agent),
        }
    }

    /// Create a route for search intents.
    #[staticmethod]
    fn search(agent: &str) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::search(agent),
        }
    }

    /// Create a route for task/action intents.
    #[staticmethod]
    fn task(agent: &str) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::task(agent),
        }
    }
}

/// Intent router for directing inputs to agents.
#[pyclass]
#[derive(Clone)]
pub struct IntentRouter {
    inner: RustIntentRouter,
}

#[pymethods]
impl IntentRouter {
    /// Create a new intent router.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustIntentRouter::new(),
        }
    }

    /// Add a route.
    fn route(&self, route: IntentRoute) -> Self {
        Self {
            inner: self.inner.clone().route(route.inner),
        }
    }

    /// Set the default agent for unmatched intents.
    fn default_agent(&self, agent: &str) -> Self {
        Self {
            inner: self.inner.clone().default_agent(agent),
        }
    }

    /// Set minimum confidence threshold.
    fn min_confidence(&self, confidence: f32) -> Self {
        Self {
            inner: self.inner.clone().min_confidence(confidence),
        }
    }

    /// Classify input and route to an agent.
    fn classify_and_route(&self, input: &str) -> Option<RoutingDecision> {
        self.inner
            .classify_and_route(input)
            .map(|d| RoutingDecision { inner: d })
    }

    fn __repr__(&self) -> String {
        "IntentRouter()".to_string()
    }
}

/// Routing decision result.
#[pyclass]
#[derive(Clone)]
pub struct RoutingDecision {
    inner: RustRoutingDecision,
}

#[pymethods]
impl RoutingDecision {
    /// Get the target agent.
    #[getter]
    fn agent(&self) -> String {
        self.inner.agent.clone()
    }

    /// Get the confidence score.
    #[getter]
    fn confidence(&self) -> f32 {
        self.inner.confidence
    }

    /// Get alternative agents.
    #[getter]
    fn alternatives(&self) -> Vec<(String, f32)> {
        self.inner.alternatives.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "RoutingDecision(agent='{}', confidence={})",
            self.inner.agent, self.inner.confidence
        )
    }
}

/// Session message in a conversation.
#[pyclass]
#[derive(Clone)]
pub struct SessionMessage {
    inner: RustSessionMessage,
}

#[pymethods]
impl SessionMessage {
    /// Create a user message.
    #[staticmethod]
    fn user(content: &str) -> Self {
        Self {
            inner: RustSessionMessage::user(content),
        }
    }

    /// Create an assistant message.
    #[staticmethod]
    #[pyo3(signature = (content, agent=None))]
    fn assistant(content: &str, agent: Option<String>) -> Self {
        Self {
            inner: RustSessionMessage::assistant(content, agent),
        }
    }

    /// Create a system message.
    #[staticmethod]
    fn system(content: &str) -> Self {
        Self {
            inner: RustSessionMessage::system(content),
        }
    }

    /// Get the role.
    #[getter]
    fn role(&self) -> String {
        self.inner.role.clone()
    }

    /// Get the content.
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    /// Get the agent name (if any).
    #[getter]
    fn agent(&self) -> Option<String> {
        self.inner.agent.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionMessage(role='{}', content='{}')",
            self.inner.role,
            if self.inner.content.len() > 50 {
                format!("{}...", &self.inner.content[..50])
            } else {
                self.inner.content.clone()
            }
        )
    }
}

/// Session for maintaining conversation state.
#[pyclass]
#[derive(Clone)]
pub struct Session {
    inner: RustSession,
}

#[pymethods]
impl Session {
    /// Create a new session.
    #[new]
    fn new(id: &str) -> Self {
        Self {
            inner: RustSession::new(id),
        }
    }

    /// Get the session ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get conversation history.
    #[getter]
    fn history(&self) -> Vec<SessionMessage> {
        self.inner
            .history
            .iter()
            .map(|m| SessionMessage { inner: m.clone() })
            .collect()
    }

    /// Add a message to history.
    fn add_message(&mut self, message: SessionMessage) {
        self.inner.add_message(message.inner);
    }

    /// Clear conversation history.
    fn clear_history(&mut self) {
        self.inner.history.clear();
    }

    /// Check if session is expired.
    fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }

    fn __repr__(&self) -> String {
        format!(
            "Session(id='{}', messages={})",
            self.inner.id,
            self.inner.history.len()
        )
    }
}

/// Session store for managing multiple sessions.
#[pyclass]
pub struct SessionStore {
    inner: RustSessionStore,
}

#[pymethods]
impl SessionStore {
    /// Create a new session store.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustSessionStore::new(),
        }
    }

    /// Set default TTL in seconds.
    fn with_default_ttl_secs(&self, secs: i64) -> Self {
        Self {
            inner: self.inner.clone().with_default_ttl_secs(secs),
        }
    }

    /// Create a new session.
    fn create<'py>(&self, py: Python<'py>, id: &str) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let id = id.to_string();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let session = inner.create(&id).await;
            Ok(Session { inner: session })
        })
    }

    /// Get a session by ID.
    fn get<'py>(&self, py: Python<'py>, id: &str) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let id = id.to_string();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let session = inner.get(&id).await;
            Ok(session.map(|s| Session { inner: s }))
        })
    }

    /// Update a session.
    fn update<'py>(&self, py: Python<'py>, session: Session) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            inner.update(session.inner).await;
            Ok(())
        })
    }

    /// Remove a session.
    fn remove<'py>(&self, py: Python<'py>, id: &str) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let id = id.to_string();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let removed = inner.remove(&id).await;
            Ok(removed.map(|s| Session { inner: s }))
        })
    }

    /// Get session count.
    fn count<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move { Ok(inner.count().await) })
    }

    /// Cleanup expired sessions.
    fn cleanup_expired<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(
            py,
            async move { Ok(inner.cleanup_expired().await) },
        )
    }

    fn __repr__(&self) -> String {
        "SessionStore()".to_string()
    }
}

/// Workflow step definition.
#[pyclass]
#[derive(Clone)]
pub struct WorkflowStep {
    inner: RustWorkflowStep,
}

#[pymethods]
impl WorkflowStep {
    /// Create a new workflow step.
    #[new]
    fn new(id: &str, name: &str, agent: &str) -> Self {
        Self {
            inner: RustWorkflowStep::new(id, name, agent),
        }
    }

    /// Set input data.
    fn input(&self, py: Python<'_>, input: PyObject) -> PyResult<Self> {
        let json_str = input.call_method0(py, "__str__")?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str.extract::<String>(py)?)
            .unwrap_or(serde_json::Value::Null);
        Ok(Self {
            inner: self.inner.clone().input(json_value),
        })
    }

    /// Add a dependency on another step.
    fn depends_on(&self, step_id: &str) -> Self {
        Self {
            inner: self.inner.clone().depends_on(step_id),
        }
    }

    /// Set a condition for execution.
    fn condition(&self, condition: &str) -> Self {
        Self {
            inner: self.inner.clone().condition(condition),
        }
    }

    /// Get the step ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the step name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the agent name.
    #[getter]
    fn agent(&self) -> String {
        self.inner.agent.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "WorkflowStep(id='{}', name='{}', agent='{}')",
            self.inner.id, self.inner.name, self.inner.agent
        )
    }
}

/// Workflow definition.
#[pyclass]
#[derive(Clone)]
pub struct Workflow {
    inner: RustWorkflow,
}

#[pymethods]
impl Workflow {
    /// Create a new workflow.
    #[new]
    fn new(id: &str, name: &str) -> Self {
        Self {
            inner: RustWorkflow::new(id, name),
        }
    }

    /// Add a step to the workflow.
    fn step(&self, step: WorkflowStep) -> Self {
        Self {
            inner: self.inner.clone().step(step.inner),
        }
    }

    /// Set timeout in seconds.
    fn timeout_secs(&self, secs: u64) -> Self {
        Self {
            inner: self.inner.clone().timeout_secs(secs),
        }
    }

    /// Set maximum parallel executions.
    fn max_parallel(&self, max: usize) -> Self {
        Self {
            inner: self.inner.clone().max_parallel(max),
        }
    }

    /// Get the workflow ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the workflow name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the steps.
    #[getter]
    fn steps(&self) -> Vec<WorkflowStep> {
        self.inner
            .steps
            .iter()
            .map(|s| WorkflowStep { inner: s.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Workflow(id='{}', name='{}', steps={})",
            self.inner.id,
            self.inner.name,
            self.inner.steps.len()
        )
    }
}

/// Step execution status.
#[pyclass]
#[derive(Clone)]
pub struct OrchestrationStepStatus {
    inner: RustStepStatus,
}

#[pymethods]
impl OrchestrationStepStatus {
    /// Pending status.
    #[staticmethod]
    fn pending() -> Self {
        Self {
            inner: RustStepStatus::Pending,
        }
    }

    /// Running status.
    #[staticmethod]
    fn running() -> Self {
        Self {
            inner: RustStepStatus::Running,
        }
    }

    /// Completed status.
    #[staticmethod]
    fn completed() -> Self {
        Self {
            inner: RustStepStatus::Completed,
        }
    }

    /// Failed status.
    #[staticmethod]
    fn failed() -> Self {
        Self {
            inner: RustStepStatus::Failed,
        }
    }

    /// Skipped status.
    #[staticmethod]
    fn skipped() -> Self {
        Self {
            inner: RustStepStatus::Skipped,
        }
    }

    fn __repr__(&self) -> String {
        format!("StepStatus::{:?}", self.inner)
    }
}

/// Orchestration strategy.
#[pyclass]
#[derive(Clone)]
pub struct OrchestrationStrategy {
    inner: RustOrchestrationStrategy,
}

#[pymethods]
impl OrchestrationStrategy {
    /// Intent-based routing.
    #[staticmethod]
    fn intent_based() -> Self {
        Self {
            inner: RustOrchestrationStrategy::IntentBased,
        }
    }

    /// Round-robin agent selection.
    #[staticmethod]
    fn round_robin() -> Self {
        Self {
            inner: RustOrchestrationStrategy::RoundRobin,
        }
    }

    /// Fixed agent selection.
    #[staticmethod]
    fn fixed() -> Self {
        Self {
            inner: RustOrchestrationStrategy::Fixed,
        }
    }

    /// Adaptive selection based on confidence.
    #[staticmethod]
    fn adaptive() -> Self {
        Self {
            inner: RustOrchestrationStrategy::Adaptive,
        }
    }

    fn __repr__(&self) -> String {
        format!("OrchestrationStrategy::{:?}", self.inner)
    }
}

/// Orchestrator configuration.
#[pyclass]
#[derive(Clone)]
pub struct OrchestratorConfig {
    inner: RustOrchestratorConfig,
}

#[pymethods]
impl OrchestratorConfig {
    /// Create a new orchestrator config.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustOrchestratorConfig::new(),
        }
    }

    /// Set the orchestration strategy.
    fn strategy(&self, strategy: OrchestrationStrategy) -> Self {
        Self {
            inner: self.inner.clone().strategy(strategy.inner),
        }
    }

    /// Set the default agent.
    fn default_agent(&self, agent: &str) -> Self {
        Self {
            inner: self.inner.clone().default_agent(agent),
        }
    }

    /// Set minimum routing confidence.
    fn min_routing_confidence(&self, confidence: f32) -> Self {
        Self {
            inner: self.inner.clone().min_routing_confidence(confidence),
        }
    }

    /// Set session TTL in seconds.
    fn session_ttl_secs(&self, secs: i64) -> Self {
        Self {
            inner: self.inner.clone().session_ttl_secs(secs),
        }
    }

    /// Enable or disable conversation history.
    fn maintain_history(&self, maintain: bool) -> Self {
        Self {
            inner: self.inner.clone().maintain_history(maintain),
        }
    }

    fn __repr__(&self) -> String {
        format!("OrchestratorConfig(strategy={:?})", self.inner.strategy)
    }
}

/// Result of an orchestration request.
#[pyclass]
#[derive(Clone)]
pub struct OrchestrationResult {
    inner: RustOrchestrationResult,
}

#[pymethods]
impl OrchestrationResult {
    /// Get the agent that handled the request.
    #[getter]
    fn agent(&self) -> String {
        self.inner.agent.clone()
    }

    /// Get the response message.
    #[getter]
    fn response(&self) -> String {
        self.inner.response.clone()
    }

    /// Get the session ID.
    #[getter]
    fn session_id(&self) -> String {
        self.inner.session_id.clone()
    }

    /// Get the routing decision (if any).
    #[getter]
    fn routing(&self) -> Option<RoutingDecision> {
        self.inner
            .routing
            .clone()
            .map(|r| RoutingDecision { inner: r })
    }

    fn __repr__(&self) -> String {
        format!(
            "OrchestrationResult(agent='{}', session_id='{}')",
            self.inner.agent, self.inner.session_id
        )
    }
}

// ============================================================================
// MCP Bindings
// ============================================================================

/// Transport type for MCP server communication.
#[pyclass]
#[derive(Clone)]
pub struct TransportType {
    inner: RustTransportType,
}

#[pymethods]
impl TransportType {
    /// Create a stdio transport type.
    #[staticmethod]
    fn stdio() -> Self {
        Self {
            inner: RustTransportType::Stdio,
        }
    }

    /// Create an SSE transport type.
    #[staticmethod]
    fn sse() -> Self {
        Self {
            inner: RustTransportType::Sse,
        }
    }

    /// Create an HTTP transport type.
    #[staticmethod]
    fn http() -> Self {
        Self {
            inner: RustTransportType::Http,
        }
    }

    fn __repr__(&self) -> String {
        format!("TransportType({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// MCP server configuration.
#[pyclass]
#[derive(Clone)]
pub struct McpServerConfig {
    inner: RustMcpServerConfig,
}

#[pymethods]
impl McpServerConfig {
    /// Create a new stdio server configuration.
    #[staticmethod]
    fn stdio(name: &str, command: &str) -> Self {
        Self {
            inner: RustMcpServerConfig::stdio(name, command),
        }
    }

    /// Create a new SSE server configuration.
    #[staticmethod]
    fn sse(name: &str, url: &str) -> Self {
        Self {
            inner: RustMcpServerConfig::sse(name, url),
        }
    }

    /// Create a new HTTP server configuration.
    #[staticmethod]
    fn http(name: &str, url: &str) -> Self {
        Self {
            inner: RustMcpServerConfig::http(name, url),
        }
    }

    /// Add a command argument.
    fn with_arg(&self, arg: &str) -> Self {
        Self {
            inner: self.inner.clone().with_arg(arg),
        }
    }

    /// Add multiple command arguments.
    fn with_args(&self, args: Vec<String>) -> Self {
        Self {
            inner: self.inner.clone().with_args(args),
        }
    }

    /// Set an environment variable.
    fn with_env_var(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self.inner.clone().with_env_var(key, value),
        }
    }

    /// Set bearer token authentication.
    fn with_bearer_token(&self, token: &str) -> Self {
        Self {
            inner: self.inner.clone().with_bearer_token(token),
        }
    }

    /// Set timeout in seconds.
    fn with_timeout_secs(&self, secs: u64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .with_timeout(std::time::Duration::from_secs(secs)),
        }
    }

    /// Get the server name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the transport type.
    #[getter]
    fn transport(&self) -> TransportType {
        TransportType {
            inner: self.inner.transport.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "McpServerConfig(name='{}', transport={})",
            self.inner.name, self.inner.transport
        )
    }
}

/// MCP configuration for multiple servers.
#[pyclass]
#[derive(Clone)]
pub struct McpConfig {
    inner: RustMcpConfig,
}

#[pymethods]
impl McpConfig {
    /// Create a new empty configuration.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustMcpConfig::new(),
        }
    }

    /// Add a server configuration.
    fn with_server(&self, config: &McpServerConfig) -> Self {
        Self {
            inner: self.inner.clone().with_server(config.inner.clone()),
        }
    }

    /// Get all server names.
    fn server_names(&self) -> Vec<String> {
        self.inner
            .server_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get a server configuration by name.
    fn get_server(&self, name: &str) -> Option<McpServerConfig> {
        self.inner
            .get_server(name)
            .map(|c| McpServerConfig { inner: c.clone() })
    }

    fn __repr__(&self) -> String {
        format!("McpConfig(servers={:?})", self.server_names())
    }
}

/// MCP server connection state.
#[pyclass]
#[derive(Clone)]
pub struct McpServerState {
    inner: RustServerState,
}

#[pymethods]
impl McpServerState {
    /// Check if disconnected.
    #[getter]
    fn is_disconnected(&self) -> bool {
        self.inner == RustServerState::Disconnected
    }

    /// Check if connecting.
    #[getter]
    fn is_connecting(&self) -> bool {
        self.inner == RustServerState::Connecting
    }

    /// Check if connected.
    #[getter]
    fn is_connected(&self) -> bool {
        self.inner == RustServerState::Connected
    }

    /// Check if failed.
    #[getter]
    fn is_failed(&self) -> bool {
        self.inner == RustServerState::Failed
    }

    fn __repr__(&self) -> String {
        format!("McpServerState({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// MCP stdio server connection.
#[pyclass]
pub struct McpStdioServer {
    inner: Arc<tokio::sync::Mutex<RustMcpStdioServer>>,
}

#[pymethods]
impl McpStdioServer {
    /// Create a new stdio server connection.
    #[new]
    fn new(config: &McpServerConfig) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(RustMcpStdioServer::new(
                config.inner.clone(),
            ))),
        }
    }

    /// Get the server name.
    #[getter]
    fn name(&self) -> String {
        futures::executor::block_on(async { self.inner.lock().await.name().to_string() })
    }

    /// Get the connection state.
    #[getter]
    fn state(&self) -> McpServerState {
        futures::executor::block_on(async {
            McpServerState {
                inner: self.inner.lock().await.state(),
            }
        })
    }

    /// Connect to the server.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .connect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// Disconnect from the server.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .disconnect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// List available tools.
    fn list_tools<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            let result = guard
                .list_tools()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let tools: Vec<String> = result.tools.iter().map(|t| t.name.clone()).collect();
            Ok(tools)
        })
    }

    fn __repr__(&self) -> String {
        futures::executor::block_on(async {
            let guard = self.inner.lock().await;
            format!(
                "McpStdioServer(name='{}', state={})",
                guard.name(),
                guard.state()
            )
        })
    }
}

/// MCP HTTP server connection.
#[pyclass]
pub struct McpHttpServer {
    inner: Arc<tokio::sync::Mutex<RustMcpHttpServer>>,
}

#[pymethods]
impl McpHttpServer {
    /// Create a new HTTP server connection.
    #[new]
    fn new(config: &McpServerConfig) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(RustMcpHttpServer::new(
                config.inner.clone(),
            ))),
        }
    }

    /// Get the server name.
    #[getter]
    fn name(&self) -> String {
        futures::executor::block_on(async { self.inner.lock().await.name().to_string() })
    }

    /// Get the connection state.
    #[getter]
    fn state(&self) -> McpServerState {
        futures::executor::block_on(async {
            McpServerState {
                inner: self.inner.lock().await.state(),
            }
        })
    }

    /// Connect to the server.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .connect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// Disconnect from the server.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .disconnect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// List available tools.
    fn list_tools<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            let result = guard
                .list_tools()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let tools: Vec<String> = result.tools.iter().map(|t| t.name.clone()).collect();
            Ok(tools)
        })
    }

    fn __repr__(&self) -> String {
        futures::executor::block_on(async {
            let guard = self.inner.lock().await;
            format!(
                "McpHttpServer(name='{}', state={})",
                guard.name(),
                guard.state()
            )
        })
    }
}

/// MCP SSE server connection.
#[pyclass]
pub struct McpSseServer {
    inner: Arc<tokio::sync::Mutex<RustMcpSseServer>>,
}

#[pymethods]
impl McpSseServer {
    /// Create a new SSE server connection.
    #[new]
    fn new(config: &McpServerConfig) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(RustMcpSseServer::new(
                config.inner.clone(),
            ))),
        }
    }

    /// Get the server name.
    #[getter]
    fn name(&self) -> String {
        futures::executor::block_on(async { self.inner.lock().await.name().to_string() })
    }

    /// Get the connection state.
    #[getter]
    fn state(&self) -> McpServerState {
        futures::executor::block_on(async {
            McpServerState {
                inner: self.inner.lock().await.state(),
            }
        })
    }

    /// Connect to the server.
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .connect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// Disconnect from the server.
    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            guard
                .disconnect()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    /// List available tools.
    fn list_tools<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let guard = inner.lock().await;
            let result = guard
                .list_tools()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let tools: Vec<String> = result.tools.iter().map(|t| t.name.clone()).collect();
            Ok(tools)
        })
    }

    fn __repr__(&self) -> String {
        futures::executor::block_on(async {
            let guard = self.inner.lock().await;
            format!(
                "McpSseServer(name='{}', state={})",
                guard.name(),
                guard.state()
            )
        })
    }
}

// =============================================================================
// OBSERVABILITY
// =============================================================================

/// Span kind indicating the type of operation.
#[pyclass]
#[derive(Clone)]
pub struct SpanKind {
    inner: RustSpanKind,
}

#[pymethods]
impl SpanKind {
    /// Internal operation.
    #[staticmethod]
    fn internal() -> Self {
        Self {
            inner: RustSpanKind::Internal,
        }
    }

    /// Server operation (handling incoming request).
    #[staticmethod]
    fn server() -> Self {
        Self {
            inner: RustSpanKind::Server,
        }
    }

    /// Client operation (making outgoing request).
    #[staticmethod]
    fn client() -> Self {
        Self {
            inner: RustSpanKind::Client,
        }
    }

    /// Producer operation (sending message).
    #[staticmethod]
    fn producer() -> Self {
        Self {
            inner: RustSpanKind::Producer,
        }
    }

    /// Consumer operation (receiving message).
    #[staticmethod]
    fn consumer() -> Self {
        Self {
            inner: RustSpanKind::Consumer,
        }
    }

    fn __repr__(&self) -> String {
        format!("SpanKind({:?})", self.inner)
    }
}

/// Span status indicating success or failure.
#[pyclass]
#[derive(Clone)]
pub struct SpanStatus {
    inner: RustSpanStatus,
}

#[pymethods]
impl SpanStatus {
    /// Unset status.
    #[staticmethod]
    fn unset() -> Self {
        Self {
            inner: RustSpanStatus::Unset,
        }
    }

    /// Successful operation.
    #[staticmethod]
    fn ok() -> Self {
        Self {
            inner: RustSpanStatus::Ok,
        }
    }

    /// Error with message.
    #[staticmethod]
    fn error(message: &str) -> Self {
        Self {
            inner: RustSpanStatus::Error(message.to_string()),
        }
    }

    fn __repr__(&self) -> String {
        format!("SpanStatus({:?})", self.inner)
    }
}

/// Span context for distributed tracing.
#[pyclass]
#[derive(Clone)]
pub struct SpanContext {
    inner: RustSpanContext,
}

#[pymethods]
impl SpanContext {
    /// Get the trace ID.
    #[getter]
    fn trace_id(&self) -> String {
        self.inner.trace_id.clone()
    }

    /// Get the span ID.
    #[getter]
    fn span_id(&self) -> String {
        self.inner.span_id.clone()
    }

    /// Get the parent span ID.
    #[getter]
    fn parent_span_id(&self) -> Option<String> {
        self.inner.parent_span_id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SpanContext(trace_id='{}', span_id='{}')",
            self.inner.trace_id, self.inner.span_id
        )
    }
}

/// A tracing span for observability.
#[pyclass]
#[derive(Clone)]
pub struct Span {
    inner: RustSpan,
}

#[pymethods]
impl Span {
    /// Get the span name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the span context.
    #[getter]
    fn context(&self) -> SpanContext {
        SpanContext {
            inner: self.inner.context.clone(),
        }
    }

    /// Get the span kind.
    #[getter]
    fn kind(&self) -> SpanKind {
        SpanKind {
            inner: self.inner.kind,
        }
    }

    /// Get the span status.
    #[getter]
    fn status(&self) -> SpanStatus {
        SpanStatus {
            inner: self.inner.status.clone(),
        }
    }

    /// Get the start time (Unix timestamp microseconds).
    #[getter]
    fn start_time_us(&self) -> u64 {
        self.inner.start_time_us
    }

    /// Get the end time (Unix timestamp microseconds).
    #[getter]
    fn end_time_us(&self) -> Option<u64> {
        self.inner.end_time_us
    }

    /// Set the status.
    fn set_status(&mut self, status: &SpanStatus) {
        self.inner.status = status.inner.clone();
    }

    /// Set an attribute.
    fn set_attribute(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_val: JsonValue = if let Ok(s) = value.extract::<String>() {
            JsonValue::String(s)
        } else if let Ok(i) = value.extract::<i64>() {
            JsonValue::Number(i.into())
        } else if let Ok(f) = value.extract::<f64>() {
            serde_json::Number::from_f64(f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        } else if let Ok(b) = value.extract::<bool>() {
            JsonValue::Bool(b)
        } else {
            JsonValue::Null
        };
        self.inner.set_attribute(key, json_val);
        Ok(())
    }

    /// End the span.
    fn end(&mut self) {
        self.inner.end();
    }

    fn __repr__(&self) -> String {
        format!(
            "Span(name='{}', status={:?})",
            self.inner.name, self.inner.status
        )
    }
}

/// Tracer for creating and managing spans.
#[pyclass]
pub struct Tracer {
    inner: Arc<std::sync::Mutex<RustTracer>>,
}

#[pymethods]
impl Tracer {
    /// Create a new tracer.
    #[new]
    #[pyo3(signature = (service_name="default"))]
    fn new(service_name: &str) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(RustTracer::new(service_name))),
        }
    }

    /// Start a new span.
    fn start_span(&self, name: &str) -> Span {
        let guard = self.inner.lock().unwrap();
        Span {
            inner: guard.start_span(name).start(),
        }
    }

    /// Get recorded spans count.
    fn span_count(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.span_count()
    }

    /// Get all recorded spans.
    fn spans(&self) -> Vec<Span> {
        let guard = self.inner.lock().unwrap();
        guard
            .spans()
            .into_iter()
            .map(|s| Span { inner: s })
            .collect()
    }

    /// Record a completed span.
    fn record_span(&self, span: &Span) {
        let guard = self.inner.lock().unwrap();
        guard.record_span(span.inner.clone());
    }

    /// Clear all recorded spans.
    fn clear_spans(&self) {
        let guard = self.inner.lock().unwrap();
        guard.clear_spans();
    }

    fn __repr__(&self) -> String {
        let guard = self.inner.lock().unwrap();
        format!(
            "Tracer(service='{}', spans={})",
            guard.service_name(),
            guard.span_count()
        )
    }
}

/// Metric value types.
#[pyclass]
#[derive(Clone)]
pub struct MetricValue {
    inner: RustMetricValue,
}

#[pymethods]
impl MetricValue {
    /// Create a counter value.
    #[staticmethod]
    fn counter(value: u64) -> Self {
        Self {
            inner: RustMetricValue::Counter(value),
        }
    }

    /// Create a gauge value.
    #[staticmethod]
    fn gauge(value: f64) -> Self {
        Self {
            inner: RustMetricValue::Gauge(value),
        }
    }

    /// Create a histogram value from recorded values.
    #[staticmethod]
    fn histogram(values: Vec<f64>) -> Self {
        let mut hist = RustHistogramData::new();
        for v in values {
            hist.record(v);
        }
        Self {
            inner: RustMetricValue::Histogram(hist),
        }
    }

    fn __repr__(&self) -> String {
        format!("MetricValue({:?})", self.inner)
    }
}

/// Metrics collector for gathering application metrics.
#[pyclass]
pub struct MetricsCollector {
    inner: Arc<std::sync::Mutex<RustMetricsCollector>>,
}

#[pymethods]
impl MetricsCollector {
    /// Create a new metrics collector.
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(RustMetricsCollector::new())),
        }
    }

    /// Increment a counter.
    fn increment(&self, name: &str, value: u64) {
        let guard = self.inner.lock().unwrap();
        guard.increment(name, value);
    }

    /// Set a gauge value.
    fn gauge(&self, name: &str, value: f64) {
        let guard = self.inner.lock().unwrap();
        guard.gauge(name, value);
    }

    /// Record a histogram observation.
    fn record_histogram(&self, name: &str, value: f64) {
        let guard = self.inner.lock().unwrap();
        guard.record_histogram(name, value);
    }

    /// Get a counter value.
    fn get_counter(&self, name: &str) -> Option<u64> {
        let guard = self.inner.lock().unwrap();
        guard.snapshot().counter(name)
    }

    /// Get a gauge value.
    fn get_gauge(&self, name: &str) -> Option<f64> {
        let guard = self.inner.lock().unwrap();
        guard.snapshot().gauge(name)
    }

    fn __repr__(&self) -> String {
        "MetricsCollector()".to_string()
    }
}

// =============================================================================
// CONVERSATION MANAGEMENT
// =============================================================================

/// Summarization strategy for compacting conversations.
#[pyclass]
#[derive(Clone)]
pub struct SummarizationStrategy {
    inner: RustSummarizationStrategy,
}

#[pymethods]
impl SummarizationStrategy {
    /// Keep only recent messages.
    #[staticmethod]
    fn keep_recent() -> Self {
        Self {
            inner: RustSummarizationStrategy::KeepRecent,
        }
    }

    /// Summarize older messages.
    #[staticmethod]
    fn summarize() -> Self {
        Self {
            inner: RustSummarizationStrategy::Summarize,
        }
    }

    /// Summarize in chunks with multiple levels.
    #[staticmethod]
    fn chunked_summary() -> Self {
        Self {
            inner: RustSummarizationStrategy::ChunkedSummary,
        }
    }

    /// Preserve system messages, summarize user/assistant.
    #[staticmethod]
    fn preserve_system() -> Self {
        Self {
            inner: RustSummarizationStrategy::PreserveSystem,
        }
    }

    fn __repr__(&self) -> String {
        format!("SummarizationStrategy({:?})", self.inner)
    }
}

/// Configuration for conversation management.
#[pyclass]
#[derive(Clone)]
pub struct ConversationConfig {
    inner: RustConversationConfig,
}

#[pymethods]
impl ConversationConfig {
    /// Create a new conversation config with default settings.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustConversationConfig::default(),
        }
    }

    /// Get max tokens.
    #[getter]
    fn max_tokens(&self) -> usize {
        self.inner.max_tokens
    }

    /// Set max tokens.
    #[setter]
    fn set_max_tokens(&mut self, value: usize) {
        self.inner.max_tokens = value;
    }

    /// Get target tokens.
    #[getter]
    fn target_tokens(&self) -> usize {
        self.inner.target_tokens
    }

    /// Set target tokens.
    #[setter]
    fn set_target_tokens(&mut self, value: usize) {
        self.inner.target_tokens = value;
    }

    /// Get preserve recent count.
    #[getter]
    fn preserve_recent(&self) -> usize {
        self.inner.preserve_recent
    }

    /// Set preserve recent count.
    #[setter]
    fn set_preserve_recent(&mut self, value: usize) {
        self.inner.preserve_recent = value;
    }

    /// Get strategy.
    #[getter]
    fn strategy(&self) -> SummarizationStrategy {
        SummarizationStrategy {
            inner: self.inner.strategy,
        }
    }

    /// Set strategy.
    #[setter]
    fn set_strategy(&mut self, strategy: &SummarizationStrategy) {
        self.inner.strategy = strategy.inner;
    }

    fn __repr__(&self) -> String {
        format!(
            "ConversationConfig(max_tokens={}, target_tokens={})",
            self.inner.max_tokens, self.inner.target_tokens
        )
    }
}

/// Managed conversation with token tracking.
#[pyclass]
pub struct ManagedConversation {
    inner: RustManagedConversation,
}

#[pymethods]
impl ManagedConversation {
    /// Create a new managed conversation.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustManagedConversation::new(),
        }
    }

    /// Set the system message.
    fn set_system(&mut self, content: &str) {
        self.inner.set_system(content);
    }

    /// Add a user message.
    fn add_user_message(&mut self, content: &str) {
        self.inner.add_user_message(content);
    }

    /// Add an assistant message.
    fn add_assistant_message(&mut self, content: &str) {
        self.inner.add_assistant_message(content);
    }

    /// Get all messages as list of dicts.
    fn messages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let messages = self.inner.messages();
        let list = PyList::empty_bound(py);
        for msg in messages {
            let dict = PyDict::new_bound(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", &msg.content)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }

    /// Get estimated token count.
    fn estimated_tokens(&self) -> usize {
        self.inner.estimated_tokens()
    }

    /// Get message count (excluding system).
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all messages.
    #[pyo3(signature = (keep_system=false))]
    fn clear(&mut self, keep_system: bool) {
        self.inner.clear(keep_system);
    }

    fn __repr__(&self) -> String {
        format!(
            "ManagedConversation(messages={}, tokens=~{})",
            self.inner.len(),
            self.inner.estimated_tokens()
        )
    }
}

/// Compacting conversation with auto-summarization.
#[pyclass]
pub struct CompactingConversation {
    inner: RustCompactingConversation,
}

#[pymethods]
impl CompactingConversation {
    /// Create a new compacting conversation.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<&ConversationConfig>) -> Self {
        let inner = match config {
            Some(c) => RustCompactingConversation::new(c.inner.clone()),
            None => RustCompactingConversation::with_defaults(),
        };
        Self { inner }
    }

    /// Add a user message.
    fn add_user(&mut self, content: &str) {
        self.inner.add_user_message(content);
    }

    /// Add an assistant message.
    fn add_assistant(&mut self, content: &str) {
        self.inner.add_assistant_message(content);
    }

    /// Get messages.
    fn messages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let messages = self.inner.messages();
        let list = PyList::empty_bound(py);
        for msg in messages {
            let dict = PyDict::new_bound(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", &msg.content)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }

    /// Get estimated token count.
    fn token_count(&self) -> usize {
        self.inner.estimated_tokens()
    }

    /// Check if compaction is needed.
    fn needs_compaction(&self) -> bool {
        self.inner.needs_compaction()
    }

    /// Compact using simple drop-oldest strategy.
    fn compact(&mut self) {
        self.inner.compact();
    }

    fn __repr__(&self) -> String {
        format!(
            "CompactingConversation(tokens=~{}, needs_compaction={})",
            self.inner.estimated_tokens(),
            self.inner.needs_compaction()
        )
    }
}

// =============================================================================
// HUMAN-IN-THE-LOOP (HITL)
// =============================================================================

/// Risk level for approval requests.
#[pyclass]
#[derive(Clone)]
pub struct RiskLevel {
    inner: RustRiskLevel,
}

#[pymethods]
impl RiskLevel {
    /// Low risk.
    #[staticmethod]
    fn low() -> Self {
        Self {
            inner: RustRiskLevel::Low,
        }
    }

    /// Medium risk.
    #[staticmethod]
    fn medium() -> Self {
        Self {
            inner: RustRiskLevel::Medium,
        }
    }

    /// High risk.
    #[staticmethod]
    fn high() -> Self {
        Self {
            inner: RustRiskLevel::High,
        }
    }

    /// Critical risk.
    #[staticmethod]
    fn critical() -> Self {
        Self {
            inner: RustRiskLevel::Critical,
        }
    }

    fn __repr__(&self) -> String {
        format!("RiskLevel({:?})", self.inner)
    }
}

/// Approval status.
#[pyclass]
#[derive(Clone)]
pub struct ApprovalStatus {
    inner: RustApprovalStatus,
}

#[pymethods]
impl ApprovalStatus {
    /// Pending approval.
    #[staticmethod]
    fn pending() -> Self {
        Self {
            inner: RustApprovalStatus::Pending,
        }
    }

    /// Approved.
    #[staticmethod]
    fn approved() -> Self {
        Self {
            inner: RustApprovalStatus::Approved,
        }
    }

    /// Denied.
    #[staticmethod]
    fn denied() -> Self {
        Self {
            inner: RustApprovalStatus::Denied,
        }
    }

    /// Timed out.
    #[staticmethod]
    fn timed_out() -> Self {
        Self {
            inner: RustApprovalStatus::TimedOut,
        }
    }

    /// Check if approved.
    fn is_approved(&self) -> bool {
        matches!(self.inner, RustApprovalStatus::Approved)
    }

    /// Check if denied.
    fn is_denied(&self) -> bool {
        matches!(self.inner, RustApprovalStatus::Denied)
    }

    fn __repr__(&self) -> String {
        format!("ApprovalStatus({:?})", self.inner)
    }
}

/// Approval request for human review.
#[pyclass]
#[derive(Clone)]
pub struct ApprovalRequest {
    inner: RustApprovalRequest,
}

#[pymethods]
impl ApprovalRequest {
    /// Create a new approval request.
    #[new]
    #[pyo3(signature = (operation, description=None))]
    fn new(operation: &str, description: Option<&str>) -> Self {
        let mut inner = RustApprovalRequest::new(operation);
        if let Some(desc) = description {
            inner = inner.description(desc);
        }
        Self { inner }
    }

    /// Set the description.
    fn with_description(&self, description: &str) -> Self {
        Self {
            inner: self.inner.clone().description(description),
        }
    }

    /// Set the risk level.
    fn with_risk_level(&self, level: &RiskLevel) -> Self {
        Self {
            inner: self.inner.clone().risk_level(level.inner),
        }
    }

    /// Add context.
    fn with_context(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self.inner.clone().context(key, value),
        }
    }

    /// Get the operation.
    #[getter]
    fn operation(&self) -> String {
        self.inner.operation.clone()
    }

    /// Get the description.
    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Get the risk level.
    #[getter]
    fn risk_level(&self) -> RiskLevel {
        RiskLevel {
            inner: self.inner.risk_level,
        }
    }

    /// Get the request ID.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ApprovalRequest(id='{}', operation='{}', risk={:?})",
            self.inner.id, self.inner.operation, self.inner.risk_level
        )
    }
}

/// Result of an approval request.
#[pyclass]
#[derive(Clone)]
pub struct ApprovalResult {
    inner: RustApprovalResult,
}

#[pymethods]
impl ApprovalResult {
    /// Create an approved result.
    #[staticmethod]
    fn approved(request_id: &str) -> Self {
        Self {
            inner: RustApprovalResult::approved(request_id),
        }
    }

    /// Create a denied result.
    #[staticmethod]
    fn denied(request_id: &str, reason: &str) -> Self {
        Self {
            inner: RustApprovalResult::denied(request_id, reason),
        }
    }

    /// Create a timed out result.
    #[staticmethod]
    fn timed_out(request_id: &str) -> Self {
        Self {
            inner: RustApprovalResult::timed_out(request_id),
        }
    }

    /// Get the status.
    #[getter]
    fn status(&self) -> ApprovalStatus {
        ApprovalStatus {
            inner: self.inner.status.clone(),
        }
    }

    /// Get the request ID.
    #[getter]
    fn request_id(&self) -> String {
        self.inner.request_id.clone()
    }

    /// Check if approved.
    fn is_approved(&self) -> bool {
        self.inner.approved
    }

    /// Get the reason (if denied).
    #[getter]
    fn reason(&self) -> Option<String> {
        self.inner.reason.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ApprovalResult(request_id='{}', status={:?})",
            self.inner.request_id, self.inner.status
        )
    }
}

// =============================================================================
// EVALS
// =============================================================================

/// Test case for evaluation.
#[pyclass]
#[derive(Clone)]
pub struct TestCase {
    inner: RustTestCase,
}

#[pymethods]
impl TestCase {
    /// Create a new test case.
    #[new]
    fn new(input: &str, expected: &str) -> Self {
        Self {
            inner: RustTestCase::new(input, expected),
        }
    }

    /// Add a tag.
    fn with_tag(&self, tag: &str) -> Self {
        Self {
            inner: self.inner.clone().tag(tag),
        }
    }

    /// Set the weight.
    fn with_weight(&self, weight: f64) -> Self {
        Self {
            inner: self.inner.clone().weight(weight),
        }
    }

    /// Add metadata.
    fn with_metadata(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self.inner.clone().metadata(key, value),
        }
    }

    /// Get the input.
    #[getter]
    fn input(&self) -> String {
        self.inner.input.clone()
    }

    /// Get the expected output.
    #[getter]
    fn expected(&self) -> String {
        self.inner.expected.clone()
    }

    /// Get the weight.
    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    /// Get tags.
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "TestCase(input='{}...', expected='{}...')",
            self.inner.input.chars().take(20).collect::<String>(),
            self.inner.expected.chars().take(20).collect::<String>()
        )
    }
}

/// Result of a single evaluation.
#[pyclass]
#[derive(Clone)]
pub struct EvalResult {
    inner: RustEvalResult,
}

#[pymethods]
impl EvalResult {
    /// Create a passed result (score = 1.0).
    #[staticmethod]
    fn pass_result() -> Self {
        Self {
            inner: RustEvalResult::pass(),
        }
    }

    /// Create a failed result (score = 0.0).
    #[staticmethod]
    fn fail_result(reason: &str) -> Self {
        Self {
            inner: RustEvalResult::fail(reason),
        }
    }

    /// Create a partial result with a custom score.
    #[staticmethod]
    fn partial(score: f64, reason: &str) -> Self {
        Self {
            inner: RustEvalResult::partial(score, reason),
        }
    }

    /// Check if passed.
    fn is_passed(&self) -> bool {
        self.inner.passed
    }

    /// Get the score.
    #[getter]
    fn score(&self) -> f64 {
        self.inner.score
    }

    /// Get the reason (if failed).
    #[getter]
    fn reason(&self) -> Option<String> {
        self.inner.reason.clone()
    }

    fn __repr__(&self) -> String {
        if self.inner.passed {
            format!("EvalResult(passed, score={})", self.inner.score)
        } else {
            format!(
                "EvalResult(failed, score={}, reason='{}')",
                self.inner.score,
                self.inner.reason.as_deref().unwrap_or("")
            )
        }
    }
}

/// Suite statistics.
#[pyclass]
#[derive(Clone)]
pub struct SuiteStats {
    inner: RustSuiteStats,
}

#[pymethods]
impl SuiteStats {
    /// Total test cases.
    #[getter]
    fn total(&self) -> usize {
        self.inner.total
    }

    /// Passed test cases.
    #[getter]
    fn passed(&self) -> usize {
        self.inner.passed
    }

    /// Failed test cases.
    #[getter]
    fn failed(&self) -> usize {
        self.inner.failed
    }

    /// Average score.
    #[getter]
    fn avg_score(&self) -> f64 {
        self.inner.avg_score
    }

    /// Duration in milliseconds.
    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    /// Pass rate (0.0 - 1.0).
    fn pass_rate(&self) -> f64 {
        self.inner.pass_rate
    }

    fn __repr__(&self) -> String {
        format!(
            "SuiteStats(passed={}/{}, avg_score={:.2}, duration={}ms)",
            self.inner.passed, self.inner.total, self.inner.avg_score, self.inner.duration_ms
        )
    }
}

/// Result of running an eval suite.
#[pyclass]
#[derive(Clone)]
pub struct SuiteResult {
    inner: RustSuiteResult,
}

#[pymethods]
impl SuiteResult {
    /// Get suite statistics.
    #[getter]
    fn stats(&self) -> SuiteStats {
        SuiteStats {
            inner: self.inner.stats.clone(),
        }
    }

    /// Check if all tests passed.
    fn all_passed(&self) -> bool {
        self.inner.stats.passed == self.inner.stats.total
    }

    fn __repr__(&self) -> String {
        let all_passed = self.inner.stats.passed == self.inner.stats.total;
        format!(
            "SuiteResult(passed={}/{}, all_passed={})",
            self.inner.stats.passed, self.inner.stats.total, all_passed
        )
    }
}

// =============================================================================
// SCHEDULER
// =============================================================================

/// Schedule type.
#[pyclass]
#[derive(Clone)]
pub struct ScheduleType {
    inner: RustScheduleType,
}

#[pymethods]
impl ScheduleType {
    /// One-time schedule.
    #[staticmethod]
    fn once() -> Self {
        Self {
            inner: RustScheduleType::Once,
        }
    }

    /// Interval-based schedule.
    #[staticmethod]
    fn interval() -> Self {
        Self {
            inner: RustScheduleType::Interval,
        }
    }

    /// Cron-based schedule.
    #[staticmethod]
    fn cron() -> Self {
        Self {
            inner: RustScheduleType::Cron,
        }
    }

    fn __repr__(&self) -> String {
        format!("ScheduleType({:?})", self.inner)
    }
}

/// Job status.
#[pyclass]
#[derive(Clone)]
pub struct JobStatus {
    inner: RustJobStatus,
}

#[pymethods]
impl JobStatus {
    /// Pending status.
    #[staticmethod]
    fn pending() -> Self {
        Self {
            inner: RustJobStatus::Pending,
        }
    }

    /// Running status.
    #[staticmethod]
    fn running() -> Self {
        Self {
            inner: RustJobStatus::Running,
        }
    }

    /// Completed status.
    #[staticmethod]
    fn completed() -> Self {
        Self {
            inner: RustJobStatus::Completed,
        }
    }

    /// Failed status.
    #[staticmethod]
    fn failed() -> Self {
        Self {
            inner: RustJobStatus::Failed,
        }
    }

    /// Cancelled status.
    #[staticmethod]
    fn cancelled() -> Self {
        Self {
            inner: RustJobStatus::Cancelled,
        }
    }

    /// Paused status.
    #[staticmethod]
    fn paused() -> Self {
        Self {
            inner: RustJobStatus::Paused,
        }
    }

    fn __repr__(&self) -> String {
        format!("JobStatus({:?})", self.inner)
    }
}

/// One-time schedule.
#[pyclass]
#[derive(Clone)]
pub struct OnceSchedule {
    inner: RustOnceSchedule,
}

#[pymethods]
impl OnceSchedule {
    /// Create a schedule that runs immediately.
    #[staticmethod]
    fn now() -> Self {
        Self {
            inner: RustOnceSchedule::now(),
        }
    }

    /// Create a schedule that runs at a specific timestamp (ms).
    #[staticmethod]
    fn at(timestamp_ms: u64) -> Self {
        Self {
            inner: RustOnceSchedule::at(timestamp_ms),
        }
    }

    /// Create a schedule that runs after a delay (seconds).
    #[staticmethod]
    fn after_secs(secs: u64) -> Self {
        Self {
            inner: RustOnceSchedule::after(std::time::Duration::from_secs(secs)),
        }
    }

    /// Get next run time.
    fn next_run(&self) -> Option<u64> {
        self.inner.next_run()
    }

    /// Check if should run now.
    fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    /// Check if exhausted.
    fn is_exhausted(&self) -> bool {
        self.inner.is_exhausted()
    }

    fn __repr__(&self) -> String {
        format!("OnceSchedule(next_run={:?})", self.inner.next_run())
    }
}

/// Interval-based schedule.
#[pyclass]
#[derive(Clone)]
pub struct IntervalSchedule {
    inner: RustIntervalSchedule,
}

#[pymethods]
impl IntervalSchedule {
    /// Create from seconds.
    #[staticmethod]
    fn from_secs(secs: u64) -> Self {
        Self {
            inner: RustIntervalSchedule::from_secs(secs),
        }
    }

    /// Create from minutes.
    #[staticmethod]
    fn from_mins(mins: u64) -> Self {
        Self {
            inner: RustIntervalSchedule::from_mins(mins),
        }
    }

    /// Create from hours.
    #[staticmethod]
    fn from_hours(hours: u64) -> Self {
        Self {
            inner: RustIntervalSchedule::from_hours(hours),
        }
    }

    /// Set initial delay.
    fn with_initial_delay_secs(&self, secs: u64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .with_initial_delay(std::time::Duration::from_secs(secs)),
        }
    }

    /// Set maximum runs.
    fn with_max_runs(&self, max: u64) -> Self {
        Self {
            inner: self.inner.clone().with_max_runs(max),
        }
    }

    /// Get next run time.
    fn next_run(&self) -> Option<u64> {
        self.inner.next_run()
    }

    /// Check if should run now.
    fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    /// Get run count.
    fn run_count(&self) -> u64 {
        self.inner.run_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "IntervalSchedule(interval={:?}, run_count={})",
            self.inner.interval(),
            self.inner.run_count()
        )
    }
}

/// Cron-based schedule.
#[pyclass]
#[derive(Clone)]
pub struct CronSchedule {
    inner: RustCronSchedule,
}

#[pymethods]
impl CronSchedule {
    /// Create from cron expression.
    #[new]
    fn new(expression: &str) -> Self {
        Self {
            inner: RustCronSchedule::new(expression),
        }
    }

    /// Every minute.
    #[staticmethod]
    fn every_minute() -> Self {
        Self {
            inner: RustCronSchedule::every_minute(),
        }
    }

    /// Every hour.
    #[staticmethod]
    fn hourly() -> Self {
        Self {
            inner: RustCronSchedule::hourly(),
        }
    }

    /// Every day at midnight.
    #[staticmethod]
    fn daily() -> Self {
        Self {
            inner: RustCronSchedule::daily(),
        }
    }

    /// Every day at a specific hour.
    #[staticmethod]
    fn daily_at(hour: u8) -> Self {
        Self {
            inner: RustCronSchedule::daily_at(hour),
        }
    }

    /// Every week on Sunday.
    #[staticmethod]
    fn weekly() -> Self {
        Self {
            inner: RustCronSchedule::weekly(),
        }
    }

    /// Every month on the 1st.
    #[staticmethod]
    fn monthly() -> Self {
        Self {
            inner: RustCronSchedule::monthly(),
        }
    }

    /// Set maximum runs.
    fn with_max_runs(&self, max: u64) -> Self {
        Self {
            inner: self.inner.clone().with_max_runs(max),
        }
    }

    /// Get the cron expression.
    #[getter]
    fn expression(&self) -> String {
        self.inner.expression().to_string()
    }

    /// Check if expression is valid.
    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Get next run time.
    fn next_run(&self) -> Option<u64> {
        self.inner.next_run()
    }

    /// Check if should run now.
    fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    fn __repr__(&self) -> String {
        format!(
            "CronSchedule(expression='{}', valid={})",
            self.inner.expression(),
            self.inner.is_valid()
        )
    }
}

// ============================================================================
// Skills
// ============================================================================

/// Configuration for a skill.
#[pyclass]
#[derive(Clone)]
pub struct SkillConfig {
    inner: RustSkillConfig,
}

#[pymethods]
impl SkillConfig {
    /// Create a new skill config.
    #[new]
    fn new(name: String, description: String) -> Self {
        Self {
            inner: RustSkillConfig::new(name, description),
        }
    }

    /// Set the model.
    fn with_model(&self, model: String) -> Self {
        Self {
            inner: self.inner.clone().with_model(model),
        }
    }

    /// Set the system prompt.
    fn with_system_prompt(&self, prompt: String) -> Self {
        Self {
            inner: self.inner.clone().with_system_prompt(prompt),
        }
    }

    /// Set the temperature.
    fn with_temperature(&self, temperature: f32) -> Self {
        Self {
            inner: self.inner.clone().with_temperature(temperature),
        }
    }

    /// Set max tokens.
    fn with_max_tokens(&self, max_tokens: u32) -> Self {
        Self {
            inner: self.inner.clone().with_max_tokens(max_tokens),
        }
    }

    /// Add a tag.
    fn with_tag(&self, tag: String) -> Self {
        Self {
            inner: self.inner.clone().with_tag(tag),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn version(&self) -> String {
        self.inner.version.clone()
    }

    #[getter]
    fn model(&self) -> Option<String> {
        self.inner.model.clone()
    }

    #[getter]
    fn system_prompt(&self) -> Option<String> {
        self.inner.system_prompt.clone()
    }

    #[getter]
    fn temperature(&self) -> Option<f32> {
        self.inner.temperature
    }

    #[getter]
    fn max_tokens(&self) -> Option<u32> {
        self.inner.max_tokens
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SkillConfig(name='{}', description='{}')",
            self.inner.name, self.inner.description
        )
    }
}

/// Input to a skill.
#[pyclass]
#[derive(Clone)]
pub struct SkillInput {
    inner: RustSkillInput,
}

#[pymethods]
impl SkillInput {
    /// Create a new skill input.
    #[new]
    fn new(text: String) -> Self {
        Self {
            inner: RustSkillInput::new(text),
        }
    }

    /// Add a parameter.
    fn with_param(&self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(value)?;
        let mut inner = self.inner.clone();
        inner.params.insert(key, json_value);
        Ok(Self { inner })
    }

    /// Set context.
    fn with_context(&self, context: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(context)?;
        Ok(Self {
            inner: self.inner.clone().with_context(json_value),
        })
    }

    #[getter]
    fn text(&self) -> String {
        self.inner.text.clone()
    }

    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.inner.params {
            dict.set_item(k, json_to_py(py, v)?)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("SkillInput(text='{}')", truncate_str(&self.inner.text, 50))
    }
}

/// Output from a skill.
#[pyclass]
#[derive(Clone)]
pub struct SkillOutput {
    inner: RustSkillOutput,
}

#[pymethods]
impl SkillOutput {
    /// Create a new skill output.
    #[new]
    fn new(text: String) -> Self {
        Self {
            inner: RustSkillOutput::new(text),
        }
    }

    /// Set structured data.
    fn with_data(&self, data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(data)?;
        Ok(Self {
            inner: self.inner.clone().with_data(json_value),
        })
    }

    #[getter]
    fn text(&self) -> String {
        self.inner.text.clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.data {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    fn __repr__(&self) -> String {
        format!("SkillOutput(text='{}')", truncate_str(&self.inner.text, 50))
    }
}

/// A prompt-based skill.
#[pyclass]
pub struct PromptSkill {
    inner: RustPromptSkill,
}

#[pymethods]
impl PromptSkill {
    /// Create a new prompt skill.
    #[new]
    fn new(config: SkillConfig) -> Self {
        Self {
            inner: RustPromptSkill::new(config.inner),
        }
    }

    /// Create a simple prompt skill.
    #[staticmethod]
    fn simple(name: String, description: String, system_prompt: String) -> Self {
        Self {
            inner: RustPromptSkill::simple(name, description, system_prompt),
        }
    }

    /// Get the skill name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get the skill config.
    #[getter]
    fn config(&self) -> SkillConfig {
        SkillConfig {
            inner: self.inner.config().clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("PromptSkill(name='{}')", self.inner.name())
    }
}

/// Get the built-in summarize skill.
#[pyfunction]
fn get_summarize_skill() -> PromptSkill {
    PromptSkill {
        inner: summarize_skill(),
    }
}

/// Get the built-in translate skill.
#[pyfunction]
fn get_translate_skill() -> PromptSkill {
    PromptSkill {
        inner: translate_skill(),
    }
}

/// Get the built-in extract skill.
#[pyfunction]
fn get_extract_skill() -> PromptSkill {
    PromptSkill {
        inner: extract_skill(),
    }
}

/// Get the built-in rewrite skill.
#[pyfunction]
fn get_rewrite_skill() -> PromptSkill {
    PromptSkill {
        inner: rewrite_skill(),
    }
}

/// Get the built-in QA skill.
#[pyfunction]
fn get_qa_skill() -> PromptSkill {
    PromptSkill { inner: qa_skill() }
}

// ============================================================================
// Pipelines
// ============================================================================

/// Context passed through pipeline steps.
#[pyclass]
#[derive(Clone)]
pub struct PipelineContext {
    inner: RustPipelineContext,
}

#[pymethods]
impl PipelineContext {
    /// Create a new empty context.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustPipelineContext::new(),
        }
    }

    /// Set a value in the context.
    fn set(&mut self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let json_value = py_to_json(value)?;
        self.inner.set(key, json_value);
        Ok(())
    }

    /// Get a value from the context.
    fn get(&self, py: Python<'_>, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.get(key) {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    /// Get a string value from the context.
    fn get_string(&self, key: &str) -> Option<String> {
        self.inner.get_string(key).map(|s| s.to_string())
    }

    /// Set metadata.
    fn set_metadata(&mut self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let json_value = py_to_json(value)?;
        self.inner.set_metadata(key, json_value);
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineContext(data_keys={}, messages={})",
            self.inner.data.len(),
            self.inner.messages.len()
        )
    }
}

/// Output from a pipeline step.
#[pyclass]
#[derive(Clone)]
pub struct PipelineStepOutput {
    inner: RustStepOutput,
}

#[pymethods]
impl PipelineStepOutput {
    /// Create a new step output.
    #[new]
    fn new(text: String) -> Self {
        Self {
            inner: RustStepOutput::new(text),
        }
    }

    /// Set structured data.
    fn with_data(&self, data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(data)?;
        Ok(Self {
            inner: self.inner.clone().with_data(json_value),
        })
    }

    /// Mark this as a terminal step.
    fn stop(&self) -> Self {
        Self {
            inner: self.inner.clone().stop(),
        }
    }

    #[getter]
    fn text(&self) -> String {
        self.inner.text.clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.data {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    #[getter]
    fn continue_pipeline(&self) -> bool {
        self.inner.continue_pipeline
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineStepOutput(text='{}', continue={})",
            truncate_str(&self.inner.text, 50),
            self.inner.continue_pipeline
        )
    }
}

// ============================================================================
// Images
// ============================================================================

/// Image size options.
#[pyclass]
#[derive(Clone)]
pub struct ImageSize {
    inner: RustImageSize,
}

#[pymethods]
impl ImageSize {
    /// 256x256 pixels.
    #[staticmethod]
    fn size_256x256() -> Self {
        Self {
            inner: RustImageSize::Size256x256,
        }
    }

    /// 512x512 pixels.
    #[staticmethod]
    fn size_512x512() -> Self {
        Self {
            inner: RustImageSize::Size512x512,
        }
    }

    /// 1024x1024 pixels (default).
    #[staticmethod]
    fn size_1024x1024() -> Self {
        Self {
            inner: RustImageSize::Size1024x1024,
        }
    }

    /// 1792x1024 pixels (landscape, DALL-E 3 only).
    #[staticmethod]
    fn size_1792x1024() -> Self {
        Self {
            inner: RustImageSize::Size1792x1024,
        }
    }

    /// 1024x1792 pixels (portrait, DALL-E 3 only).
    #[staticmethod]
    fn size_1024x1792() -> Self {
        Self {
            inner: RustImageSize::Size1024x1792,
        }
    }

    fn __repr__(&self) -> String {
        format!("ImageSize({})", self.inner)
    }
}

/// Image quality options.
#[pyclass]
#[derive(Clone)]
pub struct ImageQuality {
    inner: RustImageQuality,
}

#[pymethods]
impl ImageQuality {
    #[staticmethod]
    fn standard() -> Self {
        Self {
            inner: RustImageQuality::Standard,
        }
    }

    #[staticmethod]
    fn hd() -> Self {
        Self {
            inner: RustImageQuality::Hd,
        }
    }

    fn __repr__(&self) -> String {
        format!("ImageQuality::{:?}", self.inner)
    }
}

/// Image style options.
#[pyclass]
#[derive(Clone)]
pub struct ImageStyle {
    inner: RustImageStyle,
}

#[pymethods]
impl ImageStyle {
    #[staticmethod]
    fn vivid() -> Self {
        Self {
            inner: RustImageStyle::Vivid,
        }
    }

    #[staticmethod]
    fn natural() -> Self {
        Self {
            inner: RustImageStyle::Natural,
        }
    }

    fn __repr__(&self) -> String {
        format!("ImageStyle::{:?}", self.inner)
    }
}

/// Image response format.
#[pyclass]
#[derive(Clone)]
pub struct ImageResponseFormat {
    inner: RustResponseFormat,
}

#[pymethods]
impl ImageResponseFormat {
    #[staticmethod]
    fn url() -> Self {
        Self {
            inner: RustResponseFormat::Url,
        }
    }

    #[staticmethod]
    fn b64_json() -> Self {
        Self {
            inner: RustResponseFormat::B64Json,
        }
    }

    fn __repr__(&self) -> String {
        format!("ImageResponseFormat::{:?}", self.inner)
    }
}

/// Image request builder.
#[pyclass]
#[derive(Clone)]
pub struct ImageRequest {
    inner: RustImageRequest,
}

#[pymethods]
impl ImageRequest {
    /// Create a new image request.
    #[new]
    fn new(prompt: String) -> Self {
        Self {
            inner: RustImageRequest::new(prompt),
        }
    }

    /// Set the model.
    fn with_model(&self, model: String) -> Self {
        Self {
            inner: self.inner.clone().model(model),
        }
    }

    /// Set the number of images to generate.
    fn with_n(&self, n: u32) -> Self {
        Self {
            inner: self.inner.clone().n(n),
        }
    }

    /// Set the image size.
    fn with_size(&self, size: ImageSize) -> Self {
        Self {
            inner: self.inner.clone().size(size.inner),
        }
    }

    /// Set the quality.
    fn with_quality(&self, quality: ImageQuality) -> Self {
        Self {
            inner: self.inner.clone().quality(quality.inner),
        }
    }

    /// Set the style.
    fn with_style(&self, style: ImageStyle) -> Self {
        Self {
            inner: self.inner.clone().style(style.inner),
        }
    }

    /// Set the response format.
    fn with_response_format(&self, format: ImageResponseFormat) -> Self {
        Self {
            inner: self.inner.clone().response_format(format.inner),
        }
    }

    /// Set the user identifier.
    fn with_user(&self, user: String) -> Self {
        Self {
            inner: self.inner.clone().user(user),
        }
    }

    #[getter]
    fn prompt(&self) -> String {
        self.inner.prompt.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageRequest(prompt='{}')",
            truncate_str(&self.inner.prompt, 50)
        )
    }
}

/// Single image data from a response.
#[pyclass]
#[derive(Clone)]
pub struct ImageData {
    inner: RustImageData,
}

#[pymethods]
impl ImageData {
    #[getter]
    fn url(&self) -> Option<String> {
        self.inner.url.clone()
    }

    #[getter]
    fn b64_json(&self) -> Option<String> {
        self.inner.b64_json.clone()
    }

    #[getter]
    fn revised_prompt(&self) -> Option<String> {
        self.inner.revised_prompt.clone()
    }

    fn __repr__(&self) -> String {
        if self.inner.url.is_some() {
            "ImageData(url=...)".to_string()
        } else {
            "ImageData(b64_json=...)".to_string()
        }
    }
}

/// Image generation response.
#[pyclass]
#[derive(Clone)]
pub struct ImageResponse {
    inner: RustImageResponse,
}

#[pymethods]
impl ImageResponse {
    #[getter]
    fn created(&self) -> u64 {
        self.inner.created
    }

    #[getter]
    fn data(&self) -> Vec<ImageData> {
        self.inner
            .data
            .iter()
            .map(|d| ImageData { inner: d.clone() })
            .collect()
    }

    /// Get the first image URL if available.
    fn first_url(&self) -> Option<String> {
        self.inner.url().map(|s| s.to_string())
    }

    /// Get all image URLs.
    fn urls(&self) -> Vec<String> {
        self.inner
            .urls()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageResponse(created={}, images={})",
            self.inner.created,
            self.inner.data.len()
        )
    }
}

// ============================================================================
// Prompts
// ============================================================================

/// A prompt template with variable substitution.
#[pyclass]
#[derive(Clone)]
pub struct PromptTemplate {
    inner: RustPromptTemplate,
}

#[pymethods]
impl PromptTemplate {
    /// Create a new template.
    #[new]
    fn new(template: String) -> Self {
        Self {
            inner: RustPromptTemplate::new(template),
        }
    }

    /// Set a default value for a variable.
    fn with_default(&self, key: String, value: String) -> Self {
        Self {
            inner: self.inner.clone().with_default(key, value),
        }
    }

    /// Set strict mode (fail on missing variables).
    fn strict(&self) -> Self {
        Self {
            inner: self.inner.clone().strict(),
        }
    }

    /// Get the raw template string.
    #[getter]
    fn template(&self) -> String {
        self.inner.template().to_string()
    }

    /// Extract variable names from the template.
    fn variables(&self) -> Vec<String> {
        self.inner.variables()
    }

    /// Render the template with the given variables.
    fn render(&self, variables: &Bound<'_, PyDict>) -> PyResult<String> {
        let mut vars = std::collections::HashMap::new();
        for (k, v) in variables.iter() {
            let key: String = k.extract()?;
            let value: String = v.extract()?;
            vars.insert(key, value);
        }
        self.inner
            .render(&vars)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PromptTemplate(template='{}', vars={:?})",
            truncate_str(self.inner.template(), 50),
            self.inner.variables()
        )
    }
}

/// Configuration for a prompt template.
#[pyclass]
#[derive(Clone)]
pub struct PromptConfig {
    inner: RustPromptConfig,
}

#[pymethods]
impl PromptConfig {
    /// Create a new prompt config.
    #[new]
    fn new(name: String, template: String) -> Self {
        Self {
            inner: RustPromptConfig::new(name, template),
        }
    }

    /// Set the description.
    fn with_description(&self, description: String) -> Self {
        Self {
            inner: self.inner.clone().with_description(description),
        }
    }

    /// Add a default value.
    fn with_default(&self, key: String, value: String) -> Self {
        Self {
            inner: self.inner.clone().with_default(key, value),
        }
    }

    /// Set the category.
    fn with_category(&self, category: String) -> Self {
        Self {
            inner: self.inner.clone().with_category(category),
        }
    }

    /// Add a tag.
    fn with_tag(&self, tag: String) -> Self {
        Self {
            inner: self.inner.clone().with_tag(tag),
        }
    }

    /// Convert to a PromptTemplate.
    fn to_template(&self) -> PromptTemplate {
        PromptTemplate {
            inner: self.inner.clone().into_template(),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn template(&self) -> String {
        self.inner.template.clone()
    }

    #[getter]
    fn category(&self) -> Option<String> {
        self.inner.category.clone()
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "PromptConfig(name='{}', template='{}')",
            self.inner.name,
            truncate_str(&self.inner.template, 50)
        )
    }
}

/// A library for storing and retrieving prompt templates.
#[pyclass]
pub struct PromptLibrary {
    inner: RustPromptLibrary,
}

#[pymethods]
impl PromptLibrary {
    /// Create a new empty prompt library.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustPromptLibrary::new(),
        }
    }

    /// Add a template to the library.
    fn add(&mut self, name: String, template: PromptTemplate) {
        self.inner.add(name, template.inner);
    }

    /// Add a template with a category.
    fn add_with_category(&mut self, name: String, template: PromptTemplate, category: String) {
        self.inner.add_with_category(name, template.inner, category);
    }

    /// Get a template by name.
    fn get(&self, name: &str) -> Option<PromptTemplate> {
        self.inner
            .get(name)
            .map(|t| PromptTemplate { inner: t.clone() })
    }

    /// Remove a template.
    fn remove(&mut self, name: &str) -> Option<PromptTemplate> {
        self.inner.remove(name).map(|t| PromptTemplate { inner: t })
    }

    /// List all template names.
    fn list(&self) -> Vec<String> {
        self.inner
            .list()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// List templates in a category.
    fn list_by_category(&self, category: &str) -> Vec<String> {
        self.inner
            .list_by_category(category)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// List all categories.
    fn categories(&self) -> Vec<String> {
        self.inner
            .categories()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if a template exists.
    fn has(&self, name: &str) -> bool {
        self.inner.has(name)
    }

    /// Render a template by name with variables.
    fn render(&self, name: &str, variables: &Bound<'_, PyDict>) -> PyResult<String> {
        let mut vars = std::collections::HashMap::new();
        for (k, v) in variables.iter() {
            let key: String = k.extract()?;
            let value: String = v.extract()?;
            vars.insert(key, value);
        }
        self.inner
            .render(name, &vars)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("PromptLibrary(templates={})", self.inner.len())
    }
}

/// Common prompt templates for typical use cases.
#[pyclass]
pub struct CommonPrompts;

#[pymethods]
impl CommonPrompts {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Get a summarization prompt template.
    #[staticmethod]
    fn summarize() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::summarize(),
        }
    }

    /// Get a translation prompt template.
    #[staticmethod]
    fn translate() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::translate(),
        }
    }

    /// Get a question answering prompt template.
    #[staticmethod]
    fn qa() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::qa(),
        }
    }

    /// Get a code review prompt template.
    #[staticmethod]
    fn code_review() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::code_review(),
        }
    }

    /// Get a text classification prompt template.
    #[staticmethod]
    fn classify() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::classify(),
        }
    }

    /// Get an entity extraction prompt template.
    #[staticmethod]
    fn extract_entities() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::extract_entities(),
        }
    }

    /// Get a rewriting prompt template.
    #[staticmethod]
    fn rewrite() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::rewrite(),
        }
    }

    /// Get a chain-of-thought reasoning prompt template.
    #[staticmethod]
    fn chain_of_thought() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::chain_of_thought(),
        }
    }

    /// Create a library with all common prompts.
    #[staticmethod]
    fn library() -> PromptLibrary {
        PromptLibrary {
            inner: RustCommonPrompts::library(),
        }
    }
}

// ============================================================================
// Automation
// ============================================================================

/// Task status enumeration.
#[pyclass]
#[derive(Clone)]
pub struct AutomationTaskStatus {
    inner: RustTaskStatus,
}

#[pymethods]
impl AutomationTaskStatus {
    #[staticmethod]
    fn pending() -> Self {
        Self {
            inner: RustTaskStatus::Pending,
        }
    }

    #[staticmethod]
    fn running() -> Self {
        Self {
            inner: RustTaskStatus::Running,
        }
    }

    #[staticmethod]
    fn completed() -> Self {
        Self {
            inner: RustTaskStatus::Completed,
        }
    }

    #[staticmethod]
    fn failed() -> Self {
        Self {
            inner: RustTaskStatus::Failed,
        }
    }

    #[staticmethod]
    fn cancelled() -> Self {
        Self {
            inner: RustTaskStatus::Cancelled,
        }
    }

    fn is_pending(&self) -> bool {
        matches!(self.inner, RustTaskStatus::Pending)
    }

    fn is_running(&self) -> bool {
        matches!(self.inner, RustTaskStatus::Running)
    }

    fn is_completed(&self) -> bool {
        matches!(self.inner, RustTaskStatus::Completed)
    }

    fn is_failed(&self) -> bool {
        matches!(self.inner, RustTaskStatus::Failed)
    }

    fn __repr__(&self) -> String {
        format!("TaskStatus::{:?}", self.inner)
    }
}

/// Schedule configuration.
#[pyclass]
#[derive(Clone)]
pub struct AutomationScheduleConfig {
    inner: RustScheduleConfig,
}

#[pymethods]
impl AutomationScheduleConfig {
    /// Run once immediately.
    #[staticmethod]
    fn once() -> Self {
        Self {
            inner: RustScheduleConfig::Once,
        }
    }

    /// Run at regular intervals.
    #[staticmethod]
    fn interval(seconds: u64) -> Self {
        Self {
            inner: RustScheduleConfig::interval(seconds),
        }
    }

    /// Run on a cron schedule.
    #[staticmethod]
    fn cron(expression: String) -> Self {
        Self {
            inner: RustScheduleConfig::cron(expression),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RustScheduleConfig::Once => "ScheduleConfig::Once".to_string(),
            RustScheduleConfig::Interval { seconds } => {
                format!("ScheduleConfig::Interval({}s)", seconds)
            }
            RustScheduleConfig::Cron { expression } => {
                format!("ScheduleConfig::Cron('{}')", expression)
            }
        }
    }
}

/// Configuration for an automation task.
#[pyclass]
#[derive(Clone)]
pub struct AutomationConfig {
    inner: RustAutomationConfig,
}

#[pymethods]
impl AutomationConfig {
    /// Create a new automation config.
    #[new]
    fn new(id: String, name: String) -> Self {
        Self {
            inner: RustAutomationConfig::new(id, name),
        }
    }

    /// Set the description.
    fn with_description(&self, description: String) -> Self {
        Self {
            inner: self.inner.clone().with_description(description),
        }
    }

    /// Set the schedule.
    fn with_schedule(&self, schedule: AutomationScheduleConfig) -> Self {
        Self {
            inner: self.inner.clone().with_schedule(schedule.inner),
        }
    }

    /// Set max retries.
    fn with_retries(&self, max_retries: u32) -> Self {
        Self {
            inner: self.inner.clone().with_retries(max_retries),
        }
    }

    /// Set timeout.
    fn with_timeout(&self, timeout_secs: u64) -> Self {
        Self {
            inner: self.inner.clone().with_timeout(timeout_secs),
        }
    }

    /// Set enabled status.
    fn enabled(&self, enabled: bool) -> Self {
        Self {
            inner: self.inner.clone().enabled(enabled),
        }
    }

    /// Add a tag.
    fn with_tag(&self, tag: String) -> Self {
        Self {
            inner: self.inner.clone().with_tag(tag),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn is_enabled(&self) -> bool {
        self.inner.enabled
    }

    #[getter]
    fn max_retries(&self) -> u32 {
        self.inner.max_retries
    }

    #[getter]
    fn timeout_secs(&self) -> u64 {
        self.inner.timeout_secs
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "AutomationConfig(id='{}', name='{}', enabled={})",
            self.inner.id, self.inner.name, self.inner.enabled
        )
    }
}

/// Context for task execution.
#[pyclass]
#[derive(Clone)]
pub struct AutomationTaskContext {
    inner: RustTaskContext,
}

#[pymethods]
impl AutomationTaskContext {
    /// Create a new task context.
    #[new]
    fn new() -> Self {
        Self {
            inner: RustTaskContext::new(),
        }
    }

    /// Set an input value.
    fn set_input(&mut self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let json_value = py_to_json(value)?;
        self.inner.set_input(key, json_value);
        Ok(())
    }

    /// Get an input value.
    fn get_input(&self, py: Python<'_>, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.get_input(key) {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    /// Set a state value.
    fn set_state(&mut self, key: String, value: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let json_value = py_to_json(value)?;
        self.inner.set_state(key, json_value);
        Ok(())
    }

    /// Get a state value.
    fn get_state(&self, py: Python<'_>, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.get_state(key) {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskContext(inputs={}, state={})",
            self.inner.input.len(),
            self.inner.state.len()
        )
    }
}

/// Output from a task execution.
#[pyclass]
#[derive(Clone)]
pub struct AutomationTaskOutput {
    inner: RustTaskOutput,
}

#[pymethods]
impl AutomationTaskOutput {
    /// Create a successful output.
    #[staticmethod]
    fn success(message: String) -> Self {
        Self {
            inner: RustTaskOutput::success(message),
        }
    }

    /// Create a failed output.
    #[staticmethod]
    fn failure(message: String) -> Self {
        Self {
            inner: RustTaskOutput::failure(message),
        }
    }

    /// Set output data.
    fn with_data(&self, data: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_value = py_to_json(data)?;
        Ok(Self {
            inner: self.inner.clone().with_data(json_value),
        })
    }

    /// Set duration.
    fn with_duration(&self, duration_ms: u64) -> Self {
        Self {
            inner: self.inner.clone().with_duration(duration_ms),
        }
    }

    #[getter]
    fn is_success(&self) -> bool {
        self.inner.success
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.data {
            Some(v) => Ok(Some(json_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskOutput(success={}, message='{}')",
            self.inner.success,
            truncate_str(&self.inner.message, 50)
        )
    }
}

/// Record of a task execution.
#[pyclass]
#[derive(Clone)]
pub struct ExecutionRecord {
    inner: RustExecutionRecord,
}

#[pymethods]
impl ExecutionRecord {
    #[getter]
    fn execution_id(&self) -> String {
        self.inner.execution_id.clone()
    }

    #[getter]
    fn task_id(&self) -> String {
        self.inner.task_id.clone()
    }

    #[getter]
    fn started_at(&self) -> u64 {
        self.inner.started_at
    }

    #[getter]
    fn ended_at(&self) -> Option<u64> {
        self.inner.ended_at
    }

    #[getter]
    fn status(&self) -> AutomationTaskStatus {
        AutomationTaskStatus {
            inner: self.inner.status,
        }
    }

    #[getter]
    fn output(&self) -> Option<AutomationTaskOutput> {
        self.inner
            .output
            .as_ref()
            .map(|o| AutomationTaskOutput { inner: o.clone() })
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionRecord(id='{}', task='{}', status={:?})",
            self.inner.execution_id, self.inner.task_id, self.inner.status
        )
    }
}

/// Builder for creating automation workflows.
#[pyclass]
pub struct AutomationBuilder {
    config: RustAutomationConfig,
}

#[pymethods]
impl AutomationBuilder {
    /// Create a new automation builder.
    #[new]
    fn new(id: String) -> Self {
        Self {
            config: RustAutomationConfig::new(&id, "Unnamed Automation"),
        }
    }

    /// Set the name.
    fn name(&self, name: String) -> Self {
        let mut config = self.config.clone();
        config.name = name;
        Self { config }
    }

    /// Set the description.
    fn description(&self, description: String) -> Self {
        let mut config = self.config.clone();
        config.description = description;
        Self { config }
    }

    /// Run every N seconds.
    fn every_seconds(&self, seconds: u64) -> Self {
        let mut config = self.config.clone();
        config.schedule = RustScheduleConfig::interval(seconds);
        Self { config }
    }

    /// Run every N minutes.
    fn every_minutes(&self, minutes: u64) -> Self {
        let mut config = self.config.clone();
        config.schedule = RustScheduleConfig::interval(minutes * 60);
        Self { config }
    }

    /// Run every N hours.
    fn every_hours(&self, hours: u64) -> Self {
        let mut config = self.config.clone();
        config.schedule = RustScheduleConfig::interval(hours * 3600);
        Self { config }
    }

    /// Run on a cron schedule.
    fn cron(&self, expression: String) -> Self {
        let mut config = self.config.clone();
        config.schedule = RustScheduleConfig::cron(expression);
        Self { config }
    }

    /// Set max retries.
    fn retries(&self, max_retries: u32) -> Self {
        let mut config = self.config.clone();
        config.max_retries = max_retries;
        Self { config }
    }

    /// Set timeout.
    fn timeout(&self, timeout_secs: u64) -> Self {
        let mut config = self.config.clone();
        config.timeout_secs = timeout_secs;
        Self { config }
    }

    /// Get the configuration.
    fn build(&self) -> AutomationConfig {
        AutomationConfig {
            inner: self.config.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("AutomationBuilder(id='{}')", self.config.id)
    }
}

/// Helper to truncate strings for repr.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

/// LiteForge - High-performance Rust bindings for LLM completions.
#[pymodule]
fn liteforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Client classes
    m.add_class::<ForgeClient>()?;
    m.add_class::<AsyncForgeClient>()?;

    // Observability, module-level OTel init helpers.
    m.add_function(wrap_pyfunction!(init_otel, m)?)?;
    m.add_function(wrap_pyfunction!(otel_feature_enabled, m)?)?;

    // Streaming
    m.add_class::<CompletionStream>()?;

    // Chunking
    m.add_class::<Chunk>()?;
    m.add_function(wrap_pyfunction!(chunk, m)?)?;

    // Guardrails
    m.add_class::<GuardrailResult>()?;
    m.add_function(wrap_pyfunction!(detect_pii, m)?)?;
    m.add_function(wrap_pyfunction!(redact_pii, m)?)?;
    m.add_function(wrap_pyfunction!(find_pii, m)?)?;
    m.add_function(wrap_pyfunction!(detect_injection, m)?)?;
    m.add_function(wrap_pyfunction!(check_all, m)?)?;

    // Retry utilities
    m.add_class::<RetryConfig>()?;

    // Tools
    m.add_class::<PyTool>()?;
    m.add_class::<ToolRegistry>()?;
    m.add_class::<ToolExecutor>()?;
    m.add_class::<ToolResult>()?;
    m.add_function(wrap_pyfunction!(create_tool, m)?)?;
    m.add_function(wrap_pyfunction!(validate_json_schema, m)?)?;

    // Knowledge
    m.add_class::<Document>()?;
    m.add_class::<SearchResult>()?;
    m.add_class::<SearchOptions>()?;
    m.add_class::<ListOptions>()?;
    m.add_class::<KnowledgeStats>()?;
    m.add_class::<LocalKnowledgeBackend>()?;

    // RAG (Vector search)
    m.add_class::<EmbeddedDocument>()?;
    m.add_class::<VectorSearchResult>()?;
    m.add_class::<VectorIndex>()?;
    m.add_function(wrap_pyfunction!(cosine_similarity, m)?)?;
    m.add_function(wrap_pyfunction!(dot_product, m)?)?;
    m.add_function(wrap_pyfunction!(euclidean_distance, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;

    // Events
    m.add_class::<EventType>()?;
    m.add_class::<Event>()?;
    m.add_class::<EventBus>()?;

    // Hooks
    m.add_class::<HookEvent>()?;
    m.add_class::<HookContext>()?;
    m.add_class::<HookResult>()?;
    m.add_class::<HookManager>()?;

    // Agents
    m.add_class::<AgentState>()?;
    m.add_class::<AgentMemory>()?;
    m.add_class::<AgentConfig>()?;
    m.add_class::<StepType>()?;
    m.add_class::<StepResult>()?;
    m.add_class::<AgentStep>()?;
    m.add_class::<ToolCallingAgent>()?;

    // Orchestration
    m.add_class::<Intent>()?;
    m.add_class::<IntentRoute>()?;
    m.add_class::<CommonIntents>()?;
    m.add_class::<IntentRouter>()?;
    m.add_class::<RoutingDecision>()?;
    m.add_class::<SessionMessage>()?;
    m.add_class::<Session>()?;
    m.add_class::<SessionStore>()?;
    m.add_class::<WorkflowStep>()?;
    m.add_class::<Workflow>()?;
    m.add_class::<OrchestrationStepStatus>()?;
    m.add_class::<OrchestrationStrategy>()?;
    m.add_class::<OrchestratorConfig>()?;
    m.add_class::<OrchestrationResult>()?;

    // MCP (Model Context Protocol)
    m.add_class::<TransportType>()?;
    m.add_class::<McpServerConfig>()?;
    m.add_class::<McpConfig>()?;
    m.add_class::<McpServerState>()?;
    m.add_class::<McpStdioServer>()?;
    m.add_class::<McpHttpServer>()?;
    m.add_class::<McpSseServer>()?;

    // Observability
    m.add_class::<SpanKind>()?;
    m.add_class::<SpanStatus>()?;
    m.add_class::<SpanContext>()?;
    m.add_class::<Span>()?;
    m.add_class::<Tracer>()?;
    m.add_class::<MetricValue>()?;
    m.add_class::<MetricsCollector>()?;

    // Conversation Management
    m.add_class::<SummarizationStrategy>()?;
    m.add_class::<ConversationConfig>()?;
    m.add_class::<ManagedConversation>()?;
    m.add_class::<CompactingConversation>()?;

    // Human-in-the-Loop (HITL)
    m.add_class::<RiskLevel>()?;
    m.add_class::<ApprovalStatus>()?;
    m.add_class::<ApprovalRequest>()?;
    m.add_class::<ApprovalResult>()?;

    // Evals
    m.add_class::<TestCase>()?;
    m.add_class::<EvalResult>()?;
    m.add_class::<SuiteStats>()?;
    m.add_class::<SuiteResult>()?;

    // Scheduler
    m.add_class::<ScheduleType>()?;
    m.add_class::<JobStatus>()?;
    m.add_class::<OnceSchedule>()?;
    m.add_class::<IntervalSchedule>()?;
    m.add_class::<CronSchedule>()?;

    // Skills
    m.add_class::<SkillConfig>()?;
    m.add_class::<SkillInput>()?;
    m.add_class::<SkillOutput>()?;
    m.add_class::<PromptSkill>()?;
    m.add_function(wrap_pyfunction!(get_summarize_skill, m)?)?;
    m.add_function(wrap_pyfunction!(get_translate_skill, m)?)?;
    m.add_function(wrap_pyfunction!(get_extract_skill, m)?)?;
    m.add_function(wrap_pyfunction!(get_rewrite_skill, m)?)?;
    m.add_function(wrap_pyfunction!(get_qa_skill, m)?)?;

    // Pipelines
    m.add_class::<PipelineContext>()?;
    m.add_class::<PipelineStepOutput>()?;

    // Images
    m.add_class::<ImageSize>()?;
    m.add_class::<ImageQuality>()?;
    m.add_class::<ImageStyle>()?;
    m.add_class::<ImageResponseFormat>()?;
    m.add_class::<ImageRequest>()?;
    m.add_class::<ImageData>()?;
    m.add_class::<ImageResponse>()?;

    // Prompts
    m.add_class::<PromptTemplate>()?;
    m.add_class::<PromptConfig>()?;
    m.add_class::<PromptLibrary>()?;
    m.add_class::<CommonPrompts>()?;

    // Automation
    m.add_class::<AutomationTaskStatus>()?;
    m.add_class::<AutomationScheduleConfig>()?;
    m.add_class::<AutomationConfig>()?;
    m.add_class::<AutomationTaskContext>()?;
    m.add_class::<AutomationTaskOutput>()?;
    m.add_class::<ExecutionRecord>()?;
    m.add_class::<AutomationBuilder>()?;

    Ok(())
}
