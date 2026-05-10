# Observability API

Distributed tracing and metrics collection.

## Tracer

```rust
use liteforge::observability::Tracer;

let tracer = Tracer::new();
let span = tracer.start_span("operation_name");
```

| Method | Description |
|--------|-------------|
| `new()` | Create a tracer |
| `start_span(name)` | Begin a `SpanBuilder` |
| `drain_spans()` | Collect and return all finished spans |

## SpanBuilder

```rust
let span = tracer.start_span("http_request")
    .kind(SpanKind::Client)
    .attribute("url", "https://api.example.com")
    .attribute("method", "POST")
    .start();
```

## Span

| Method | Description |
|--------|-------------|
| `set_status(status)` | Set `Ok`, `Error`, or `Unset` |
| `add_event(name)` | Add a timestamped event |
| `set_attribute(key, value)` | Add metadata |
| `end()` | Finish the span |

## SpanKind

`Internal` | `Client` | `Server` | `Producer` | `Consumer`

## SpanStatus

`Ok` | `Error(String)` | `Unset`

## MetricsCollector

```rust
use liteforge::observability::MetricsCollector;

let metrics = MetricsCollector::new();
```

| Method | Description |
|--------|-------------|
| `increment(name)` | Increment counter by 1 |
| `record_duration(name, duration)` | Record a duration |
| `gauge(name, value)` | Set a gauge value |
| `snapshot()` | Get `MetricsSnapshot` |

## MetricsSnapshot

```rust
pub struct MetricsSnapshot {
    pub metrics: HashMap<String, MetricValue>,
}
```

## MetricValue

Represents counter, histogram, or gauge values.
