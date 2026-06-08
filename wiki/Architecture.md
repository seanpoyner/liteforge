# Architecture

LiteForge is a Cargo **workspace**: one core library, three language bindings, and a CLI — all
sharing the same engine. Write the behavior once in Rust; expose it everywhere.

## Workspace layout

```mermaid
graph TD
    subgraph ws ["Cargo workspace"]
        core["crates/liteforge\ncore SDK"]
        py["crates/liteforge-py\nPyO3 → import liteforge"]
        js["crates/liteforge-js\nnapi-rs → @seanpoyner/liteforge"]
        java["crates/liteforge-java\nJNI → com.liteforge"]
        cli["crates/forge-cli\nforge binary"]
    end
    py --> core
    js --> core
    java --> core
    cli --> core
```

| Crate | Role |
|---|---|
| `crates/liteforge` | The engine: client, types, streaming, tools, agents, RAG, MCP, guardrails, … |
| `crates/liteforge-py` | Python bindings (PyO3, built with maturin) |
| `crates/liteforge-js` | JS/TS bindings (napi‑rs) |
| `crates/liteforge-java` | Java bindings (JNI, experimental) |
| `crates/forge-cli` | The `forge` CLI |

The workspace `Cargo.toml` pins shared dependencies (reqwest + rustls, tokio, serde, the OpenTelemetry
stack) and a single inherited `version`, so a release bumps all crates together.

## Request lifecycle

What happens on a `complete(...)` call:

```mermaid
sequenceDiagram
    participant App
    participant Client as ForgeClient / AsyncForgeClient
    participant Cfg as ForgeConfig
    participant Tx as transport (reqwest + rustls)
    participant API as OpenAI-compatible endpoint

    App->>Client: complete(messages)
    Client->>Cfg: resolve api_key / base_url / model / headers
    Client->>Tx: build request (merge default headers + metadata)
    Tx->>API: POST /chat/completions
    Note over Tx,API: retries on 429 / 5xx / network<br/>(exponential backoff + jitter)
    API-->>Tx: response (or SSE stream)
    Tx-->>Client: ChatCompletion / Stream<ChatCompletionChunk>
    Client-->>App: typed result (Result<_, ForgeError>)
```

Key pieces:

- **`config.rs`** resolves settings with the precedence in [Configuration](Configuration).
- **`transport.rs`** owns the reqwest client, header/metadata merging, and the retry hooks.
- **`retry.rs`** decides what's retryable (`is_retryable`) and computes backoff (`RetryConfig`).
- **`streaming.rs`** turns the SSE byte stream into typed `ChatCompletionChunk`s.
- **`error.rs`** maps everything to a single `ForgeError` enum (auth, rate limit, server, network,
  timeout, streaming, serialization, configuration, …).

## Bindings are thin

```mermaid
flowchart TB
    subgraph host ["Host language"]
        u["User code (Python / JS / Java)"]
    end
    u --> shim["Binding shim\n(type conversion only)"]
    shim --> core["liteforge core\n(all logic)"]
    core --> net["reqwest + rustls"]
```

A binding's job is **type marshaling** — turn a Python dict / JS object / Java object into the core's
`Message`, call the core, convert the result back. No business logic is duplicated, which is why
behavior stays consistent and why CPU‑bound helpers run at Rust speed (see
[Language Bindings](Language-Bindings)).

## The core, by capability

```mermaid
mindmap
  root((liteforge core))
    Client
      sync ForgeClient
      async AsyncForgeClient
      builder + config
    Conversation
      chat completions
      streaming (SSE)
      conversation compaction
    Tools & Agents
      tools (registry/executor)
      tool-calling agent
      code agent
      planning agent
      orchestration & routing
    Knowledge
      chunking
      embeddings
      RAG vector index
      knowledge base
    Safety
      guardrails (PII/injection)
      HITL approvals
      hooks
    Ops
      retry/backoff
      observability (trace/metrics)
      OTel export
      evals
    Extend
      MCP client
      skills
      prompts
      pipelines
      scheduler
      triggers
      events
      images
```

## Design choices

- **OpenAI‑compatible protocol** — chat completions, embeddings, and models endpoints follow the
  OpenAI spec, so any compatible backend (OpenAI, Anthropic, LiteLLM, Ollama, Bedrock via gateway)
  works by changing one base URL.
- **Async‑first, sync‑wrapped** — library code is `async`; `ForgeClient` wraps it with an embedded
  tokio runtime for blocking callers.
- **Trait‑based extensibility** — `Tool`, `Agent`, `Hook`, `Evaluator`, `ApprovalHandler` are traits
  you can implement.
- **rustls + aws‑lc‑rs** — no system OpenSSL dependency; clean cross‑compilation; bundled WebPKI
  roots, with an opt‑in extra‑CA hook (`LITEFORGE_EXTRA_CA_FILE`).
- **Lean by default** — `default = []`; optional integrations (e.g. `otel`) are feature‑gated so
  consumers opt in.

See the [`docs/`](https://github.com/seanpoyner/liteforge/tree/main/docs) tree and
[docs.rs/liteforge](https://docs.rs/liteforge) for module‑level detail.
