//! LiteLLM-compatible YAML configuration for the router.
//!
//! The schema mirrors LiteLLM's `model_list` / `router_settings` so existing
//! configs port across. A LiteForge router file may also carry a
//! `model_routing:` block (parsed by the `model_routing` module); unknown
//! top-level keys are ignored here.

use super::deployment::{Deployment, DeploymentId};
use super::strategy::RoutingStrategy;
use crate::error::{ForgeError, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// Top-level router configuration (the `model_list` + `router_settings` parts).
#[derive(Debug, Clone, Deserialize)]
pub struct RouterFileConfig {
    /// Deployment entries.
    pub model_list: Vec<ModelEntry>,
    /// Router-wide settings.
    #[serde(default)]
    pub router_settings: RouterSettingsFile,
}

/// One `model_list` entry (a deployment).
#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    /// Logical group name clients request.
    pub model_name: String,
    /// Provider params for this deployment.
    pub litellm_params: LiteLlmParams,
    /// Optional extra info (tags, etc.).
    #[serde(default)]
    pub model_info: Option<ModelInfo>,
}

/// Provider params, mirroring LiteLLM's `litellm_params`.
#[derive(Debug, Clone, Deserialize)]
pub struct LiteLlmParams {
    /// Real model id sent upstream.
    pub model: String,
    /// OpenAI-compatible base URL (supports `os.environ/VAR`).
    #[serde(default)]
    pub api_base: Option<String>,
    /// API key (supports `os.environ/VAR`).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Weight for weighted strategies.
    #[serde(default)]
    pub weight: Option<u32>,
    /// Requests-per-minute hint.
    #[serde(default)]
    pub rpm: Option<u32>,
    /// Tokens-per-minute hint.
    #[serde(default)]
    pub tpm: Option<u32>,
    /// Per-deployment timeout (e.g. `30s`).
    #[serde(default, with = "humantime_serde")]
    pub timeout: Option<Duration>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Optional `model_info` block.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelInfo {
    /// Tags for filtering / observability.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Router-wide settings.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RouterSettingsFile {
    /// Load-balancing strategy.
    #[serde(default)]
    pub routing_strategy: Option<RoutingStrategy>,
    /// Failures before a deployment is cooled down.
    #[serde(default)]
    pub allowed_fails: Option<u32>,
    /// Cooldown duration (e.g. `60s`).
    #[serde(default, with = "humantime_serde")]
    pub cooldown_time: Option<Duration>,
    /// Total cross-deployment retry budget per request.
    #[serde(default)]
    pub num_retries: Option<u32>,
    /// EWMA smoothing factor for latency-based routing (0..1).
    #[serde(default)]
    pub ewma_alpha: Option<f64>,
    /// As-last-resort, ignore cooldowns rather than fail the request.
    #[serde(default)]
    pub allow_cooled_fallback: Option<bool>,
    /// Model-group fallbacks: list of `{ group: [fallback, ...] }` maps
    /// (LiteLLM's list-of-maps shape).
    #[serde(default)]
    pub fallbacks: Vec<HashMap<String, Vec<String>>>,
}

/// Resolve a `os.environ/NAME` reference against the process environment.
/// Non-reference values pass through unchanged. Missing env vars yield `None`.
pub(crate) fn resolve_env(value: &str) -> Option<String> {
    if let Some(name) = value.strip_prefix("os.environ/") {
        std::env::var(name).ok().filter(|s| !s.is_empty())
    } else {
        Some(value.to_string())
    }
}

impl RouterFileConfig {
    /// Parse a router config from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| ForgeError::config(format!("invalid router YAML: {e}")))
    }

    /// Flatten the `fallbacks` list-of-maps into a single map.
    pub(crate) fn fallback_map(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for entry in &self.router_settings.fallbacks {
            for (k, v) in entry {
                out.entry(k.clone()).or_insert_with(Vec::new).extend(v.clone());
            }
        }
        out
    }

    /// Build the runtime deployment list, resolving env references and
    /// assigning sequential [`DeploymentId`]s.
    pub(crate) fn deployments(&self) -> Result<Vec<Deployment>> {
        let mut out = Vec::with_capacity(self.model_list.len());
        for (i, entry) in self.model_list.iter().enumerate() {
            let p = &entry.litellm_params;
            let base_url = match &p.api_base {
                Some(b) => resolve_env(b).ok_or_else(|| {
                    ForgeError::config(format!(
                        "api_base env reference unset for model_name '{}'",
                        entry.model_name
                    ))
                })?,
                None => String::new(), // filled from the router base config later
            };
            let api_key = match &p.api_key {
                Some(k) => resolve_env(k),
                None => None,
            };
            let mut tags = p.tags.clone();
            if let Some(info) = &entry.model_info {
                tags.extend(info.tags.clone());
            }
            let mut d = Deployment::new(DeploymentId(i), &entry.model_name, &p.model, base_url);
            d.api_key = api_key;
            d.weight = p.weight.unwrap_or(1);
            d.rpm = p.rpm;
            d.tpm = p.tpm;
            d.timeout = p.timeout;
            d.tags = tags;
            out.push(d);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
model_list:
  - model_name: premium
    litellm_params:
      model: gpt-real
      api_base: https://east/v1
      api_key: os.environ/ROUTER_TEST_KEY
      weight: 2
      timeout: 30s
  - model_name: cheap
    litellm_params:
      model: haiku-real
      api_base: https://gw/v1
router_settings:
  routing_strategy: latency-based-routing
  allowed_fails: 5
  cooldown_time: 90s
  num_retries: 4
  fallbacks:
    - premium: [cheap]
model_routing:
  ignored: true
"#;

    #[test]
    fn parses_model_list_and_settings_ignoring_extra_keys() {
        std::env::set_var("ROUTER_TEST_KEY", "sekret");
        let cfg = RouterFileConfig::from_yaml_str(YAML).expect("parse");
        assert_eq!(cfg.model_list.len(), 2);
        assert_eq!(
            cfg.router_settings.routing_strategy,
            Some(RoutingStrategy::LatencyBased)
        );
        assert_eq!(cfg.router_settings.allowed_fails, Some(5));
        assert_eq!(cfg.router_settings.cooldown_time, Some(Duration::from_secs(90)));

        let deps = cfg.deployments().expect("deployments");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].model_group, "premium");
        assert_eq!(deps[0].model, "gpt-real");
        assert_eq!(deps[0].weight, 2);
        assert_eq!(deps[0].timeout, Some(Duration::from_secs(30)));
        assert_eq!(deps[0].api_key.as_deref(), Some("sekret"));

        let fb = cfg.fallback_map();
        assert_eq!(fb.get("premium"), Some(&vec!["cheap".to_string()]));
        std::env::remove_var("ROUTER_TEST_KEY");
    }
}
