//! Native model routing (Layer 2): content/quality model selection.
//!
//! These selectors implement the [`ModelSelector`](crate::routing::ModelSelector)
//! seam from the Layer-1 [`routing`](crate::routing) module. They decide *which*
//! model group a request should target, based on the prompt itself:
//!
//! - [`StaticSelector`] - passthrough (use the requested model).
//! - [`SemanticSelector`] - route by content category via embedding similarity
//!   (LiteLLM auto-router style). Natively N-way.
//! - [`MfSelector`] - a native Rust port of RouteLLM's matrix-factorization
//!   quality router: predict prompt difficulty and bucket it across capability
//!   tiers. N-way via [`TierPolicy`].
//! - [`RemoteClassifierSelector`] - call a BERT/causal classifier served behind
//!   LiteLLM and map its labels to groups.
//!
//! All selectors fetch embeddings over HTTP ([`EmbeddingSource`]) and do no local
//! ML inference; the MF forward pass is plain linear algebra. A [`DecisionCache`]
//! keeps repeated decisions off the network hot path.
//!
//! # Example
//!
//! ```no_run
//! use liteforge::model_routing::{ModelRoutingConfig};
//! use liteforge::routing::Router;
//! use std::sync::Arc;
//!
//! # async fn example(yaml: &str) -> liteforge::Result<()> {
//! let router = Router::from_yaml_str(yaml)?;
//! if let Some(mr) = ModelRoutingConfig::parse_optional(yaml)? {
//!     let selector = mr.build_selector().await?;
//!     let router = router.with_selector(Arc::from(selector));
//!     let _ = router;
//! }
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod config;
pub mod embedder;
pub mod group;
pub mod mf;
pub mod selectors;

pub use cache::{decision_key, DecisionCache};
pub use config::{
    CacheConfig, EndpointConfig, ModelRoutingConfig, OnError, SelectorConfig, SemanticRouteConfig,
};
pub use embedder::{EmbeddingModelConfig, EmbeddingSource};
pub use group::{CapabilityTier, GroupCatalog, ModelGroup};
pub use mf::{MfWeights, TierDirection, TierPolicy};
pub use selectors::{
    ClassifierEndpoint, ClassifierResponse, MfSelector, RemoteClassifierSelector, SemanticRoute,
    SemanticSelector, StaticSelector,
};
