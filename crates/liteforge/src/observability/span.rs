//! Span types for distributed tracing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Generate a unique ID.
fn generate_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}{:08x}", timestamp, count as u32)
}

/// Span context for propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    /// Trace ID (shared across related spans).
    pub trace_id: String,
    /// Span ID (unique to this span).
    pub span_id: String,
    /// Parent span ID (if any).
    pub parent_span_id: Option<String>,
    /// Baggage items for propagation.
    #[serde(default)]
    pub baggage: HashMap<String, String>,
}

impl SpanContext {
    /// Create a new root span context.
    pub fn new() -> Self {
        Self {
            trace_id: generate_id(),
            span_id: generate_id(),
            parent_span_id: None,
            baggage: HashMap::new(),
        }
    }

    /// Create a child context from this context.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_id(),
            parent_span_id: Some(self.span_id.clone()),
            baggage: self.baggage.clone(),
        }
    }

    /// Add a baggage item.
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }
}

impl Default for SpanContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Kind of span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpanKind {
    /// Internal operation.
    #[default]
    Internal,
    /// Incoming request (server-side).
    Server,
    /// Outgoing request (client-side).
    Client,
    /// Message producer.
    Producer,
    /// Message consumer.
    Consumer,
}

impl std::fmt::Display for SpanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanKind::Internal => write!(f, "internal"),
            SpanKind::Server => write!(f, "server"),
            SpanKind::Client => write!(f, "client"),
            SpanKind::Producer => write!(f, "producer"),
            SpanKind::Consumer => write!(f, "consumer"),
        }
    }
}

/// Status of a span.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Unset status.
    #[default]
    Unset,
    /// Operation completed successfully.
    Ok,
    /// Operation failed with error.
    Error(String),
}

/// An event that occurred during a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name.
    pub name: String,
    /// Timestamp (Unix epoch microseconds).
    pub timestamp_us: u64,
    /// Event attributes.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl SpanEvent {
    /// Create a new event.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute.
    pub fn attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// A span representing a unit of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Span context.
    pub context: SpanContext,
    /// Operation name.
    pub name: String,
    /// Span kind.
    pub kind: SpanKind,
    /// Start time (Unix epoch microseconds).
    pub start_time_us: u64,
    /// End time (Unix epoch microseconds).
    pub end_time_us: Option<u64>,
    /// Duration in microseconds.
    pub duration_us: Option<u64>,
    /// Span status.
    pub status: SpanStatus,
    /// Span attributes.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
    /// Events that occurred during the span.
    #[serde(default)]
    pub events: Vec<SpanEvent>,
    /// Service/component name.
    pub service_name: Option<String>,
}

impl Span {
    /// Create a new span.
    pub fn new(name: impl Into<String>, context: SpanContext) -> Self {
        Self {
            context,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            end_time_us: None,
            duration_us: None,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
            service_name: None,
        }
    }

    /// Set the span kind.
    pub fn kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add an attribute (builder pattern, consumes self).
    pub fn attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Set an attribute on an existing span (mutable reference).
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Set the service name.
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Add an event.
    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    /// Record an event with just a name.
    pub fn event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent::new(name));
    }

    /// Set status to Ok.
    pub fn set_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }

    /// Set status to Error.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error(message.into());
    }

    /// End the span and record duration.
    pub fn end(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        self.end_time_us = Some(now);
        self.duration_us = Some(now.saturating_sub(self.start_time_us));
    }

    /// Check if the span has ended.
    pub fn is_ended(&self) -> bool {
        self.end_time_us.is_some()
    }

    /// Get duration as Duration type.
    pub fn duration(&self) -> Option<Duration> {
        self.duration_us.map(Duration::from_micros)
    }

    /// Get the trace ID.
    pub fn trace_id(&self) -> &str {
        &self.context.trace_id
    }

    /// Get the span ID.
    pub fn span_id(&self) -> &str {
        &self.context.span_id
    }

    /// Get the parent span ID.
    pub fn parent_span_id(&self) -> Option<&str> {
        self.context.parent_span_id.as_deref()
    }
}

/// Builder for creating spans.
pub struct SpanBuilder {
    name: String,
    context: SpanContext,
    kind: SpanKind,
    attributes: HashMap<String, serde_json::Value>,
    service_name: Option<String>,
}

impl SpanBuilder {
    /// Create a new span builder.
    pub fn new(name: impl Into<String>, context: SpanContext) -> Self {
        Self {
            name: name.into(),
            context,
            kind: SpanKind::Internal,
            attributes: HashMap::new(),
            service_name: None,
        }
    }

    /// Set the span kind.
    pub fn kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add an attribute.
    pub fn attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Set the service name.
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Build and start the span.
    pub fn start(self) -> Span {
        let mut span = Span::new(self.name, self.context);
        span.kind = self.kind;
        span.attributes = self.attributes;
        span.service_name = self.service_name;
        span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_context_creation() {
        let ctx = SpanContext::new();
        assert!(!ctx.trace_id.is_empty());
        assert!(!ctx.span_id.is_empty());
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_span_context_child() {
        let parent = SpanContext::new();
        let child = parent.child();

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
    }

    #[test]
    fn test_span_creation() {
        let ctx = SpanContext::new();
        let span = Span::new("test_operation", ctx);

        assert_eq!(span.name, "test_operation");
        assert!(!span.is_ended());
    }

    #[test]
    fn test_span_end() {
        let ctx = SpanContext::new();
        let mut span = Span::new("test_operation", ctx);

        std::thread::sleep(std::time::Duration::from_millis(10));
        span.end();

        assert!(span.is_ended());
        assert!(span.duration_us.unwrap() >= 10_000); // At least 10ms
    }

    #[test]
    fn test_span_attributes() {
        let ctx = SpanContext::new();
        let span = Span::new("test", ctx)
            .attribute("key1", "value1")
            .attribute("key2", 42);

        assert_eq!(
            span.attributes.get("key1"),
            Some(&serde_json::json!("value1"))
        );
        assert_eq!(span.attributes.get("key2"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_span_events() {
        let ctx = SpanContext::new();
        let mut span = Span::new("test", ctx);

        span.event("started");
        span.add_event(SpanEvent::new("processed").attribute("count", 10));

        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "started");
        assert_eq!(span.events[1].name, "processed");
    }

    #[test]
    fn test_span_status() {
        let ctx = SpanContext::new();
        let mut span = Span::new("test", ctx);

        assert_eq!(span.status, SpanStatus::Unset);

        span.set_ok();
        assert_eq!(span.status, SpanStatus::Ok);

        span.set_error("something went wrong");
        assert_eq!(
            span.status,
            SpanStatus::Error("something went wrong".to_string())
        );
    }

    #[test]
    fn test_span_builder() {
        let ctx = SpanContext::new();
        let span = SpanBuilder::new("operation", ctx)
            .kind(SpanKind::Client)
            .attribute("url", "https://api.example.com")
            .service_name("my-service")
            .start();

        assert_eq!(span.name, "operation");
        assert_eq!(span.kind, SpanKind::Client);
        assert_eq!(span.service_name, Some("my-service".to_string()));
    }
}
