//! Observability support for tracing and metrics.
//!
//! This module provides tools for monitoring and debugging agent execution,
//! including distributed tracing and metrics collection.
//!
//! # Overview
//!
//! - **Tracer**: Creates and manages spans for tracking operations
//! - **Span**: Represents a unit of work with timing and attributes
//! - **MetricsCollector**: Collects and aggregates metrics
//!
//! # Example
//!
//! ```rust
//! use liteforge::observability::{Tracer, SpanKind};
//!
//! // Create a tracer
//! let tracer = Tracer::new("my-agent");
//!
//! // Start a span
//! let mut span = tracer.start_span("process_request")
//!     .kind(SpanKind::Internal)
//!     .attribute("user_id", "123")
//!     .start();
//!
//! // Do work...
//!
//! // End the span
//! span.end();
//!
//! // Get completed spans
//! let spans = tracer.drain_spans();
//! ```
//!
//! # Metrics Example
//!
//! ```rust
//! use liteforge::observability::MetricsCollector;
//!
//! let metrics = MetricsCollector::new();
//!
//! // Record metrics
//! metrics.increment("requests_total", 1);
//! metrics.record_duration("request_latency_ms", 150);
//! metrics.gauge("active_connections", 5.0);
//!
//! // Get a snapshot
//! let snapshot = metrics.snapshot();
//! ```

mod metrics;
mod span;
mod tracer;

pub use metrics::{HistogramData, MetricValue, MetricsCollector, MetricsSnapshot};
pub use span::{Span, SpanBuilder, SpanContext, SpanEvent, SpanKind, SpanStatus};
pub use tracer::Tracer;
