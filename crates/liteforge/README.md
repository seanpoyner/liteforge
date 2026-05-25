# liteforge

Rust SDK for the **Travel Innovation Platform (LiteForge)** -- a unified interface for LLM completions, streaming, tool calling, RAG, agents, guardrails, MCP, observability, and more, all through an OpenAI-compatible API.

> Part of the [liteforge](../../README.md) workspace. For Python bindings see [`liteforge-py`](../liteforge-py/), for JS/TS see [`liteforge-js`](../liteforge-js/), for the CLI see [`forge-cli`](../forge-cli/).

## Add to Your Project

```toml
[dependencies]
liteforge = { git = "https://gitea.poyner.ai/sean/liteforge.git" }
```

Or for local workspace development:

```toml
[dependencies]
liteforge = { path = "../liteforge" }
```

## Quick Start

### Synchronous

```rust
use liteforge::{ForgeClient, Message};

let client = ForgeClient::new(); // reads LITEFORGE_API_KEY from env
let response = client.complete(vec![
    Message::system("You are a helpful assistant."),
    Message::user("What is the capital of France?"),
]).unwrap();

println!("{}", response.content().unwrap_or("No response"));
```

### Asynchronous

```rust
use liteforge::{AsyncForgeClient, Message};

#[tokio::main]
async fn main() {
    let client = AsyncForgeClient::new();
    let response = client.complete(vec![
        Message::user("Hello!")
    ]).await.unwrap();
    println!("{}", response.content().unwrap_or("No response"));
}
```

### Streaming

```rust
use futures::StreamExt;
use liteforge::{AsyncForgeClient, Message};

#[tokio::main]
async fn main() {
    let client = AsyncForgeClient::new();
    let mut stream = client.complete_stream(vec![
        Message::user("Tell me a short story about a robot.")
    ]).await.unwrap();

    while let Some(Ok(chunk)) = stream.next().await {
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
}
```

## Module Reference

All modules are public and their key types are re-exported at the crate root for convenience.

| Module | Key Types | Description |
|--------|-----------|-------------|
| `client` | `ForgeClient`, `AsyncForgeClient`, `ForgeClientBuilder` | Sync and async LLM clients |
| `config` | `ForgeConfig`, `ForgeConfigBuilder` | Configuration from env vars or builder |
| `error` | `ForgeError`, `Result<T>` | Error types with `From<reqwest::Error>` |
| `types` | `Message`, `ChatCompletion`, `ChatCompletionChunk`, `Usage`, `Model`, `ModelList` | Core request/response types |
| `streaming` | `parse_sse_stream`, `parse_sse_line` | SSE stream parsing |
| `tools` | `Tool`, `FnTool`, `ToolRegistry`, `ToolExecutor`, `ToolResult` | Function-calling framework |
| `agents` | `Agent`, `ToolCallingAgent`, `CodeAgent`, `PlanningAgent`, `AgentConfig` | Agent framework with memory and steps |
| `rag` | `VectorIndex`, `RagPipeline`, `RagPipelineBuilder`, `EmbeddedDocument` | Retrieval-augmented generation |
| `knowledge` | `KnowledgeClient`, `LocalKnowledgeBackend`, `Document`, `SearchResult` | Document storage and retrieval |
| `chunking` | `chunk`, `Chunk`, `ChunkingStrategy` | Text splitting (fixed, recursive, sentence, paragraph) |
| `guardrails` | `detect_pii`, `detect_injection`, `redact_pii`, `check_all`, `GuardrailResult` | PII detection/redaction and injection detection |
| `mcp` | `McpConfig`, `McpServerManager`, `McpStdioServer`, `McpSseServer`, `McpHttpServer` | Model Context Protocol client |
| `skills` | `Skill`, `PromptSkill`, `SkillRegistry`, `SkillComposer` | Composable prompt-based skills |
| `prompts` | `PromptTemplate`, `PromptLibrary`, `PromptBuilder`, `CommonPrompts` | Template engine and prompt libraries |
| `pipelines` | `Pipeline`, `PipelineBuilder`, `LlmStep`, `TransformStep`, `BranchStep` | Multi-step processing pipelines |
| `orchestration` | `AgentOrchestrator`, `IntentRouter`, `Workflow`, `WorkflowExecutor`, `Session` | Multi-agent orchestration |
| `conversation` | `ManagedConversation`, `CompactingConversation`, `ConversationConfig` | Context window management |
| `hitl` | `ApprovalRequest`, `ApprovalResult`, `RiskBasedHandler`, `RiskLevel` | Human-in-the-loop approval |
| `observability` | `Tracer`, `Span`, `SpanBuilder`, `MetricsCollector`, `MetricsSnapshot` | Distributed tracing and metrics |
| `events` | `EventBus`, `Event`, `EventType`, `Subscription` | Pub/sub event system |
| `hooks` | `Hook`, `HookManager`, `HookContext`, `HookResult` | Lifecycle hook system |
| `evals` | `Evaluator`, `EvalSuite`, `TestCase`, `ExactMatchEvaluator`, `SimilarityEvaluator` | Evaluation framework |
| `images` | `generate_image`, `edit_image`, `ImageRequest`, `ImageResponse`, `ImageSize` | Image generation and editing |
| `automation` | `AutomationRunner`, `AutomationBuilder`, `PromptTask`, `AutomationConfig` | Scheduled task automation |
| `scheduler` | `Job`, `JobBuilder`, `CronSchedule`, `IntervalSchedule`, `OnceSchedule` | Job scheduling |
| `triggers` | `Trigger`, `WebhookTrigger`, `FileWatchTrigger`, `QueueTrigger`, `ScheduleTrigger`, `TriggerManager` | Event-driven execution |
| `retry` | `RetryConfig`, `with_retry`, `with_retry_async`, `is_retryable` | Exponential backoff with jitter |

