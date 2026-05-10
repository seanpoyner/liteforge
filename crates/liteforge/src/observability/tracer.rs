//! Tracer for creating and managing spans.

use super::span::{Span, SpanBuilder, SpanContext};
use std::sync::{Arc, Mutex};

/// A tracer for creating spans.
#[derive(Debug, Clone)]
pub struct Tracer {
    service_name: String,
    spans: Arc<Mutex<Vec<Span>>>,
    current_context: Arc<Mutex<Option<SpanContext>>>,
}

impl Tracer {
    /// Create a new tracer.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            spans: Arc::new(Mutex::new(Vec::new())),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Start a new span.
    pub fn start_span(&self, name: impl Into<String>) -> SpanBuilder {
        let context = {
            let current = self.current_context.lock().unwrap();
            match &*current {
                Some(ctx) => ctx.child(),
                None => SpanContext::new(),
            }
        };

        SpanBuilder::new(name, context).service_name(self.service_name.clone())
    }

    /// Start a new span with a specific parent context.
    pub fn start_span_with_context(
        &self,
        name: impl Into<String>,
        parent: &SpanContext,
    ) -> SpanBuilder {
        SpanBuilder::new(name, parent.child()).service_name(self.service_name.clone())
    }

    /// Start a new root span (no parent).
    pub fn start_root_span(&self, name: impl Into<String>) -> SpanBuilder {
        SpanBuilder::new(name, SpanContext::new()).service_name(self.service_name.clone())
    }

    /// Set the current context for automatic parent propagation.
    pub fn set_current_context(&self, context: SpanContext) {
        *self.current_context.lock().unwrap() = Some(context);
    }

    /// Clear the current context.
    pub fn clear_current_context(&self) {
        *self.current_context.lock().unwrap() = None;
    }

    /// Get the current context.
    pub fn current_context(&self) -> Option<SpanContext> {
        self.current_context.lock().unwrap().clone()
    }

    /// Record a completed span.
    pub fn record_span(&self, span: Span) {
        self.spans.lock().unwrap().push(span);
    }

    /// Get all recorded spans without clearing.
    pub fn spans(&self) -> Vec<Span> {
        self.spans.lock().unwrap().clone()
    }

    /// Drain all recorded spans.
    pub fn drain_spans(&self) -> Vec<Span> {
        std::mem::take(&mut *self.spans.lock().unwrap())
    }

    /// Get the number of recorded spans.
    pub fn span_count(&self) -> usize {
        self.spans.lock().unwrap().len()
    }

    /// Clear all recorded spans.
    pub fn clear_spans(&self) {
        self.spans.lock().unwrap().clear();
    }

    /// Execute a closure within a span, recording it when complete.
    pub fn in_span<T, F>(&self, name: impl Into<String>, f: F) -> T
    where
        F: FnOnce(&mut Span) -> T,
    {
        let mut span = self.start_span(name).start();
        let result = f(&mut span);
        span.end();
        self.record_span(span);
        result
    }

    /// Execute an async closure within a span.
    pub async fn in_span_async<T, F, Fut>(&self, name: impl Into<String>, f: F) -> T
    where
        F: FnOnce(Span) -> Fut,
        Fut: std::future::Future<Output = (T, Span)>,
    {
        let span = self.start_span(name).start();
        let (result, mut span) = f(span).await;
        span.end();
        self.record_span(span);
        result
    }
}

/// A guard that automatically ends and records a span when dropped.
#[allow(dead_code)]
pub struct SpanGuard {
    span: Option<Span>,
    tracer: Tracer,
}

#[allow(dead_code)]
impl SpanGuard {
    /// Create a new span guard.
    pub fn new(span: Span, tracer: Tracer) -> Self {
        Self {
            span: Some(span),
            tracer,
        }
    }

    /// Get a reference to the span.
    pub fn span(&self) -> &Span {
        self.span.as_ref().unwrap()
    }

    /// Get a mutable reference to the span.
    pub fn span_mut(&mut self) -> &mut Span {
        self.span.as_mut().unwrap()
    }

