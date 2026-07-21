//! Local stripping proxy for Claude Code -> LiteForge.
//!
//! LiteForge's LiteLLM proxy rejects the `context_management` body field and the
//! `context-management-*` entries in the `anthropic-beta` header that
//! recent Claude Code versions send by default. This module runs a tiny
//! localhost HTTP proxy that strips them before forwarding to the real LiteForge
//! base URL so Claude Code works unmodified.

use axum::{
    body::Body,
    extract::{Request, State},
    http::HeaderValue,
    response::Response,
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
struct ProxyState {
    upstream_base: String,
    http: reqwest::Client,
}

/// Start the stripping proxy. Returns `(local_url, addr)`. The server runs
/// on a tokio task until the process exits.
pub async fn start(upstream_base: String) -> std::io::Result<(String, SocketAddr)> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .expect("build reqwest client");

    // Claude Code appends `/v1/messages` to ANTHROPIC_BASE_URL (the proxy), so we
    // must forward to the upstream ROOT. The configured base ends in `/v1`
    // (OpenAI-style), which would yield `…/v1/v1/messages` (404). Strip a trailing
    // `/v1` so `/v1/messages` maps to the gateway's Anthropic endpoint.
    let upstream_root = {
        let b = upstream_base.trim_end_matches('/');
        b.strip_suffix("/v1").unwrap_or(b).to_string()
    };
    let state = ProxyState {
        upstream_base: upstream_root,
        http,
    };

    let app = Router::new().fallback(proxy).with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{}", addr);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok((url, addr))
}

async fn proxy(State(state): State<ProxyState>, req: Request) -> Response {
    let (parts, body) = req.into_parts();

    // Collect the request body. Claude Code's largest payload is messages +
    // tool results; 100 MiB is a generous ceiling.
    let bytes = match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => return err_response(400, format!("read body: {}", e)),
    };

    // Strip unsupported fields if the body is JSON. If parsing fails, pass
    // through unchanged — e.g. empty bodies on GET.
    let forwarded_body: Vec<u8> = match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(mut json) => {
            if let Some(obj) = json.as_object_mut() {
                // LiteLLM rejects `context_management`, and its Anthropic
                // pass-through rejects the newer Claude Code fields `thinking`
                // (e.g. {"type":"adaptive"}) and `output_config`
                // (e.g. {"effort":"high"}) as invalid_request_error, which makes
                // the gateway silently fall back to a small local model.
                obj.remove("context_management");
                obj.remove("thinking");
                obj.remove("output_config");

                // Claude Code's mid-conversation-system beta puts `role:"system"`
                // entries in `messages`. The upstream rejects "role 'system'", so
                // hoist them into the top-level `system` field (supported by every
                // model) to keep the request on Claude instead of the fallback.
                hoist_system_messages(obj);
            }
            serde_json::to_vec(&json).unwrap_or_else(|_| bytes.to_vec())
        }
        Err(_) => bytes.to_vec(),
    };

    let path = parts.uri.path();
    let query = parts
        .uri
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let url = format!("{}{}{}", state.upstream_base, path, query);

    // Copy inbound headers, stripping hop-by-hop and the context-management
    // beta negotiation.
    let mut headers = parts.headers.clone();
    headers.remove("host");
    headers.remove("content-length");
    headers.remove("transfer-encoding");
    headers.remove("connection");
    if let Some(v) = headers.get("anthropic-beta").cloned() {
        if let Ok(s) = v.to_str() {
            let kept: Vec<&str> = s
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty() && !p.starts_with("context-management"))
                .collect();
            if kept.is_empty() {
                headers.remove("anthropic-beta");
            } else if let Ok(hv) = HeaderValue::from_str(&kept.join(",")) {
                headers.insert("anthropic-beta", hv);
            }
        }
    }

    let upstream = match state
        .http
        .request(parts.method.clone(), &url)
        .headers(headers)
        .body(forwarded_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return err_response(502, format!("upstream: {}", e)),
    };

    let status = upstream.status();
    let resp_headers = upstream.headers().clone();
    let stream = upstream.bytes_stream();
    let body = Body::from_stream(stream);

    let mut builder = Response::builder().status(status);
    for (k, v) in resp_headers.iter() {
        let name = k.as_str();
        if name == "content-length" || name == "transfer-encoding" || name == "connection" {
            continue;
        }
        builder = builder.header(k, v);
    }
    builder
        .body(body)
        .unwrap_or_else(|e| err_response(500, format!("build response: {}", e)))
}

/// Move any `role:"system"` messages out of the `messages` array and into the
/// top-level `system` field (as text blocks). Some upstreams reject system-role
/// messages even when the model supports a top-level system prompt.
fn hoist_system_messages(obj: &mut serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;

    let mut sys_blocks: Vec<Value> = match obj.get("system") {
        Some(Value::String(s)) => vec![serde_json::json!({"type": "text", "text": s})],
        Some(Value::Array(a)) => a.clone(),
        _ => Vec::new(),
    };

    let messages = match obj.get_mut("messages") {
        Some(Value::Array(a)) => std::mem::take(a),
        _ => return,
    };

    let mut had_system = false;
    let mut kept: Vec<Value> = Vec::with_capacity(messages.len());
    for m in messages {
        if m.get("role").and_then(|r| r.as_str()) == Some("system") {
            had_system = true;
            match m.get("content") {
                Some(Value::String(s)) => {
                    sys_blocks.push(serde_json::json!({"type": "text", "text": s}))
                }
                Some(Value::Array(a)) => sys_blocks.extend(a.iter().cloned()),
                _ => {}
            }
        } else {
            kept.push(m);
        }
    }

    if let Some(Value::Array(a)) = obj.get_mut("messages") {
        *a = kept;
    }
    if had_system {
        obj.insert("system".to_string(), Value::Array(sys_blocks));
    }
}

fn err_response(status: u16, msg: String) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(msg))
        .unwrap()
}
