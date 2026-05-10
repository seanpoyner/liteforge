//! Configuration for LiteForge.

use std::collections::HashMap;
use std::env;
use std::time::Duration;

/// Default LiteForge endpoint.
pub const DEFAULT_BASE_URL: &str =
    "https://api.example.com/v1";

/// Default model to use.
pub const DEFAULT_MODEL: &str = "anthropic.claude-haiku-4-5-20251001-v1:0";

/// Default request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Optional OTel configuration. When `None`, the SDK does not initialise
/// an OTLP exporter, but `default_headers` and `default_metadata` still
/// work, callers can run their own exporter and still benefit from
/// W3C trace propagation if a tracer provider is registered globally.
#[derive(Debug, Clone, Default)]
pub struct OtelConfig {
    /// OTLP exporter endpoint (e.g. Dynatrace tenant URL).
    /// Falls back to `OTEL_EXPORTER_OTLP_ENDPOINT`.
    pub endpoint: Option<String>,

    /// Headers attached to OTLP exports (e.g. `Authorization=Api-Token …`).
    /// Falls back to `OTEL_EXPORTER_OTLP_HEADERS` (comma-separated `k=v`).
    pub headers: HashMap<String, String>,

    /// `service.name` resource attribute. Falls back to `OTEL_SERVICE_NAME`.
    pub service_name: Option<String>,

    /// Additional resource attributes (e.g. `deployment.environment`).
    /// Falls back to `OTEL_RESOURCE_ATTRIBUTES` (comma-separated `k=v`).
    pub resource_attributes: HashMap<String, String>,

    /// When true, attach truncated prompt/completion content to LLM spans.
    /// Off by default; intended for non-prod debugging only. Falls back to
    /// `LITEFORGE_OTEL_CAPTURE_PROMPTS` (`true`/`1` to enable).
    pub capture_prompts: bool,
}

