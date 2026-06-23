//! YAML configuration for the Layer-2 selectors.
//!
//! The `model_routing:` block of a router file describes the embedding model,
//! the model-group catalog, and the selector to use. [`ModelRoutingConfig::build_selector`]
//! turns it into a boxed [`ModelSelector`](crate::routing::ModelSelector) ready
//! to attach to a [`Router`](crate::routing::Router).

use super::cache::DecisionCache;
use super::embedder::{EmbeddingModelConfig, EmbeddingSource};
use super::group::{GroupCatalog, ModelGroup};
use super::mf::TierPolicy;
use super::selectors::{
    ClassifierEndpoint, MfSelector, RemoteClassifierSelector, SemanticRoute, SemanticSelector,
    StaticSelector,
};
use crate::client::AsyncForgeClient;
use crate::config::ForgeConfig;
use crate::error::{ForgeError, Result};
use crate::routing::ModelSelector;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// The full `model_routing:` configuration block.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelRoutingConfig {
    /// Embedding model used by the semantic / MF selectors.
    #[serde(default)]
    pub embedding: Option<EmbeddingModelConfig>,
    /// Model-group catalog (tiers, cost, member models).
    #[serde(default)]
    pub groups: Vec<ModelGroup>,
    /// The selector to build.
    pub selector: SelectorConfig,
}

/// Cache configuration shared by selectors.
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of cached decisions.
    pub capacity: usize,
    /// Optional time-to-live in seconds.
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

impl CacheConfig {
    fn build(&self) -> Arc<DecisionCache> {
        Arc::new(DecisionCache::new(
            self.capacity,
            self.ttl_secs.map(Duration::from_secs),
        ))
    }
}

/// What to do if a selector fails to construct (e.g. MF weights missing).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OnError {
    /// Fall back to a static (passthrough) selector.
    #[default]
    Static,
    /// Propagate the error.
    Fail,
}

/// A route for the semantic selector.
#[derive(Debug, Clone, Deserialize)]
pub struct SemanticRouteConfig {
    /// Target model group.
    pub group: String,
    /// Example utterances.
    pub utterances: Vec<String>,
}

/// How a remote classifier is reached.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EndpointConfig {
    /// A LiteLLM chat model returning JSON.
    Chat {
        /// The classifier model id.
        model: String,
        /// Forward the full request messages (codebase context) to the classifier
        /// instead of a JSON-instruction prompt. Default false.
        #[serde(default)]
        forward_messages: bool,
    },
    /// A custom HTTP path.
    Custom {
        /// Path relative to the client base URL.
        path: String,
    },
}

fn default_threshold() -> f32 {
    0.3
}

/// The selector variant and its parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectorConfig {
    /// Passthrough.
    Static,
    /// Semantic similarity routing.
    Semantic {
        /// Fallback group when nothing matches.
        #[serde(default)]
        default_group: Option<String>,
        /// Minimum cosine score to count as a match.
        #[serde(default = "default_threshold")]
        threshold: f32,
        /// Category routes.
        routes: Vec<SemanticRouteConfig>,
        /// Optional decision cache.
        #[serde(default)]
        cache: Option<CacheConfig>,
    },
    /// MF quality routing.
    Mf {
        /// Path to the JSON weights file.
        weights_path: String,
        /// Tier bucketing policy.
        #[serde(default)]
        tier_policy: TierPolicy,
        /// Optional decision cache.
        #[serde(default)]
        cache: Option<CacheConfig>,
        /// Behaviour when weights are missing/invalid.
        #[serde(default)]
        on_error: OnError,
    },
    /// Remote classifier routing.
    RemoteClassifier {
        /// Classifier endpoint.
        endpoint: EndpointConfig,
        /// Map of classifier labels to model groups.
        label_to_group: HashMap<String, String>,
        /// Optional decision cache.
        #[serde(default)]
        cache: Option<CacheConfig>,
    },
}