    /// Add an event to the span.
    pub fn event(&mut self, name: impl Into<String>) {
        if let Some(span) = &mut self.span {
            span.event(name);
        }
    }

    /// Set the span status to Ok.
    pub fn set_ok(&mut self) {
        if let Some(span) = &mut self.span {
            span.set_ok();
        }
    }

    /// Set the span status to Error.
    pub fn set_error(&mut self, message: impl Into<String>) {
        if let Some(span) = &mut self.span {
            span.set_error(message);
        }
    }

    /// Manually end and record the span, consuming the guard.
    pub fn finish(mut self) {
        if let Some(mut span) = self.span.take() {
            span.end();
            self.tracer.record_span(span);
        }
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        if let Some(mut span) = self.span.take() {
            span.end();
            self.tracer.record_span(span);
        }
    }
}

/// Extension trait for Tracer to create span guards.
#[allow(dead_code)]
pub trait TracerExt {
    /// Start a guarded span that automatically ends when dropped.
    fn guarded_span(&self, name: impl Into<String>) -> SpanGuard;
}

impl TracerExt for Tracer {
    fn guarded_span(&self, name: impl Into<String>) -> SpanGuard {
        let span = self.start_span(name).start();
        SpanGuard::new(span, self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_creation() {
        let tracer = Tracer::new("test-service");
        assert_eq!(tracer.service_name(), "test-service");
        assert_eq!(tracer.span_count(), 0);
    }

    #[test]
    fn test_tracer_start_span() {
        let tracer = Tracer::new("test-service");
        let span = tracer.start_span("operation").start();

        assert_eq!(span.name, "operation");
        assert_eq!(span.service_name, Some("test-service".to_string()));
    }

    #[test]
    fn test_tracer_record_span() {
        let tracer = Tracer::new("test-service");
        let mut span = tracer.start_span("operation").start();
        span.end();

        tracer.record_span(span);
        assert_eq!(tracer.span_count(), 1);

        let spans = tracer.drain_spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(tracer.span_count(), 0);
    }

    #[test]
    fn test_tracer_in_span() {
        let tracer = Tracer::new("test-service");

        let result = tracer.in_span("compute", |span| {
            span.set_attribute("input", 42);
            42 * 2
        });

        assert_eq!(result, 84);
        assert_eq!(tracer.span_count(), 1);

        let spans = tracer.drain_spans();
        assert_eq!(spans[0].name, "compute");
        assert!(spans[0].is_ended());
    }

    #[test]
    fn test_tracer_context_propagation() {
        let tracer = Tracer::new("test-service");

        // Create a root span and set as current context
        let root = tracer.start_root_span("root").start();
        tracer.set_current_context(root.context.clone());

        // Child span should have root as parent
        let child = tracer.start_span("child").start();
        assert_eq!(child.parent_span_id(), Some(root.span_id()));
        assert_eq!(child.trace_id(), root.trace_id());

        tracer.clear_current_context();

        // New span should be a new root
        let new_root = tracer.start_span("new_root").start();
        assert!(new_root.parent_span_id().is_none());
    }

    #[test]
    fn test_span_guard() {
        let tracer = Tracer::new("test-service");

        {
            let mut guard = tracer.guarded_span("guarded_operation");
            guard.event("started");
            guard.set_ok();
            // Guard is dropped here
        }

        assert_eq!(tracer.span_count(), 1);
        let spans = tracer.drain_spans();
        assert!(spans[0].is_ended());
        assert_eq!(spans[0].events.len(), 1);
    }

    #[test]
    fn test_span_guard_manual_finish() {
        let tracer = Tracer::new("test-service");

        let guard = tracer.guarded_span("operation");
        guard.finish();

        assert_eq!(tracer.span_count(), 1);
    }

    #[tokio::test]
    async fn test_tracer_in_span_async() {
        let tracer = Tracer::new("test-service");

        let result = tracer
            .in_span_async("async_operation", |mut span| async move {
                span.event("processing");
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                (42, span)
            })
            .await;

        assert_eq!(result, 42);
        assert_eq!(tracer.span_count(), 1);

        let spans = tracer.drain_spans();
        assert!(spans[0].duration_us.unwrap() >= 10_000);
    }
}
