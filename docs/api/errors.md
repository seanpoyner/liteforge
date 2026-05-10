# Error Handling

All fallible operations return `Result<T, ForgeError>`.

## ForgeError

```rust
use liteforge::ForgeError;
```

### Variants

| Variant | HTTP Status | Description |
|---------|-------------|-------------|
| `Authentication` | 401 | Invalid or missing API key |
| `RateLimit` | 429 | Rate limit exceeded |
| `InvalidRequest` | 400 | Malformed request parameters |
| `Server` | 5xx | Server-side error |
| `Network` | -- | Connection failure |
| `Timeout` | -- | Request exceeded timeout |
| `Stream` | -- | SSE parsing error |
| `ModelNotFound` | 404 | Requested model not found |
| `Json` | -- | JSON serialization/deserialization error |
| `Config` | -- | Configuration error |
| `Internal` | -- | Internal SDK error |
| `Other` | -- | Unclassified error |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `status_code()` | `Option<u16>` | HTTP status code if applicable |
| `from_status(code, msg)` | `ForgeError` | Create from HTTP status code |

### From Implementations

- `From<reqwest::Error>` -- automatically maps network/timeout errors
- `From<serde_json::Error>` -- maps to `ForgeError::Json`

## Error Handling Patterns

### Basic

```rust
match client.complete(messages) {
    Ok(response) => println!("{}", response.content().unwrap_or("")),
    Err(ForgeError::Authentication(msg)) => eprintln!("Auth failed: {msg}"),
    Err(ForgeError::RateLimit(msg)) => eprintln!("Rate limited: {msg}"),
    Err(e) => eprintln!("Error: {e}"),
}
```

### With Retry

```rust
use liteforge::retry::{RetryConfig, with_retry, is_retryable};

let config = RetryConfig::new()
    .max_retries(3)
    .initial_delay_ms(1000)
    .backoff_multiplier(2.0);

let response = with_retry(&config, || {
    client.complete(messages.clone())
})?;
```

### Retryable Errors

`is_retryable()` returns `true` for:

- `RateLimit` (429)
- `Server` (5xx)
- `Network`
- `Timeout`

Non-retryable: `Authentication`, `InvalidRequest`, `ModelNotFound`, `Json`, `Config`.

## AgentError

Errors specific to the agent framework:

| Variant | Description |
|---------|-------------|
| `LlmError(ForgeError)` | Underlying API error |
| `ToolError(String)` | Tool execution failure |
| `MaxStepsExceeded` | Agent exceeded step limit |
| `Stopped` | Agent was manually stopped |
| `ConfigError(String)` | Agent configuration error |
| `Other(String)` | Unclassified agent error |