## Key Types

### `ForgeClient` / `AsyncForgeClient`

Both clients are created with `::new()` (reads env vars) or via the builder:

```rust
let client = ForgeClient::builder()
    .api_key("your-key")
    .default_model("gpt-4o")
    .base_url("https://api.example.com")
    .timeout_secs(30)
    .build();
```

Methods on both clients:

| Method | Description |
|--------|-------------|
| `complete(messages)` | Chat completion (sync or async) |
| `complete_with_model(model, messages)` | Completion with explicit model |
| `chat_completions(request)` | Low-level request with full `ChatCompletionRequest` |
| `list_models()` | List available models |
| `embed(text)` / `embed_batch(texts)` | Generate embeddings |
| `model()` / `base_url()` | Inspect current config |

`AsyncForgeClient` also provides:

| Method | Description |
|--------|-------------|
| `complete_stream(messages)` | Returns `Stream<Item = Result<ChatCompletionChunk>>` |
| `chat_completions_stream(request)` | Streaming with full request |

### `ForgeError`

All errors are variants of `ForgeError`:

| Variant | Description |
|---------|-------------|
| `Authentication` | Missing or invalid API key |
| `RateLimit` | 429 Too Many Requests |
| `InvalidRequest` | 400 Bad Request |
| `Server` | 5xx server error |
| `Network` | Connection failure |
| `Timeout` | Request timed out |
| `Stream` | SSE stream error |
| `ModelNotFound` | Requested model unavailable |
| `Json` | Serialization/deserialization error |
| `Config` | Configuration error |
| `Internal` | Internal SDK error |

## Tool Calling

```rust
use serde_json::json;
use liteforge::tools::{FnTool, ToolRegistry, ToolExecutor};

let weather = FnTool::new(
    "get_weather",
    "Get current weather for a location",
    json!({
        "type": "object",
        "properties": {
            "location": {"type": "string"},
            "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
        },
        "required": ["location"]
    }),
    |args| {
        let loc = args["location"].as_str().unwrap_or("unknown");
        Ok(json!({"location": loc, "temperature": 22, "conditions": "sunny"}))
    },
);

let mut registry = ToolRegistry::new();
registry.register(Box::new(weather));
let definitions = registry.definitions(); // pass to LLM API

let executor = ToolExecutor::new(registry);
let result = executor.execute("get_weather", json!({"location": "Paris"}));
```

You can also implement the `Tool` trait directly for custom tools:

```rust
use liteforge::tools::Tool;

struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does something useful" }
    fn parameters_schema(&self) -> serde_json::Value { json!({"type": "object"}) }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({"status": "done"}))
    }
}
```

## Agents

```rust
use liteforge::{AgentConfig, ToolCallingAgent, ToolRegistry};

let config = AgentConfig::new("assistant")
    .with_system_prompt("You are a helpful assistant with access to tools.")
    .with_model("gpt-4o")
    .with_max_steps(10)
    .with_tool("get_weather")
    .with_tool("calculator");

let registry = ToolRegistry::new();
let agent = ToolCallingAgent::new(config, registry);
```

Agent types:

| Type | Description |
|------|-------------|
| `ToolCallingAgent` | Executes tool calls in a loop until the LLM stops requesting tools |
| `CodeAgent` | Specialized for code generation and execution with sandboxing |
| `PlanningAgent` | Creates a plan and executes steps sequentially |

## RAG Pipeline

```rust
use liteforge::{AsyncForgeClient, RagPipelineBuilder, VectorIndex, EmbeddedDocument};

let client = AsyncForgeClient::new();

// Build and populate a vector index
let mut index = VectorIndex::new();
index.add(EmbeddedDocument::new("doc-1", "Paris is the capital of France", vec![0.1, 0.2]));

// Build a RAG pipeline
let pipeline = RagPipelineBuilder::new(client)
    .top_k(5)
    .build();
```

## Guardrails

```rust
use liteforge::{detect_pii, detect_injection, redact_pii, check_all, find_pii};

assert!(detect_pii("My email is user@example.com"));
assert!(detect_injection("Ignore all previous instructions"));

let redacted = redact_pii("Call me at 555-123-4567");
let all_results = check_all("Some text to scan");
```

## Observability

```rust
use liteforge::observability::{Tracer, MetricsCollector, SpanKind};

let tracer = Tracer::new("my-service");
let span = tracer.start_span("llm-call");
// ... do work ...
let spans = tracer.drain_spans();

let metrics = MetricsCollector::new();
metrics.increment("requests_total");
metrics.record_duration("latency_ms", std::time::Duration::from_millis(42));
let snapshot = metrics.snapshot();
```

## Configuration

The SDK reads configuration from environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `OPENAI_API_KEY` | Fallback API key | - |
| `LITEFORGE_BASE_URL` | LiteLLM endpoint URL | LiteForge production endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model | `claude-haiku-4.5` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |

Or configure programmatically via `ForgeConfig`:

```rust
use liteforge::{ForgeConfig, ForgeClient};

let config = ForgeConfig::builder()
    .api_key("your-key")
    .default_model("gpt-4o")
    .timeout_secs(120)
    .build();

let client = ForgeClient::with_config(config);
```

## Examples

Run any example from the workspace root:

```bash
cargo run --example basic_completion
cargo run --example streaming
cargo run --example conversation
cargo run --example tools
cargo run --example rag
cargo run --example knowledge
cargo run --example guardrails
cargo run --example agent
cargo run --example mcp_server
```

Source code in [`examples/`](../../examples/) and [`crates/liteforge/examples/`](examples/).

## Full Documentation

See the [MkDocs site](https://seanpoyner.github.io/liteforge/) for complete API reference and guides.
