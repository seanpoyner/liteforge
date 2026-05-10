# Retry API

Configurable retry logic with exponential backoff.

## RetryConfig

```rust
use liteforge::retry::RetryConfig;

let config = RetryConfig::new()
    .max_retries(3)
    .initial_delay_ms(1000)
    .max_delay_ms(30000)
    .backoff_multiplier(2.0);

let delay = config.delay_for_attempt(2); // 4000ms
```

| Method | Description |
|--------|-------------|
| `new()` | Default config (3 retries, 1s initial, 2x backoff) |
| `max_retries(n)` | Set max retry count |
| `initial_delay_ms(ms)` | Set initial delay |
| `max_delay_ms(ms)` | Set maximum delay cap |
| `backoff_multiplier(m)` | Set backoff multiplier |
| `delay_for_attempt(n)` | Calculate delay for attempt N |

## is_retryable

```rust
pub fn is_retryable(error: &ForgeError) -> bool
```

Returns `true` for: `RateLimit`, `Server` (5xx), `Network`, `Timeout`.

## with_retry (sync)

```rust
pub fn with_retry<F, T>(config: &RetryConfig, f: F) -> Result<T>
where F: Fn() -> Result<T>
```

## with_retry_async

```rust
pub async fn with_retry_async<F, Fut, T>(config: &RetryConfig, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>
```
