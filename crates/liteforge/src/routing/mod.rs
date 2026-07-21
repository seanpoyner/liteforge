//! Native model routing (Layer 1): LiteLLM-style load balancing.
//!
//! A [`Router`] fronts many [`Deployment`]s grouped by a logical model name and
//! load-balances across them with a pluggable [`RoutingStrategy`], per-deployment
//! health tracking, cooldowns after repeated failures, model-group fallbacks, and
//! a cross-deployment retry budget. It reuses the SDK's existing HTTP transport
//! unchanged by minting a per-deployment [`ForgeConfig`](crate::ForgeConfig).
//!
//! An optional Layer-2 [`ModelSelector`] (implemented in the
//! [`model_routing`](crate::model_routing) module) can pick *which* model group a
//! request targets, based on the prompt content/difficulty. The router then load
//! balances within the chosen group and falls back down the selector's ranking.
//!
//! # Example
//!
//! ```no_run
//! use liteforge::routing::{Router, RoutingStrategy};
//! use liteforge::{ChatCompletionRequest, Message};
//!
//! # async fn example() -> liteforge::Result<()> {
//! let router = Router::builder()
//!     .strategy(RoutingStrategy::LatencyBased)
//!     .add_deployment("premium", "gpt-5.2-pro", "https://gw-a/v1")
//!     .add_deployment("premium", "gpt-5.2-pro", "https://gw-b/v1")
//!     .add_deployment("cheap", "claude-haiku-4.5", "https://gw/v1")
//!     .fallback("premium", vec!["cheap".into()])
//!     .build()?;
//!
//! let req = ChatCompletionRequest::new("premium", vec![Message::user("hello")]);
//! let resp = router.chat_completions(req).await?;
//! # let _ = resp; Ok(())
//! # }
//! ```

pub mod config;
pub mod deployment;
pub mod health;
pub mod router;
pub mod selector;
pub mod strategy;

pub use config::{LiteLlmParams, ModelEntry, ModelInfo, RouterFileConfig, RouterSettingsFile};
pub use deployment::{Deployment, DeploymentId};
pub use health::{DeploymentHealth, HealthSnapshot, InFlightGuard, OwnedInFlightGuard};
pub use router::{Router, RouterBuilder, RouterSettings};
pub use selector::{ModelSelector, RouteDecision, ScoredGroup, SelectionContext};
pub use strategy::{
    Candidate, LatencyBased, LeastBusy, RoundRobin, RoutingStrategy, SelectionStrategy,
    SimpleShuffle,
};
