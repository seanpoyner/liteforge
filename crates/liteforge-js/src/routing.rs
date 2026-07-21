//! JavaScript/TypeScript bindings for native model routing (config-driven).

use liteforge::model_routing::ModelRoutingConfig;
use liteforge::routing::Router as CoreRouter;
use liteforge::{ChatCompletionRequest, Message as RustMessage};
use napi::bindgen_prelude::*;
use std::sync::Arc;

fn to_napi<E: std::fmt::Display>(e: E) -> Error {
    Error::from_reason(e.to_string())
}

/// The outcome of a routing decision.
#[napi(object)]
pub struct JsRouteDecision {
    /// Chosen model group.
    pub group: String,
    /// Concrete model id within the group.
    pub model: String,
    /// Base URL of the chosen deployment.
    pub base_url: String,
    /// Selection strategy name.
    pub strategy: String,
    /// Selector score for the chosen group, if a selector ran.
    pub score: Option<f64>,
    /// Ordered groups the router would try (incl. fallbacks).
    pub fallback_chain: Vec<String>,
}

/// A model router built from a LiteLLM-compatible YAML config.
#[napi]
pub struct Router {
    inner: Arc<CoreRouter>,
}

async fn build_router(yaml: String) -> Result<Router> {
    let mut router = CoreRouter::from_yaml_str(&yaml).map_err(to_napi)?;
    if let Some(mr) = ModelRoutingConfig::parse_optional(&yaml).map_err(to_napi)? {
        let selector = mr.build_selector().await.map_err(to_napi)?;
        router = router.with_selector(Arc::from(selector));
    }
    Ok(Router {
        inner: Arc::new(router),
    })
}

#[napi]
impl Router {
    /// Build a router from a YAML string.
    #[napi(factory)]
    pub async fn from_yaml(yaml: String) -> Result<Router> {
        build_router(yaml).await
    }

    /// Build a router from a YAML file path.
    #[napi(factory)]
    pub async fn from_file(path: String) -> Result<Router> {
        let yaml = std::fs::read_to_string(&path).map_err(to_napi)?;
        build_router(yaml).await
    }

    /// The concrete model id a prompt would route to.
    #[napi]
    pub async fn which_model(&self, prompt: String) -> Result<String> {
        self.inner.which_model(prompt).await.map_err(to_napi)
    }

    /// The full routing decision for a prompt.
    #[napi]
    pub async fn route(&self, prompt: String) -> Result<JsRouteDecision> {
        let req = ChatCompletionRequest::new("auto", vec![RustMessage::user(prompt)]);
        let d = self.inner.route_decision(&req).await.map_err(to_napi)?;
        Ok(JsRouteDecision {
            group: d.group,
            model: d.model,
            base_url: d.base_url,
            strategy: d.strategy,
            score: d.score.map(|s| s as f64),
            fallback_chain: d.fallback_chain,
        })
    }

    /// The model group names this router serves.
    #[napi]
    pub fn model_groups(&self) -> Vec<String> {
        self.inner
            .model_groups()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// The load-balancing strategy name.
    #[napi]
    pub fn strategy(&self) -> String {
        self.inner.strategy_name().to_string()
    }
}
