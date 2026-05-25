# Configuration

LiteForge can be configured via environment variables, `.env` files, or programmatically.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `OPENAI_API_KEY` | Fallback API key (if `LITEFORGE_API_KEY` not set) | -- |
| `LITEFORGE_BASE_URL` | Custom API endpoint | LiteForge endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model for completions | `claude-haiku-4.5` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |

## `.env` File

The SDK uses `dotenvy` to automatically load a `.env` file from the current directory:

```env
LITEFORGE_API_KEY=your-api-key
LITEFORGE_DEFAULT_MODEL=gpt-4
LITEFORGE_BASE_URL=https://custom-endpoint.example.com
LITEFORGE_TIMEOUT=120
```

## Programmatic Configuration

### ForgeConfig

Build configuration objects directly:

```rust
use liteforge::ForgeConfig;
use std::time::Duration;

let config = ForgeConfig::builder()
    .api_key("your-key")
    .default_model("gpt-4")
    .base_url("https://api.example.com")
    .timeout(Duration::from_secs(30))
    .build();
```

### From Environment

Load configuration from environment variables:

```rust
let config = ForgeConfig::from_env();
```

### ForgeClient Builder

The client builder provides a convenient shorthand:

```rust
let client = ForgeClient::builder()
    .api_key("your-key")
    .default_model("gpt-4")
    .base_url("https://api.example.com")
    .timeout_secs(30)
    .build();
```

### Async Client

```rust
let client = ForgeClient::builder()
    .api_key("your-key")
    .build_async();
```

## Config Validation

`ForgeConfig` provides helper methods for validation:

```rust
let config = ForgeConfig::from_env();

if !config.has_api_key() {
    eprintln!("Warning: No API key configured");
}

// Returns Err(ForgeError::Config) if no API key
let key = config.api_key_required()?;
```

## JavaScript / TypeScript Configuration

The JS bindings read the same environment variables as the Rust core. Create a client from environment:

```javascript
import { AsyncForgeClient } from '@forge/sdk';

// Reads LITEFORGE_API_KEY, LITEFORGE_BASE_URL, LITEFORGE_DEFAULT_MODEL, LITEFORGE_TIMEOUT from env
const client = new AsyncForgeClient();
```

Or configure programmatically:

```javascript
const client = AsyncForgeClient.withConfig(
  'your-api-key',              // apiKey
  'gpt-4',                     // defaultModel
  'https://api.example.com',   // baseUrl
  30,                          // timeoutSecs
);
```

## Defaults

| Setting | Default Value |
|---------|--------------|
| Base URL | `https://api.example.com/v1` |
| Model | `claude-haiku-4.5` |
| Timeout | 60 seconds |

## TLS

LiteForge uses **rustls** with the **aws-lc-rs** crypto provider for TLS. This eliminates the need for system OpenSSL and provides better cross-compilation support across macOS, Linux, and Windows. WebPKI root certificates are bundled via the `webpki-roots` crate.
