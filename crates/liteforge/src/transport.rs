//! Internal HTTP transport layer.
//!
//! Provides a single-dispatch pattern so every API method delegates to
//! one of two functions (`request` or `request_stream`) instead of
//! duplicating HTTP plumbing.
//!
//! # Observability
//!
//! With the `otel` cargo feature on, every outbound request carries the
//! W3C `traceparent` (and `baggage`) header for the currently-active
//! tracing span, which lets the LiteLLM gateway stitch its spans under
//! the caller's trace. Static headers from
//! [`ForgeConfig::default_headers`](crate::ForgeConfig::default_headers) and
//! sticky body metadata from
//! [`ForgeConfig::default_metadata`](crate::ForgeConfig::default_metadata)
//! are also merged in here.

use crate::config::ForgeConfig;
use crate::error::{Result, ForgeError};
use crate::streaming::parse_sse_stream;
use crate::types::ChatCompletionChunk;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::pin::Pin;
use std::str::FromStr;

/// Build the standard headers (Authorization + Content-Type), merge any
/// `default_headers` from config, and, when the `otel` feature is on,
/// inject the W3C trace context for the currently active span.
pub(crate) fn build_headers(config: &ForgeConfig) -> Result<HeaderMap> {
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| ForgeError::config("API key not configured"))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| ForgeError::config(format!("Invalid API key: {}", e)))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    for (k, v) in &config.default_headers {
        let name = HeaderName::from_str(k)
            .map_err(|e| ForgeError::config(format!("Invalid default header name '{}': {}", k, e)))?;
        let value = HeaderValue::from_str(v).map_err(|e| {
            ForgeError::config(format!("Invalid default header value for '{}': {}", k, e))
        })?;
        headers.insert(name, value);
    }

    inject_trace_context(&mut headers);

    Ok(headers)
}

/// Inject W3C `traceparent` and `baggage` into outbound headers from the
/// currently-active OTel context. No-op when there is no active span or
/// when the `otel` feature is disabled.
#[cfg(feature = "otel")]
fn inject_trace_context(headers: &mut HeaderMap) {
    use opentelemetry::global;
    use opentelemetry_http::HeaderInjector;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let cx = tracing::Span::current().context();
    let mut injector = HeaderInjector(headers);
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut injector);
    });
}

#[cfg(not(feature = "otel"))]
#[inline]
fn inject_trace_context(_headers: &mut HeaderMap) {
    // no-op: feature off
}

/// Shallow-merge `config.default_metadata` into a request body's top-level
/// `metadata` object. Per-request metadata wins on key collision.
///
/// LiteLLM honours top-level `metadata` (the probe confirmed `extra_body`
/// is rejected by Bedrock-routed models), so this is the documented
/// passthrough channel.
fn merge_default_metadata(mut body: serde_json::Value, config: &ForgeConfig) -> serde_json::Value {
    if config.default_metadata.is_empty() {
        return body;
    }
    let obj = match body.as_object_mut() {
        Some(o) => o,
        // Body is not a JSON object (rare, e.g. raw form/text): nothing
        // sensible to merge into. Leave it alone.
        None => return body,
    };

    let metadata = obj
        .entry("metadata")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    // If the caller supplied a non-object `metadata`, don't clobber it.
    let metadata_obj = match metadata.as_object_mut() {
        Some(m) => m,
        None => return body,
    };

    for (k, v) in &config.default_metadata {
        metadata_obj.entry(k.clone()).or_insert_with(|| v.clone());
    }

    body
}

/// Extract a structured [`ForgeError`] from a non-success HTTP response.
async fn extract_api_error(status: reqwest::StatusCode, response: reqwest::Response) -> ForgeError {
    let body: Option<serde_json::Value> = response.json().await.ok();
    let message = body
        .as_ref()
        .and_then(|b| b.get("error"))
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("Unknown error")
        .to_string();
    ForgeError::from_status(status.as_u16(), message, body)
}

/// Single-dispatch function for JSON request/response endpoints that
/// carry a serializable body (POST, PUT, PATCH).
pub(crate) async fn request_with_body<B: Serialize, T: DeserializeOwned>(
    http: &Client,
    config: &ForgeConfig,
    method: Method,
    path: &str,
    body: &B,
) -> Result<T> {
    let url = format!("{}{}", config.base_url, path);
    let headers = build_headers(config)?;
    let body_value = serde_json::to_value(body)?;
    let body_value = merge_default_metadata(body_value, config);

    let response = http
        .request(method, &url)
        .headers(headers)
        .json(&body_value)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(extract_api_error(status, response).await);
    }

    Ok(response.json().await?)
}

