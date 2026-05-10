//! # LiteForge
//!
//! A Rust SDK for the LiteForge with an OpenAI-compatible API.
//!
//! ## Quick Start
//!
//! ```no_run
//! use liteforge::{ForgeClient, Message};
//!
//! // Create client (reads LITEFORGE_API_KEY from environment)
//! let client = ForgeClient::new();
//!
//! // Make a completion request
//! let response = client.complete(vec![
//!     Message::user("What is the capital of France?")
//! ]).unwrap();
//!
//! println!("{}", response.content().unwrap_or("No response"));
//! ```
//!
//! ## Async Usage
//!
//! ```no_run
//! use liteforge::{AsyncForgeClient, Message};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = AsyncForgeClient::new();
//!
//!     let response = client.complete(vec![
//!         Message::user("Hello!")
//!     ]).await.unwrap();
//!
//!     println!("{}", response.content().unwrap_or("No response"));
//! }
//! ```
//!
//! ## Streaming
//!
//! ```no_run
//! use liteforge::{AsyncForgeClient, Message};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = AsyncForgeClient::new();
//!
//!     let mut stream = client.complete_stream(vec![
//!         Message::user("Tell me a story")
//!     ]).await.unwrap();
//!
//!     while let Some(chunk) = stream.next().await {
//!         if let Ok(chunk) = chunk {
//!             if let Some(content) = chunk.content() {
//!                 print!("{}", content);
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Configuration
//!
//! The SDK reads configuration from environment variables:
//!
//! - `LITEFORGE_API_KEY` or `OPENAI_API_KEY`: API key for authentication
//! - `LITEFORGE_BASE_URL`: Custom API endpoint
//! - `LITEFORGE_DEFAULT_MODEL`: Default model to use
//! - `LITEFORGE_TIMEOUT`: Request timeout in seconds
//!
//! You can also configure programmatically:
//!
//! ```no_run
//! use liteforge::ForgeClient;
//!
//! let client = ForgeClient::builder()
//!     .api_key("your-api-key")
//!     .default_model("gpt-4")
//!     .base_url("https://api.example.com")
//!     .timeout_secs(30)
//!     .build();
//! ```

pub mod agents;
pub mod automation;
pub mod chunking;
pub mod client;
pub mod config;
pub mod conversation;
pub mod error;
pub mod evals;
pub mod events;
pub mod guardrails;
pub mod hitl;
pub mod hooks;
pub mod images;
pub mod knowledge;
pub mod mcp;
pub mod observability;
pub mod orchestration;
pub mod otel_init;
pub mod pipelines;
pub mod prompts;
pub mod rag;
pub mod retry;
pub mod scheduler;
pub mod skills;
pub mod streaming;
pub mod tools;
pub(crate) mod transport;
pub mod triggers;
pub mod types;

// Re-export main types at crate root
pub use client::{AsyncForgeClient, ForgeClient, ForgeClientBuilder};
pub use config::{OtelConfig, ForgeConfig, ForgeConfigBuilder};
pub use error::{Result, ForgeError};
pub use otel_init::{init_otel, otel_feature_enabled};
pub use types::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionRequest, Choice, ChoiceDelta, EmbeddingData,
    EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage, FunctionCall, Message,
    Model, ModelList, StreamChoice, ToolCall, ToolDefinition, ToolParameters, Usage,
};

// Re-export chunking
pub use chunking::{chunk, Chunk, ChunkingStrategy};

// Re-export guardrails
pub use guardrails::{
    check_all, detect_injection, detect_pii, find_pii, redact_pii, GuardrailResult, PiiType,
    INJECTION_PATTERNS, PII_PATTERNS,
};

// Re-export retry utilities
pub use retry::{is_retryable, with_retry, with_retry_async, RetryConfig};

// Re-export tools
pub use tools::{
    validate_json_schema, FnTool, FunctionDefinition, SchemaValidationError, Tool, ToolExecutor,
    ToolRegistry, ToolResult,
};

// Re-export knowledge
pub use knowledge::{
    Document, KnowledgeClient, KnowledgeStats, ListOptions, LocalKnowledgeBackend, SearchOptions,
    SearchResult, SyncKnowledgeClient,
};

// Re-export RAG
pub use rag::{
    cosine_similarity, dot_product, euclidean_distance, normalize, EmbeddedDocument, RagConfig,
    RagPipeline, RagPipelineBuilder, RetrievalResult, VectorIndex, VectorSearchResult,
};

// Re-export events
pub use events::{Event, EventBus, EventData, EventType, Subscription};

// Re-export hooks
pub use hooks::{Hook, HookContext, HookManager, HookResult};

// Re-export agents
pub use agents::{
    Agent, AgentConfig, AgentContext, AgentError, AgentMemory, AgentState, AgentStep, StepResult,
    StepType, ToolCallingAgent,
};

// Re-export orchestration
pub use orchestration::{
    AgentOrchestrator, CommonIntents, EchoExecutor, ExecutionContext, Intent, IntentRoute,
    IntentRouter, OrchestratedAgent, OrchestrationResult, OrchestrationStrategy,
    OrchestratorConfig, OrchestratorError, RoutingDecision, Session, SessionMessage, SessionStore,
    StepExecutionResult, StepExecutor, StepStatus, ToolCallingAgentWrapper, Workflow,
    WorkflowError, WorkflowExecutor, WorkflowResult, WorkflowStep,
};

// Re-export observability
pub use observability::{
    MetricValue, MetricsCollector, MetricsSnapshot, Span, SpanBuilder, SpanContext, SpanEvent,
    SpanKind, SpanStatus, Tracer,
};

// Re-export conversation
pub use conversation::{
    CompactingConversation, ConversationConfig, ManagedConversation, SummarizationStrategy,
};

// Re-export HITL
pub use hitl::{
    ApprovalHandler, ApprovalRequest, ApprovalResult, ApprovalStatus, AutoApprovalHandler,
    DenyAllHandler, QueueApprovalHandler, RiskBasedHandler, RiskLevel, TimeoutApprovalHandler,
};

// Re-export evals
pub use evals::{
    ContainsEvaluator, EvalResult, EvalSuite, Evaluator, ExactMatchEvaluator, JsonMatchEvaluator,
    RegexEvaluator, SimilarityEvaluator, SuiteResult, SuiteStats, TestCase, TestCaseBuilder,
};

// Re-export scheduler
pub use scheduler::{
    CronSchedule, IntervalSchedule, Job, JobBuilder, JobStatus, OnceSchedule, Schedule,
    ScheduleType,
};

// Re-export pipelines
pub use pipelines::{
    BranchStep, LlmStep, ModelTransform, Pipeline, PipelineBuilder, PipelineContext, PipelineError,
    PipelineOutput, PipelineResult, PipelineStep, StepOutput, TransformChain, TransformStep,
};

// Re-export images
pub use images::{
    create_variations, edit_image, generate_image, ImageData, ImageEditRequest, ImageQuality,
    ImageRequest, ImageResponse, ImageSize, ImageStyle, ImageVariationRequest, ResponseFormat,
};

// Re-export prompts
pub use prompts::{
    CommonPrompts, PromptBuilder, PromptConfig, PromptLibrary, PromptTemplate, TemplateError,
    TemplateResult,
};

// Re-export automation
pub use automation::{
    AutomationBuilder, AutomationConfig, AutomationError, AutomationResult, AutomationRunner,
    AutomationTask, ExecutionRecord, PromptTask, ScheduleConfig, TaskContext, TaskOutput,
    TaskStatus,
};

// Re-export futures for streaming
pub use futures;
pub use tokio_stream;
