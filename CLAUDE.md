# LiteForge — Claude Code Context

## What is this project?

LiteForge — a Rust-first SDK and CLI for LLM completions, agents, tools, MCP, guardrails, and RAG, with an OpenAI-compatible interface. The core SDK is Rust; first-class bindings are published for Python (PyO3), JavaScript/TypeScript (napi-rs), and Java (JNI).

- **Rust 2021 edition**, **tokio** async runtime, **reqwest + rustls** HTTP
- Backend: any OpenAI-compatible endpoint (LiteLLM, OpenAI, Anthropic, Bedrock via gateway, etc.)

## Workspace layout

```
crates/
├── liteforge/         # Core SDK (lib name: liteforge)
├── liteforge-py/      # Python bindings via PyO3 → import liteforge
├── liteforge-js/      # JavaScript/TypeScript bindings via napi-rs
├── liteforge-java/    # Java bindings via JNI (package: com.liteforge)
└── forge-cli/         # CLI binary `forge` — chat, models, config, serve, adk
```

Workspace `Cargo.toml` pins shared deps (reqwest, tokio, serde, OpenTelemetry stack).

## Core SDK modules (`crates/liteforge/src/`)

```
client.rs            # ForgeClient / AsyncForgeClient / ForgeClientBuilder
config.rs            # ForgeConfig / ForgeConfigBuilder, env-var resolution
error.rs             # ForgeError, Result<T>
transport.rs         # HTTP transport, header merging, retry hooks
streaming.rs         # SSE → ChatCompletionChunk stream
agents/              # BaseAgent, CodeAgent, ToolCallingAgent, planning
automation.rs        # Workflow runner
chunking.rs          # Chunk + ChunkingStrategy (fixed, sentence)
conversation/        # Stateful conversation with context compaction
evals/               # Eval suite, evaluators (exact_match, llm_judge)
events/              # EventBus + Subscription
guardrails/          # PII, prompt-injection detection
hitl/                # Human-in-the-loop approval flows
hooks/               # Lifecycle hooks (on_agent_start, on_tool_call, …)
images.rs            # Image input helpers
knowledge/           # Knowledge base client + local backend
mcp/                 # Model Context Protocol client + auth
observability/       # Tracing, metrics, structured logging
model_routing/       # Layer-2 content/quality selectors (feature `model-routing`)
orchestration/       # Multi-agent orchestrator/router (intent-based; distinct from routing/)
otel_init.rs         # OTLP exporter setup (gated by `otel` feature)
pipelines/           # Provider detection, capability resolution, model config
prompts/             # Prompt templates
rag/                 # Embedding + vector index
retry.rs             # with_retry / with_retry_async, RetryConfig
routing/             # Layer-1 model router: deployments/strategies/health/fallbacks (feature `routing`)
scheduler/           # Cron/interval scheduled jobs
skills/              # Skill registry + middleware
tools/               # Tool trait, ToolRegistry, ToolExecutor
triggers/            # Webhook, file watch, queue, schedule triggers
types/               # Message, ChatCompletion, ChatCompletionChunk, …
```

## Public re-exports (Rust)

```rust
pub use client::{AsyncForgeClient, ForgeClient, ForgeClientBuilder};
pub use config::{ForgeConfig, ForgeConfigBuilder, OtelConfig};
pub use error::{ForgeError, Result};
pub use types::{Message, ChatCompletion, ChatCompletionChunk, /* … */};
pub use chunking::{chunk, Chunk, ChunkingStrategy};
pub use guardrails::{detect_pii, detect_injection, redact_pii, check_all};
pub use retry::{is_retryable, with_retry, with_retry_async, RetryConfig};
```

## Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `LITEFORGE_API_KEY` | Auth token (also accepts `OPENAI_API_KEY`) | — |
| `LITEFORGE_BASE_URL` | OpenAI-compatible endpoint | — |
| `LITEFORGE_DEFAULT_MODEL` | Default model id | — |
| `LITEFORGE_TIMEOUT` | Request timeout (seconds) | `60` |
| `LITEFORGE_DEFAULT_METADATA` | JSON merged into every request body's `metadata` | — |
| `LITEFORGE_OTEL_CAPTURE_PROMPTS` | Capture prompt/completion text in spans | `false` |
| `FORGE_ROUTER_CONFIG` | Path to a router YAML (used by `forge serve --router` / `forge route`) | — |
| `FORGE_ROUTER_WEIGHTS` | Override the MF selector `weights_path` | — |
| `FORGE_ROUTER_EMBEDDING_BASE_URL` | Override the selector embedding endpoint | — |

Standard OTel env vars (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`, …) are honored when the `otel` feature is enabled.

## Build & test

```bash
# Workspace check / build
cargo check --workspace
cargo build --release -p liteforge          # core lib
cargo build --release -p forge-cli          # `forge` binary

# Bindings
cargo build --release -p liteforge-py       # cdylib for Python wheel (use maturin)
cargo build --release -p liteforge-js       # cdylib for napi (use scripts/build-js.sh)
cargo build --release -p liteforge-java     # cdylib for JNI

# Tests
cargo test --workspace
cargo test -p liteforge --lib
```

The CLI binary is `forge` (e.g. `target/release/forge --help`).

## Code style & conventions

- **Edition:** Rust 2021, MSRV not currently pinned.
- **Async:** tokio with `#[tokio::main]` for examples; library code uses `async fn` + `Send` futures.
- **Errors:** `thiserror`-derived `ForgeError`; never `unwrap()` in library code, prefer `?`.
- **Deps:** add to root `Cargo.toml` `[workspace.dependencies]` and reference `{ workspace = true }` from member crates.
- **Features:** keep `default = []`; gate optional integrations (e.g. `otel`, `routing`, `model-routing`) so consumers opt in. `model-routing` implies `routing`.
- **Public API:** use `#[non_exhaustive]` on enums/structs that may grow.
- **Tests:** colocate with the module (`#[cfg(test)] mod tests`); integration tests in `tests/`.

## Bindings

- **Python:** `crates/liteforge-py` → `import liteforge`. Build with `maturin build` from that crate dir.
- **JavaScript:** `crates/liteforge-js` → `require('liteforge')` / `import` (TypeScript types in `index.d.ts`). Build with `scripts/build-js.sh`.
- **Java:** `crates/liteforge-java` → `com.liteforge.ForgeClient` / `ForgeClientBuilder`. Build the cdylib then assemble JAR via `gradle build`.
