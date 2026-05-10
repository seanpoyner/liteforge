//! Optional OTel initialisation helpers.
//!
//! When the `otel` cargo feature is enabled, [`init_otel`] sets the
//! global tracer provider and W3C propagator so that
//! `#[tracing::instrument]`-emitted spans (and the headers injected in
//! [`crate::transport::build_headers`]) are exported via OTLP HTTP/proto
//! to the configured endpoint.
//!
//! When the feature is off, [`init_otel`] is a no-op that returns `Ok(())`.
//! This lets binding crates (Python, JS, Java) expose `init_otel(...)`
//! unconditionally, calling it in a feature-disabled build is harmless.

use crate::config::OtelConfig;
use crate::error::Result;
#[cfg(feature = "otel")]
use crate::error::ForgeError;

/// Initialise the global OTel tracer provider + propagator from
/// [`OtelConfig`]. Idempotent: safe to call multiple times. When the
/// `otel` cargo feature is disabled, returns `Ok(())` without doing
/// anything.
///
/// Existing `OTEL_EXPORTER_OTLP_*` env vars are honoured automatically by
/// the OTLP exporter as a fallback for any field not supplied here.
pub fn init_otel(config: &OtelConfig) -> Result<()> {
    init_otel_inner(config)
}

#[cfg(feature = "otel")]
fn init_otel_inner(config: &OtelConfig) -> Result<()> {
    use opentelemetry::global;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use opentelemetry_sdk::trace::Config as SdkConfig;
    use opentelemetry_sdk::Resource;
    use std::sync::OnceLock;

    static INIT: OnceLock<()> = OnceLock::new();
    if INIT.get().is_some() {
        return Ok(());
    }

    // Resource attributes, start with service.name (highest priority),
    // merge in the caller's resource_attributes map.
    let mut resource_kvs: Vec<KeyValue> = Vec::new();
    if let Some(name) = &config.service_name {
        resource_kvs.push(KeyValue::new("service.name", name.clone()));
    }
    for (k, v) in &config.resource_attributes {
        resource_kvs.push(KeyValue::new(k.clone(), v.clone()));
    }
    let resource = Resource::new(resource_kvs);

    // OTLP HTTP/proto exporter. Endpoint may be None, the SDK will fall
    // back to OTEL_EXPORTER_OTLP_ENDPOINT.
    let mut exporter_builder = opentelemetry_otlp::new_exporter()
        .http()
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary);
    if let Some(endpoint) = &config.endpoint {
        exporter_builder = exporter_builder.with_endpoint(endpoint.clone());
    }
    if !config.headers.is_empty() {
        exporter_builder = exporter_builder.with_headers(config.headers.clone());
    }

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter_builder)
        .with_trace_config(SdkConfig::default().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .map_err(|e| ForgeError::config(format!("failed to install OTLP pipeline: {}", e)))?;

    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(tracer_provider);
    let _ = INIT.set(());
    Ok(())
}

#[cfg(not(feature = "otel"))]
#[inline]
fn init_otel_inner(_config: &OtelConfig) -> Result<()> {
    // Feature disabled, caller's request is a no-op rather than an error
    // so that bindings can expose init_otel() unconditionally.
    Ok(())
}

/// True when this build of the SDK was compiled with the `otel` feature
/// enabled. Bindings can use this to decide whether to log a warning when
/// the user requests OTel from a feature-disabled build.
pub const fn otel_feature_enabled() -> bool {
    cfg!(feature = "otel")
}