impl ModelRoutingConfig {
    /// Parse the optional `model_routing:` block from a full router YAML.
    pub fn parse_optional(yaml: &str) -> Result<Option<ModelRoutingConfig>> {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default)]
            model_routing: Option<ModelRoutingConfig>,
        }
        let w: Wrapper = serde_yaml::from_str(yaml)
            .map_err(|e| ForgeError::config(format!("invalid model_routing YAML: {e}")))?;
        Ok(w.model_routing)
    }

    /// The model-group catalog.
    pub fn catalog(&self) -> GroupCatalog {
        GroupCatalog::new(self.groups.clone())
    }

    fn embedder(&self) -> Result<Arc<EmbeddingSource>> {
        let cfg = self.embedding.as_ref().ok_or_else(|| {
            ForgeError::config("model_routing.embedding is required for this selector")
        })?;
        // Allow overriding the embedding endpoint from the environment.
        let mut cfg = cfg.clone();
        if let Ok(url) = std::env::var("FORGE_ROUTER_EMBEDDING_BASE_URL") {
            if !url.is_empty() {
                cfg.base_url = url;
            }
        }
        Ok(Arc::new(EmbeddingSource::new(&cfg)?))
    }

    fn classifier_client(&self) -> AsyncForgeClient {
        let mut fc = ForgeConfig::from_env();
        if let Some(emb) = &self.embedding {
            fc.base_url = emb.base_url.clone();
            if let Some(k) = &emb.api_key {
                fc.api_key = Some(k.clone());
            }
        }
        AsyncForgeClient::with_config(fc)
    }

    /// Build a boxed selector from this config.
    pub async fn build_selector(&self) -> Result<Box<dyn ModelSelector>> {
        match &self.selector {
            SelectorConfig::Static => Ok(Box::new(StaticSelector::new())),

            SelectorConfig::Semantic {
                default_group,
                threshold,
                routes,
                cache,
            } => {
                let embedder = self.embedder()?;
                let routes = routes
                    .iter()
                    .map(|r| SemanticRoute::new(&r.group, r.utterances.clone()))
                    .collect();
                let mut sel =
                    SemanticSelector::build(embedder, routes, default_group.clone(), *threshold)
                        .await?;
                if let Some(c) = cache {
                    sel = sel.with_cache(c.build());
                }
                Ok(Box::new(sel))
            }

            SelectorConfig::Mf {
                weights_path,
                tier_policy,
                cache,
                on_error,
            } => {
                let embedder = self.embedder()?;
                // FORGE_ROUTER_WEIGHTS overrides the configured weights path.
                let resolved_path = std::env::var("FORGE_ROUTER_WEIGHTS")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| weights_path.clone());
                let built = MfSelector::from_file(
                    &resolved_path,
                    embedder,
                    tier_policy.clone(),
                    self.catalog(),
                );
                match built {
                    Ok(mut sel) => {
                        if let Some(c) = cache {
                            sel = sel.with_cache(c.build());
                        }
                        Ok(Box::new(sel))
                    }
                    Err(e) => match on_error {
                        OnError::Static => {
                            tracing::warn!("MF selector unavailable ({e}); using static selector");
                            Ok(Box::new(StaticSelector::new()))
                        }
                        OnError::Fail => Err(e),
                    },
                }
            }

            SelectorConfig::RemoteClassifier {
                endpoint,
                label_to_group,
                cache,
            } => {
                let client = self.classifier_client();
                let endpoint = match endpoint {
                    EndpointConfig::Chat {
                        model,
                        forward_messages,
                    } => ClassifierEndpoint::Chat {
                        model: model.clone(),
                        forward_messages: *forward_messages,
                    },
                    EndpointConfig::Custom { path } => {
                        ClassifierEndpoint::Custom { path: path.clone() }
                    }
                };
                let mut sel =
                    RemoteClassifierSelector::new(client, endpoint, label_to_group.clone());
                if let Some(c) = cache {
                    sel = sel.with_cache(c.build());
                }
                Ok(Box::new(sel))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_optional_returns_none_when_absent() {
        let yaml = "model_list: []\n";
        assert!(ModelRoutingConfig::parse_optional(yaml).unwrap().is_none());
    }

    #[test]
    fn parses_mf_selector_block() {
        let yaml = r#"
model_routing:
  embedding:
    base_url: https://litellm.poyner.ai/v1
    model: bge-m3
    dimensions: 1024
  groups:
    - { name: cheap, tier: Small }
    - { name: premium, tier: Frontier }
  selector:
    kind: mf
    weights_path: /tmp/mf.json
    tier_policy: { thresholds: [0.5] }
    on_error: static
"#;
        let cfg = ModelRoutingConfig::parse_optional(yaml).unwrap().unwrap();
        assert_eq!(cfg.groups.len(), 2);
        match &cfg.selector {
            SelectorConfig::Mf {
                weights_path,
                on_error,
                ..
            } => {
                assert_eq!(weights_path, "/tmp/mf.json");
                assert!(matches!(on_error, OnError::Static));
            }
            _ => panic!("expected MF selector"),
        }
    }

    #[tokio::test]
    async fn mf_missing_weights_falls_back_to_static() {
        let yaml = r#"
model_routing:
  embedding: { base_url: http://x/v1, model: bge-m3, dimensions: 1024 }
  groups:
    - { name: cheap, tier: Small }
  selector:
    kind: mf
    weights_path: /nonexistent/mf.json
    on_error: static
"#;
        let cfg = ModelRoutingConfig::parse_optional(yaml).unwrap().unwrap();
        let sel = cfg.build_selector().await.unwrap();
        assert_eq!(sel.name(), "static");
    }
}
