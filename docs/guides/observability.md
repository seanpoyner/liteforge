# Observability

LiteForge includes built-in distributed tracing and metrics collection.

## Tracing

The `Tracer` collects spans representing units of work:

```rust
use liteforge::observability::{Tracer, SpanKind};

let tracer = Tracer::new();

// Start a span
let mut span = tracer.start_span("chat_completion")
    .kind(SpanKind::Client)
    .attribute("model", "gpt-4")
    .attribute("message_count", "3")
    .start();

// Do work...

// End the span
span.set_status(SpanStatus::Ok);
span.end();

// Drain collected spans
let spans = tracer.drain_spans();
```

### Span Properties

| Field | Description |
|-------|-------------|
| `name` | Operation name |
| `kind` | `Internal`, `Client`, `Server`, `Producer`, `Consumer` |
| `status` | `Ok`, `Error(String)`, `Unset` |
| `events` | Timestamped annotations |
| `attributes` | Key-value metadata |

## Metrics

Collect counters, gauges, and duration histograms:

```rust
use liteforge::observability::MetricsCollector;
use std::time::Duration;

let metrics = MetricsCollector::new();

// Counters
metrics.increment("api.requests.total");
metrics.increment("api.requests.total");

// Duration histograms
metrics.record_duration("api.latency", Duration::from_millis(150));

// Gauges
metrics.gauge("active_connections", 42.0);

// Snapshot all metrics
let snapshot = metrics.snapshot();
for (name, value) in &snapshot.metrics {
    println!("{name}: {value:?}");
}
```

### Metric Types

| Method | Type | Use Case |
|--------|------|----------|
| `increment(name)` | Counter | Request counts, error counts |
| `record_duration(name, duration)` | Histogram | Latency, processing time |
| `gauge(name, value)` | Gauge | Active connections, queue size |

## Event System

The SDK's event bus integrates with observability:

```rust
use liteforge::events::{EventBus, EventType};

let bus = EventBus::new();

// Subscribe to LLM events for tracing
let sub = bus.subscribe(EventType::LlmRequest);
let sub2 = bus.subscribe(EventType::LlmResponse);

// Subscribe to multiple event types
let filtered = bus.subscribe_many(vec![
    EventType::ToolCall,
    EventType::ToolResult,
    EventType::ToolError,
]);
```

See [Events & Hooks API](../api/events.md) for the full event system reference.

## Python Usage

```python
from liteforge import Tracer, MetricsCollector

# Tracing
tracer = Tracer()
span = tracer.start_span("my_operation")
# ... do work ...
span.end()
spans = tracer.drain_spans()

# Metrics
metrics = MetricsCollector()
metrics.increment("requests")
metrics.gauge("active", 5.0)
snapshot = metrics.snapshot()
```

## JavaScript / TypeScript Usage

```javascript
import { Tracer, MetricsCollector } from '@forge/sdk';

// Tracing
const tracer = new Tracer('my-service');
const span = tracer.startSpan('chat_completion');
// ... do work ...
span.end();
const spans = tracer.drainSpans();

// Metrics
const metrics = new MetricsCollector();
metrics.increment('api.requests.total');
metrics.gauge('active_connections', 42.0);
const snapshot = metrics.snapshot();
```
