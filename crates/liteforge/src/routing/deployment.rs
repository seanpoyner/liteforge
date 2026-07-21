//! Deployment definitions for the model router.
//!
//! A [`Deployment`] is one concrete backend a logical model group can route
//! to. The router mints a per-deployment [`ForgeConfig`] via
//! [`Deployment::to_config`] so the existing transport layer is reused
//! unchanged (it derives URL and auth purely from a `&ForgeConfig`).

use crate::config::ForgeConfig;
use std::collections::HashMap;
use std::time::Duration;

/// Stable, cheap-to-copy identity for a deployment within one router.
///
/// The inner value is the index into the router's `deployments` vector,
/// which also indexes the parallel health vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeploymentId(pub usize);

/// One concrete backend a model group can route to.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Deployment {
    /// Index-based identity (assigned by the builder).
    pub id: DeploymentId,
    /// Logical name clients request (the model group).
    pub model_group: String,
    /// Real model id sent to the upstream gateway.
    pub model: String,
    /// OpenAI-compatible base URL for this deployment.
    pub base_url: String,
    /// API key for this deployment (falls back to the router base config key).
    pub api_key: Option<String>,
    /// Selection weight for weighted strategies. `0` means never picked.
    pub weight: u32,
    /// Optional requests-per-minute hint (reserved for future limit enforcement).
    pub rpm: Option<u32>,
    /// Optional tokens-per-minute hint (reserved for future limit enforcement).
    pub tpm: Option<u32>,
    /// Free-form tags for filtering / observability.
    pub tags: Vec<String>,
    /// Per-deployment request timeout override.
    pub timeout: Option<Duration>,
    /// Extra static headers merged into every request to this deployment.
    pub extra_headers: HashMap<String, String>,
}

impl Deployment {
    /// Create a deployment with the required fields and sensible defaults.
    pub fn new(
        id: DeploymentId,
        model_group: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            id,
            model_group: model_group.into(),
            model: model.into(),
            base_url: base_url.into(),
            api_key: None,
            weight: 1,
            rpm: None,
            tpm: None,
            tags: Vec::new(),
            timeout: None,
            extra_headers: HashMap::new(),
        }
    }

    /// Mint a per-deployment [`ForgeConfig`] from the router's base config.
    ///
    /// `base_url`, `default_model` and (if set) `api_key`/`timeout` are
    /// overridden for this deployment; `extra_headers` are merged into the
    /// base `default_headers`. The base `default_metadata` / `otel` settings
    /// are preserved so router traffic keeps the same observability behaviour
    /// as a plain client.
    pub(crate) fn to_config(&self, base: &ForgeConfig) -> ForgeConfig {
        let mut cfg = base.clone();
        // An empty base_url means "use the router's base config endpoint".
        if !self.base_url.is_empty() {
            cfg.base_url = self.base_url.clone();
        }
        cfg.default_model = self.model.clone();
        if self.api_key.is_some() {
            cfg.api_key = self.api_key.clone();
        }
        if let Some(t) = self.timeout {
            cfg.timeout = t;
        }
        if !self.extra_headers.is_empty() {
            for (k, v) in &self.extra_headers {
                cfg.default_headers.insert(k.clone(), v.clone());
            }
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ForgeConfig {
        ForgeConfig {
            api_key: Some("base-key".into()),
            default_model: "base-model".into(),
            base_url: "https://base/v1".into(),
            timeout: Duration::from_secs(60),
            default_headers: HashMap::new(),
            default_metadata: HashMap::new(),
            otel: None,
        }
    }

    #[test]
    fn to_config_overrides_url_and_model() {
        let d = Deployment::new(DeploymentId(0), "grp", "real-model", "https://east/v1");
        let cfg = d.to_config(&base());
        assert_eq!(cfg.base_url, "https://east/v1");
        assert_eq!(cfg.default_model, "real-model");
        // api_key falls back to base when not set on the deployment.
        assert_eq!(cfg.api_key.as_deref(), Some("base-key"));
    }

    #[test]
    fn to_config_uses_deployment_key_and_timeout_and_headers() {
        let mut d = Deployment::new(DeploymentId(1), "grp", "m", "https://west/v1");
        d.api_key = Some("dep-key".into());
        d.timeout = Some(Duration::from_secs(5));
        d.extra_headers.insert("X-Tenant".into(), "acme".into());
        let cfg = d.to_config(&base());
        assert_eq!(cfg.api_key.as_deref(), Some("dep-key"));
        assert_eq!(cfg.timeout, Duration::from_secs(5));
        assert_eq!(
            cfg.default_headers.get("X-Tenant").map(String::as_str),
            Some("acme")
        );
    }
}