/// Single-dispatch function for JSON request/response endpoints with
/// no request body (GET, DELETE).
pub(crate) async fn request_no_body<T: DeserializeOwned>(
    http: &Client,
    config: &ForgeConfig,
    method: Method,
    path: &str,
) -> Result<T> {
    let url = format!("{}{}", config.base_url, path);
    let headers = build_headers(config)?;

    let response = http.request(method, &url).headers(headers).send().await?;

    let status = response.status();
    if !status.is_success() {
        return Err(extract_api_error(status, response).await);
    }

    Ok(response.json().await?)
}

/// Single-dispatch function for SSE streaming endpoints.
pub(crate) async fn request_stream<B: Serialize>(
    http: &Client,
    config: &ForgeConfig,
    path: &str,
    body: &B,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>> {
    let url = format!("{}{}", config.base_url, path);
    let headers = build_headers(config)?;
    let body_value = serde_json::to_value(body)?;
    let body_value = merge_default_metadata(body_value, config);

    let response = http
        .request(Method::POST, &url)
        .headers(headers)
        .json(&body_value)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(extract_api_error(status, response).await);
    }

    let byte_stream = response.bytes_stream();
    Ok(Box::pin(parse_sse_stream(byte_stream)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_with_default_metadata(meta: HashMap<String, serde_json::Value>) -> ForgeConfig {
        ForgeConfig {
            api_key: Some("test".into()),
            default_model: "test-model".into(),
            base_url: "http://example".into(),
            timeout: std::time::Duration::from_secs(60),
            default_headers: HashMap::new(),
            default_metadata: meta,
            otel: None,
        }
    }

    #[test]
    fn merge_metadata_adds_defaults_when_missing() {
        let mut defaults = HashMap::new();
        defaults.insert("app".into(), serde_json::json!("btsales"));
        let cfg = config_with_default_metadata(defaults);

        let body = serde_json::json!({"model":"x","messages":[]});
        let merged = merge_default_metadata(body, &cfg);

        assert_eq!(merged["metadata"]["app"], serde_json::json!("btsales"));
    }

    #[test]
    fn merge_metadata_preserves_caller_keys() {
        let mut defaults = HashMap::new();
        defaults.insert("app".into(), serde_json::json!("default"));
        defaults.insert("env".into(), serde_json::json!("preprod"));
        let cfg = config_with_default_metadata(defaults);

        let body = serde_json::json!({
            "model":"x",
            "messages":[],
            "metadata": {"app": "caller-wins"}
        });
        let merged = merge_default_metadata(body, &cfg);

        assert_eq!(merged["metadata"]["app"], serde_json::json!("caller-wins"));
        assert_eq!(merged["metadata"]["env"], serde_json::json!("preprod"));
    }

    #[test]
    fn merge_metadata_noop_when_no_defaults() {
        let cfg = config_with_default_metadata(HashMap::new());
        let body = serde_json::json!({"model":"x","messages":[]});
        let merged = merge_default_metadata(body.clone(), &cfg);
        assert_eq!(merged, body);
    }

    #[test]
    fn merge_metadata_skips_non_object_metadata() {
        let mut defaults = HashMap::new();
        defaults.insert("app".into(), serde_json::json!("btsales"));
        let cfg = config_with_default_metadata(defaults);

        let body = serde_json::json!({"metadata": "not-an-object"});
        let merged = merge_default_metadata(body.clone(), &cfg);
        assert_eq!(merged, body);
    }

    #[test]
    fn build_headers_merges_default_headers() {
        let mut cfg = config_with_default_metadata(HashMap::new());
        cfg.default_headers
            .insert("X-App-Id".into(), "test-app".into());
        cfg.default_headers
            .insert("X-Tenant".into(), "default".into());

        let headers = build_headers(&cfg).expect("headers ok");
        assert_eq!(headers.get("authorization").unwrap(), "Bearer test");
        assert_eq!(headers.get("x-app-id").unwrap(), "test-app");
        assert_eq!(headers.get("x-tenant").unwrap(), "default");
    }
}
