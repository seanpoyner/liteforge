use std::sync::Arc;
use liteforge::observability::{
    MetricsCollector as RustMetricsCollector, Span as RustSpan, SpanKind as RustSpanKind,
    Tracer as RustTracer,
};

#[napi(string_enum)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

fn js_span_kind_to_rust(k: &SpanKind) -> RustSpanKind {
    match k {
        SpanKind::Internal => RustSpanKind::Internal,
        SpanKind::Server => RustSpanKind::Server,
        SpanKind::Client => RustSpanKind::Client,
        SpanKind::Producer => RustSpanKind::Producer,
        SpanKind::Consumer => RustSpanKind::Consumer,
    }
}

#[napi(string_enum)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

#[napi(object)]
pub struct JsSpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

#[napi]
pub struct Tracer {
    inner: Arc<RustTracer>,
}

#[napi]
impl Tracer {
    #[napi(constructor)]
    pub fn new(service_name: String) -> Self {
        Self {
            inner: Arc::new(RustTracer::new(service_name)),
        }
    }

    #[napi]
    pub fn start_span(&self, name: String, kind: Option<SpanKind>) -> JsSpan {
        let mut builder = self.inner.start_span(&name);
        if let Some(k) = kind {
            builder = builder.kind(js_span_kind_to_rust(&k));
        }
        let span = builder.start();
        JsSpan { inner: Some(span) }
    }

    #[napi]
    pub fn drain_spans(&self) -> Vec<serde_json::Value> {
        self.inner
            .drain_spans()
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "trace_id": s.context.trace_id,
                    "span_id": s.context.span_id,
                    "parent_span_id": s.context.parent_span_id,
                    "status": format!("{:?}", s.status),
                    "duration_ms": s.duration().map(|d| d.as_millis() as f64),
                })
            })
            .collect()
    }

    #[napi]
    pub fn span_count(&self) -> u32 {
        self.inner.span_count() as u32
    }
}

#[napi]
pub struct JsSpan {
    inner: Option<RustSpan>,
}

#[napi]
impl JsSpan {
    #[napi]
    pub fn set_attribute(&mut self, key: String, value: String) {
        if let Some(ref mut span) = self.inner {
            span.set_attribute(key, value);
        }
    }

    #[napi]
    pub fn set_ok(&mut self) {
        if let Some(ref mut span) = self.inner {
            span.set_ok();
        }
    }

    #[napi]
    pub fn set_error(&mut self, message: String) {
        if let Some(ref mut span) = self.inner {
            span.set_error(message);
        }
    }

    #[napi]
    pub fn add_event(&mut self, name: String) {
        if let Some(ref mut span) = self.inner {
            span.event(name);
        }
    }

    #[napi]
    pub fn end(&mut self) {
        if let Some(ref mut span) = self.inner {
            span.end();
        }
    }
}

#[napi]
pub struct MetricsCollector {
    inner: Arc<RustMetricsCollector>,
}

#[napi]
impl MetricsCollector {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RustMetricsCollector::new()),
        }
    }

    #[napi]
    pub fn increment(&self, name: String, value: i64) {
        self.inner.increment(&name, value as u64);
    }

    #[napi]
    pub fn record_duration(&self, name: String, ms: f64) {
        self.inner.record_duration(&name, ms as u64);
    }

    #[napi]
    pub fn gauge(&self, name: String, value: f64) {
        self.inner.gauge(&name, value);
    }

    #[napi]
    pub fn snapshot(&self) -> serde_json::Value {
        let snap = self.inner.snapshot();
        serde_json::to_value(&snap).unwrap_or(serde_json::Value::Null)
    }
}
