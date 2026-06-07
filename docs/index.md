# LiteForge (Rust)

**LiteForge** is a high-performance Rust SDK for building LLM applications, with Python bindings via PyO3 and JavaScript/TypeScript bindings via napi-rs.

## Overview

LiteForge provides a unified interface for interacting with LLMs through the LiteForge's OpenAI-compatible API. Built in Rust for performance and safety, it offers first-class support for Rust, Python, and JavaScript/TypeScript developers.

## Key Features

- **OpenAI-Compatible API** -- Drop-in replacement patterns for chat completions, embeddings, and streaming
- **Sync & Async** -- Both `ForgeClient` and `AsyncForgeClient` for flexible integration
- **Streaming** -- Server-Sent Events (SSE) support for real-time token-by-token responses
- **Tool Calling** -- Full function-calling framework with registry, executor, and schema validation
- **RAG Pipeline** -- Built-in chunking, vector indexing, and retrieval pipeline
- **Agent Framework** -- Tool-calling agents with memory, orchestration, and human-in-the-loop
- **Skills** -- Composable prompt-based skills (summarize, translate, extract, rewrite, Q&A)
- **Guardrails** -- PII detection/redaction and prompt injection detection
- **MCP Support** -- Model Context Protocol client for stdio, SSE, and HTTP transports
- **Observability** -- Distributed tracing and metrics collection
- **Prompt Templates** -- Variable substitution, prompt libraries, and common prompt patterns
- **Pipelines & Automation** -- Multi-step processing pipelines and scheduled task automation
- **Image Generation** -- Image generation, editing, and variation APIs
- **Python Bindings** -- Full PyO3 bindings exposing the entire API surface to Python
- **JavaScript/TypeScript Bindings** -- Native napi-rs bindings with auto-generated `.d.ts` type definitions
- **CLI** -- Feature-rich `forge` command for chat, embeddings, chunking, agents, and more
- **Agent Development Kit** -- Scaffold, build, run, and test containerized agent ecosystems via `forge adk`
- **Multi-Port Server** -- Role-based HTTP servers (user API, MCP, tools, A2A, knowledge, skills) via `forge serve`

## Project Structure

```
liteforge/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── liteforge/            # Core Rust library
│   ├── liteforge-py/         # Python bindings (PyO3/maturin)
│   ├── liteforge-js/         # JavaScript/TypeScript bindings (napi-rs)
│   └── forge-cli/            # CLI binary
├── examples/
│   ├── *.rs                # Rust examples
│   ├── python/             # Python examples
│   └── javascript/         # JavaScript examples
└── benchmarks/             # Performance benchmarks
```

## Quick Install

=== "macOS / Linux"

    ```bash
    curl -fsSL https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.sh | bash
    ```

=== "Windows (PowerShell)"

    ```powershell
    irm https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.ps1 | iex
    ```

=== "Homebrew"

    ```bash
    brew tap seanpoyner/forge https://github.com/seanpoyner/homebrew-forge.git
    brew install forge-cli
    ```

## Quick Links

| Resource | Description |
|----------|-------------|
| [Installation Guide](installation.md) | All install methods and platform setup |
| [Getting Started](getting-started.md) | SDK setup for Rust, Python, and JS/TS |
| [Quick Start](quickstart.md) | First API call in 2 minutes |
| [API Reference](api/client.md) | Full Rust API documentation |
| [Python Bindings](python/index.md) | Using LiteForge from Python |
| [JavaScript/TypeScript Bindings](javascript/index.md) | Using LiteForge from Node.js |
| [CLI Reference](cli.md) | Command-line tool usage |
