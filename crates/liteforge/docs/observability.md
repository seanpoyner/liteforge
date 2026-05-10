# Observability: OTel propagation, spans, and metadata passthrough

This page documents the SDK's OpenTelemetry integration, added in the
`otel` cargo feature, and how to use it from Rust, Python, JavaScript,
and Java.

## What ships in the box

The SDK now emits four kinds of observability signal when the `otel`
feature is enabled and a tracer provider has been initialised:

| Span | Where | Notable attributes |
| --- | --- | --- |
| `agent.step` | wraps each `ToolCallingAgent::step()` | `agent.name`, `agent.step.number`, `gen_ai.request.model` |
| `gen_ai.completion` | child span around each LLM call | `gen_ai.system="tipai"`, `gen_ai.request.{model,temperature,max_tokens}`, `gen_ai.usage.{input,output,total}_tokens`, `gen_ai.response.finish_reasons` |
| `mcp.tool_call` | child span around each tool invocation | `mcp.tool.name`, `mcp.tool.duration_ms`, `mcp.tool.result_size_bytes` |
| (HTTP CLIENT) | every outbound HTTP call carries W3C `traceparent`/`baggage` so the LiteLLM gateway's spans nest under ours |, |

Field names use the [OpenTelemetry GenAI semantic conventions][1] so
existing tooling (Dynatrace AI Observability dashboards, Grafana panels)
recognises them out of the box.

[1]: https://opentelemetry.io/docs/specs/semconv/gen-ai/

## Two channels for context

### 1. Trace stitching (W3C)

Every outbound request injects `traceparent` and `baggage` from the
currently-active span. When `init_otel(...)` has been called and a
tracer provider is registered, the LiteLLM gateway sees the trace
context and continues the trace under its own spans, so a Dynatrace
session view shows a single trace from FastAPI handler → `agent.step`
→ `gen_ai.completion` → gateway's LiteLLM spans → upstream model.

This works without any caller-side configuration, the headers are
attached automatically.

### 2. `metadata` passthrough

LiteLLM accepts a top-level `metadata: {...}` JSON field on chat
completion requests and forwards every key as a span attribute on its
own OTel spans. This is the supported channel for app-specific tagging
(`session_id`, `user_eid`, `purpose`, etc.).

> **Note:** `extra_body.metadata` is **not** the right field. The
> LiteLLM gateway routes through Bedrock, which rejects
> `extra_body` as a malformed key (HTTP 400). This was confirmed by
> probe before this change shipped.

There are two ways to set metadata:

1. **Sticky**: set on the client config; merged into every request:
   ```rust
   let cfg = ForgeConfig::builder()
       .default_metadata(HashMap::from([
           ("app".to_string(), json!("btsales")),
           ("deployment_env".to_string(), json!("preprod")),
       ]))
       .build();
   ```
2. **Per-request**: set on the `ChatCompletionRequest` itself:
   ```rust
   let req = ChatCompletionRequest::new(model, messages)
       .metadata(HashMap::from([
           ("session_id".to_string(), json!(session_id)),
           ("user_eid".to_string(), json!(user_eid)),
           ("purpose".to_string(), json!("agent")),
       ]));
   ```

The transport layer shallow-merges sticky into per-request, so per-call
keys win on collision and the rest fill in.

## Minimum setup

### Rust
```rust
use liteforge::{init_otel, OtelConfig};
use std::collections::HashMap;

let otel = OtelConfig {
    endpoint: Some("https://irn08782.apps.dynatrace.com/api/v2/otlp/v1/traces".into()),
    headers: HashMap::from([(
        "Authorization".into(),
        format!("Api-Token {}", std::env::var("DT_TOKEN")?),
    )]),
    service_name: Some("btsales-agent".into()),
    resource_attributes: HashMap::from([(
        "deployment.environment".into(),
        "preprod".into(),
    )]),
    capture_prompts: false,
};
init_otel(&otel)?;
```

A working end-to-end example lives in
`examples/with_otel.rs`. Run:
```bash
cargo run -p liteforge --example with_otel --features otel
```

### Python
```python
import liteforge

liteforge.init_otel(
    endpoint="https://irn08782.apps.dynatrace.com/api/v2/otlp/v1/traces",
    headers={"Authorization": f"Api-Token {os.environ['DT_TOKEN']}"},
    service_name="btsales-agent",
    resource_attributes={"deployment.environment": "preprod"},
)

client = liteforge.AsyncForgeClient(
    default_metadata={"app": "btsales", "deployment_env": "preprod"},
)
await client.complete(
    [{"role": "user", "content": "..."}],
    metadata={"session_id": session_id, "user_eid": user_eid, "purpose": "agent"},
)
```

### JavaScript
```javascript
const forge = require('@liteforge/sdk');

forge.initOtel({
  endpoint: 'https://irn08782.apps.dynatrace.com/api/v2/otlp/v1/traces',
  headers: { Authorization: `Api-Token ${process.env.DT_TOKEN}` },
  serviceName: 'btsales-agent',
  resourceAttributes: { 'deployment.environment': 'preprod' },
});

const client = forge.AsyncForgeClient.builder()
  .apiKey(process.env.LITEFORGE_API_KEY)
  .defaultMetadata({ app: 'btsales', deployment_env: 'preprod' })
  .build();
```

### Java
```java
String otelJson = new ObjectMapper().writeValueAsString(Map.of(
    "endpoint", "https://irn08782.apps.dynatrace.com/api/v2/otlp/v1/traces",
    "headers", Map.of("Authorization", "Api-Token " + System.getenv("DT_TOKEN")),
    "service_name", "btsales-agent",
    "resource_attributes", Map.of("deployment.environment", "preprod")
));
ForgeClient.nativeInitOtel(otelJson);
```

## Capture-prompts flag (non-prod only)

`OtelConfig.capture_prompts` (env: `LITEFORGE_OTEL_CAPTURE_PROMPTS=true`) attaches
truncated prompt and completion content to `gen_ai.completion` spans.
Truncated to 4 KB per attribute. **Off by default**, turn on only in
non-prod for debugging. Treat OTel as not a PII sink: deal economics,
account names, and customer details flow through chat content.

## Build matrix

| Build | Default | `--features otel` |
| --- | --- | --- |
| Deps pulled in | none new | `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `opentelemetry-http`, `tracing-opentelemetry`, `reqwest-middleware`, `reqwest-tracing` |
| `init_otel(cfg)` | no-op, returns Ok | configures global tracer + propagator |
| Trace headers | not injected | injected from active span |
| `agent.step` / `gen_ai.completion` / `mcp.tool_call` spans | tracing-only (no exporter); no overhead | exported via OTLP HTTP/proto |

The default build keeps zero new transitive deps so embedded users and
the CLI don't pay the OTel cost. Bindings (`liteforge-py`, `liteforge-js`,
`liteforge-java`) each forward their own `otel` feature to the core
crate, so a wheel/native module compiled without it is fully no-op.

## What we deliberately did not do

- **Keep the in-house `crates/liteforge/src/observability/` module
  untouched.** It defines a custom `Tracer` / `Span` API that is not
  wired anywhere in the agent or HTTP layer. A separate cleanup PR can
  reconcile / remove it; this change adds OTel alongside.
- **No metrics or logs export.** Traces only. The btsales backend's
  current OTel setup also exports traces only. Metrics/logs can be a
  follow-up once we know what we want.
- **No automatic dashboard provisioning.** The dashboard build is a
  separate work item that consumes the data this SDK now emits.
