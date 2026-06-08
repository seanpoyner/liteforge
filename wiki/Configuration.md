# Configuration

LiteForge reads configuration from (in order of precedence) **explicit builder arguments → environment
variables → a `.env` file → `~/.forge/config.toml` → built‑in defaults**. The same resolution applies
across Rust, Python, JS, and the CLI, because they all share the Rust core.

## Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `LITEFORGE_API_KEY` | Auth token | — (required for most providers) |
| `LITEFORGE_BASE_URL` | OpenAI‑compatible endpoint | provider default |
| `LITEFORGE_DEFAULT_MODEL` | Default model id | `claude-haiku-4.5` |
| `LITEFORGE_TIMEOUT` | Request timeout (seconds) | `60` |
| `LITEFORGE_DEFAULT_METADATA` | JSON merged into every request body's `metadata` | — |
| `LITEFORGE_EXTRA_CA_FILE` | Extra CA PEM, added to **this client only** | — |
| `LITEFORGE_OTEL_CAPTURE_PROMPTS` | Capture prompt/completion text in spans | `false` |

### Fallback keys

To make LiteForge a drop‑in for existing setups, several common variables are honored as fallbacks
when the `LITEFORGE_*` one is unset:

- **API key:** `LITEFORGE_API_KEY` → `OPENAI_API_KEY` → `ANTHROPIC_API_KEY`
- **Base URL:** `LITEFORGE_BASE_URL` → `ANTHROPIC_BASE_URL` → `OPENAI_BASE_URL`

This is what lets you point an OpenAI‑oriented tool's `OPENAI_BASE_URL` at a LiteLLM proxy and have
LiteForge follow it — see **[LiteLLM and Ollama](LiteLLM-and-Ollama)**.

Standard OpenTelemetry variables (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`,
`OTEL_RESOURCE_ATTRIBUTES`) are honored when the `otel` feature is enabled — see
**[Observability and Telemetry](Observability-and-Telemetry)**.

## `.env` file

The SDK auto‑loads a `.env` from the working directory (via `dotenvy`):

```env
LITEFORGE_API_KEY=sk-…
LITEFORGE_BASE_URL=https://api.openai.com/v1
LITEFORGE_DEFAULT_MODEL=gpt-4o-mini
LITEFORGE_TIMEOUT=120
```

## Programmatic configuration

### Rust

```rust
use liteforge::{ForgeClient, ForgeConfig};
use std::time::Duration;

// Client builder (shorthand)
let client = ForgeClient::builder()
    .api_key("sk-…")
    .base_url("https://api.openai.com/v1")
    .default_model("gpt-4o-mini")
    .timeout_secs(30)
    .build();

// Or a full ForgeConfig
let config = ForgeConfig::builder()
    .api_key("sk-…")
    .default_model("gpt-4o-mini")
    .timeout(Duration::from_secs(30))
    .build();

// Or load entirely from the environment
let config = ForgeConfig::from_env();
if !config.has_api_key() {
    eprintln!("Warning: no API key configured");
}
```

Build an async client from the same builder with `.build_async()`.

### JavaScript / TypeScript

```javascript
import { AsyncForgeClient } from '@seanpoyner/liteforge';

// From env
const a = new AsyncForgeClient();

// Explicit
const b = AsyncForgeClient.withConfig(
  'sk-…',                        // apiKey
  'gpt-4o-mini',                 // defaultModel
  'https://api.openai.com/v1',   // baseUrl
  30,                            // timeoutSecs
);
```

## CLI configuration & secrets

The CLI persists settings to `~/.forge/config.toml` and shell env files, and can store secrets in
the OS keyring as a secondary backend.

```bash
forge config init                      # scaffold config dir
forge config show                      # print effective settings
forge config set base-url https://api.openai.com/v1
forge config get model
forge config paths                     # show all effective paths

# Keyring-backed secrets
forge config set-secret forge-api-key  # prompts interactively
forge config get-secret forge-api-key
forge config list-secrets
forge config delete-secret forge-api-key
```

Example `config.toml`:

```toml
api_key = "sk-…"

[endpoints]
base_url = "https://api.openai.com/v1"

[defaults]
model = "gpt-4o-mini"
timeout = 60
```

| Platform | Keyring backend |
|---|---|
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring / KWallet) |
| Windows | Credential Manager |

## TLS

LiteForge uses **rustls** + **aws‑lc‑rs** with bundled WebPKI roots (no system OpenSSL dependency,
clean cross‑compilation). To trust an additional CA — e.g. a corporate inspection proxy — set
`LITEFORGE_EXTRA_CA_FILE` to a PEM bundle. It is added to LiteForge's HTTP client **only**, never to
the system trust store. See [Installation → Corporate CA](Installation#corporate-ca--tls-proxies-optin).

Next: **[Quickstart](Quickstart)** · **[LiteLLM and Ollama](LiteLLM-and-Ollama)**