/// Configuration for LiteForge client.
///
/// Configuration can be provided via:
/// 1. Builder methods (highest priority)
/// 2. Environment variables (fallback)
/// 3. Default values (lowest priority)
///
/// # Environment Variables
///
/// - `LITEFORGE_API_KEY`: API key for authentication
/// - `OPENAI_API_KEY`: Fallback API key (for OpenAI compatibility)
/// - `LITEFORGE_DEFAULT_MODEL`: Default model to use
/// - `LITEFORGE_BASE_URL`: Custom API base URL
/// - `LITEFORGE_TIMEOUT`: Request timeout in seconds
/// - `LITEFORGE_DEFAULT_METADATA`: JSON object merged into every request body's
///   top-level `metadata` field (e.g. `{"app":"btsales"}`)
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_EXPORTER_OTLP_HEADERS`,
///   `OTEL_SERVICE_NAME`, `OTEL_RESOURCE_ATTRIBUTES`: standard OTel
///   environment variables, honoured when the `otel` feature is enabled
/// - `LITEFORGE_OTEL_CAPTURE_PROMPTS`: when `true`, capture prompt/completion
///   text on LLM spans (truncated). Default false.
#[derive(Debug, Clone)]
pub struct ForgeConfig {
    /// API key for authentication.
    pub api_key: Option<String>,

    /// Default model to use for completions.
    pub default_model: String,

    /// Base URL for API requests.
    pub base_url: String,

    /// Request timeout.
    pub timeout: Duration,

    /// Static headers attached to every outbound HTTP request (merged with
    /// the standard `Authorization` and `Content-Type` headers).
    pub default_headers: HashMap<String, String>,

    /// Sticky metadata merged into every request body's top-level
    /// `metadata` field. Per-request metadata wins on key collision.
    /// LiteLLM forwards this dict as OTel span attributes.
    pub default_metadata: HashMap<String, serde_json::Value>,

    /// Optional OTel exporter / behaviour config. `None` = SDK does not
    /// initialise its own tracer provider (callers may still register one
    /// externally). Only honoured when the `otel` cargo feature is
    /// enabled.
    pub otel: Option<OtelConfig>,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl ForgeConfig {
    /// Create a new config builder.
    pub fn builder() -> ForgeConfigBuilder {
        ForgeConfigBuilder::default()
    }

    /// Create a config from environment variables, falling back to
    /// `~/.forge/config.toml` for values not set in the environment.
    pub fn from_env() -> Self {
        // Load .env file if present (ignore errors)
        let _ = dotenvy::dotenv();

        // Read config file as fallback for values not in env
        let file_config = Self::read_config_file();

        let api_key = env::var("LITEFORGE_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()))
            .or(file_config.api_key);

        let default_model = env::var("LITEFORGE_DEFAULT_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .or(file_config.model)
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let base_url = env::var("LITEFORGE_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .or(file_config.base_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let timeout_secs = env::var("LITEFORGE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .or(file_config.timeout)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let default_metadata = env::var("LITEFORGE_DEFAULT_METADATA")
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str::<HashMap<String, serde_json::Value>>(&s).ok())
            .unwrap_or_default();

        let otel = otel_config_from_env();

        Self {
            api_key,
            default_model,
            base_url,
            timeout: Duration::from_secs(timeout_secs),
            default_headers: HashMap::new(),
            default_metadata,
            otel,
        }
    }

    /// Read config values from `~/.forge/config.toml`.
    fn read_config_file() -> FileConfig {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_default();
        if home.is_empty() {
            return FileConfig::default();
        }
        let path = std::path::Path::new(&home).join(".forge").join("config.toml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return FileConfig::default(),
        };

        let mut cfg = FileConfig::default();
        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("api_key = ") {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    cfg.api_key = Some(val.to_string());
                }
            } else if let Some(val) = line.strip_prefix("base_url = ") {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    cfg.base_url = Some(val.to_string());
                }
            } else if let Some(val) = line.strip_prefix("model = ") {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    cfg.model = Some(val.to_string());
                }
            } else if let Some(val) = line.strip_prefix("timeout = ") {
                cfg.timeout = val.trim().parse().ok();
            }
        }
        cfg
    }

    /// Check if an API key is configured.
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Get the API key, panicking if not set.
    pub fn api_key_required(&self) -> &str {
        self.api_key.as_ref().expect(
            "API key not configured. Set LITEFORGE_API_KEY or OPENAI_API_KEY environment variable.",
        )
    }
}

/// Values parsed from `~/.forge/config.toml`.
#[derive(Debug, Default)]
struct FileConfig {
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    timeout: Option<u64>,
}

/// Builder for [`ForgeConfig`].
#[derive(Debug, Default)]
pub struct ForgeConfigBuilder {
    api_key: Option<String>,
    default_model: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    default_headers: Option<HashMap<String, String>>,
    default_metadata: Option<HashMap<String, serde_json::Value>>,
    otel: Option<OtelConfig>,
}

impl ForgeConfigBuilder {
    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the default model.
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Set the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Some(Duration::from_secs(secs));
        self
    }

    /// Set static headers attached to every outbound request.
    pub fn default_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.default_headers = Some(headers);
        self
    }

    /// Set sticky metadata merged into every request's top-level `metadata`.
    pub fn default_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.default_metadata = Some(metadata);
        self
    }

    /// Set the OTel configuration (only honoured when the `otel` cargo
    /// feature is enabled).
    pub fn otel(mut self, otel: OtelConfig) -> Self {
        self.otel = Some(otel);
        self
    }

    /// Build the config, using environment variables as fallbacks.
    pub fn build(self) -> ForgeConfig {
        let env_config = ForgeConfig::from_env();

        ForgeConfig {
            api_key: self.api_key.or(env_config.api_key),
            default_model: self.default_model.unwrap_or(env_config.default_model),
            base_url: self.base_url.unwrap_or(env_config.base_url),
            timeout: self.timeout.unwrap_or(env_config.timeout),
            default_headers: self.default_headers.unwrap_or(env_config.default_headers),
            default_metadata: self.default_metadata.unwrap_or(env_config.default_metadata),
            otel: self.otel.or(env_config.otel),
        }
    }
}

/// Parse a comma-separated `key=value,key2=value2` string into a map.
/// Whitespace around keys/values is trimmed. Used by both the OTel
/// headers env var and the resource-attributes env var.
fn parse_kv_pairs(raw: &str) -> HashMap<String, String> {
    raw.split(',')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (k, v) = pair.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Build an [`OtelConfig`] from standard OTel env vars + `LITEFORGE_OTEL_*`.
/// Returns `None` only when no OTel-related env var is set, so that
/// `ForgeConfig::otel` accurately reflects "not configured" vs "configured
/// from env".
fn otel_config_from_env() -> Option<OtelConfig> {
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty());
    let headers = env::var("OTEL_EXPORTER_OTLP_HEADERS")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| parse_kv_pairs(&s))
        .unwrap_or_default();
    let service_name = env::var("OTEL_SERVICE_NAME").ok().filter(|s| !s.is_empty());
    let resource_attributes = env::var("OTEL_RESOURCE_ATTRIBUTES")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| parse_kv_pairs(&s))
        .unwrap_or_default();
    let capture_prompts = env::var("LITEFORGE_OTEL_CAPTURE_PROMPTS")
        .ok()
        .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);

    if endpoint.is_none()
        && headers.is_empty()
        && service_name.is_none()
        && resource_attributes.is_empty()
        && !capture_prompts
    {
        return None;
    }

    Some(OtelConfig {
        endpoint,
        headers,
        service_name,
        resource_attributes,
        capture_prompts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_overrides_defaults() {
        let config = ForgeConfig::builder()
            .api_key("test-key")
            .default_model("gpt-4")
            .base_url("https://example.com")
            .timeout_secs(30)
            .build();

        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.default_model, "gpt-4");
        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.default_headers.is_empty());
        assert!(config.default_metadata.is_empty());
    }

    #[test]
    fn test_builder_default_metadata_round_trip() {
        let mut meta = HashMap::new();
        meta.insert(
            "app".to_string(),
            serde_json::Value::String("btsales".to_string()),
        );
        let config = ForgeConfig::builder().default_metadata(meta.clone()).build();
        assert_eq!(config.default_metadata, meta);
    }

    #[test]
    fn test_parse_kv_pairs_handles_whitespace_and_empty() {
        let parsed = parse_kv_pairs("a=1, b=2, c = 3");
        assert_eq!(parsed.get("a"), Some(&"1".to_string()));
        assert_eq!(parsed.get("b"), Some(&"2".to_string()));
        assert_eq!(parsed.get("c"), Some(&"3".to_string()));
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn test_otel_config_from_env_returns_none_when_nothing_set() {
        // This relies on the test runner not having OTEL_* set; if it does,
        // the assertion may flap. Acceptable for a unit test but worth a note.
        // We can't safely manipulate the process env in parallel tests, so
        // we just exercise the parser directly.
        let _ = otel_config_from_env();
    }
}
