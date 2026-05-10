# Client API

The client module provides synchronous and asynchronous clients for the LiteForge API.

## ForgeClient

Synchronous client that wraps `AsyncForgeClient` with an internal Tokio runtime.

```rust
use liteforge::ForgeClient;
```

### Constructors

| Method | Description |
|--------|-------------|
| `ForgeClient::new()` | Create from environment config |
| `ForgeClient::with_config(config)` | Create with explicit `ForgeConfig` |
| `ForgeClient::builder()` | Start a `ForgeClientBuilder` |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `complete(messages)` | `Result<ChatCompletion>` | Send chat completion request |
| `complete_with_model(model, messages)` | `Result<ChatCompletion>` | Completion with specific model |
| `chat_completions(request)` | `Result<ChatCompletion>` | Full `ChatCompletionRequest` |
| `list_models()` | `Result<ModelList>` | List available models |
| `embed(text)` | `Result<EmbeddingResponse>` | Create single embedding |
| `embed_batch(texts)` | `Result<EmbeddingResponse>` | Create batch embeddings |
| `embeddings(request)` | `Result<EmbeddingResponse>` | Full `EmbeddingRequest` |
| `model()` | `&str` | Get default model name |
| `base_url()` | `&str` | Get configured base URL |

## AsyncForgeClient

Asynchronous client using `reqwest` with `tokio`.

```rust
use liteforge::AsyncForgeClient;
```

All methods from `ForgeClient` are available as `async fn`, plus streaming:

| Method | Returns | Description |
|--------|---------|-------------|
| `complete_stream(messages)` | `Result<impl Stream<Item = Result<ChatCompletionChunk>>>` | Stream completion |
| `chat_completions_stream(request)` | `Result<impl Stream<Item = Result<ChatCompletionChunk>>>` | Stream with full request |

## ForgeClientBuilder

Builder pattern for constructing clients:

```rust
let client = ForgeClient::builder()
    .api_key("sk-...")
    .default_model("gpt-4")
    .base_url("https://api.example.com")
    .timeout_secs(30)
    .build();       // -> ForgeClient

// Or build async:
let async_client = ForgeClient::builder()
    .api_key("sk-...")
    .build_async();  // -> AsyncForgeClient
```

| Method | Parameter | Description |
|--------|-----------|-------------|
| `api_key(key)` | `impl Into<String>` | Set API key |
| `default_model(model)` | `impl Into<String>` | Set default model |
| `base_url(url)` | `impl Into<String>` | Set API base URL |
| `timeout_secs(secs)` | `u64` | Set timeout in seconds |
| `build()` | -- | Build `ForgeClient` |
| `build_async()` | -- | Build `AsyncForgeClient` |
