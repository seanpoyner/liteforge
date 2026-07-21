//! MF quality selector: native RouteLLM matrix-factorization routing.

use crate::error::Result;
use crate::model_routing::cache::{decision_key, DecisionCache};
use crate::model_routing::embedder::EmbeddingSource;
use crate::model_routing::group::GroupCatalog;
use crate::model_routing::mf::{mf_hardness, MfWeights, TierPolicy};
use crate::routing::{ModelSelector, ScoredGroup, SelectionContext};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Routes by predicted prompt difficulty using a trained MF model, mapping the
/// scalar hardness onto the catalog's capability tiers.
pub struct MfSelector {
    embedder: Arc<EmbeddingSource>,
    weights: MfWeights,
    policy: TierPolicy,
    catalog: GroupCatalog,
    cache: Option<Arc<DecisionCache>>,
    catalog_sig: String,
}

impl MfSelector {
    /// Build an MF selector from in-memory weights.
    pub fn new(
        embedder: Arc<EmbeddingSource>,
        weights: MfWeights,
        policy: TierPolicy,
        catalog: GroupCatalog,
    ) -> Self {
        let catalog_sig = catalog.names().join(",");
        Self {
            embedder,
            weights,
            policy,
            catalog,
            cache: None,
            catalog_sig,
        }
    }

    /// Build an MF selector by loading weights from a JSON file.
    pub fn from_file(
        weights_path: impl AsRef<Path>,
        embedder: Arc<EmbeddingSource>,
        policy: TierPolicy,
        catalog: GroupCatalog,
    ) -> Result<Self> {
        let weights = MfWeights::load(weights_path)?;
        // The embedding model must match what MF was trained against.
        if weights.text_dim != embedder.dimensions() as usize {
            return Err(crate::error::ForgeError::config(format!(
                "MF weights text_dim {} != embedding dimensions {}",
                weights.text_dim,
                embedder.dimensions()
            )));
        }
        Ok(Self::new(embedder, weights, policy, catalog))
    }

    /// Attach a decision cache.
    pub fn with_cache(mut self, cache: Arc<DecisionCache>) -> Self {
        self.cache = Some(cache);
        self
    }
}

#[async_trait]
impl ModelSelector for MfSelector {
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
        let text = ctx.prompt_text();
        let key = decision_key("mf", &text, &self.catalog_sig);
        if let Some(cache) = &self.cache {
            if let Some(hit) = cache.get(key) {
                return Ok(hit);
            }
        }
        let e = self.embedder.embed(&text).await?;
        let s = mf_hardness(&self.weights, &e)?;
        let ranked = self.policy.rank(s, &self.catalog);
        if let Some(cache) = &self.cache {
            cache.put(key, ranked.clone());
        }
        Ok(ranked)
    }

    fn name(&self) -> &str {
        "mf"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_routing::group::{CapabilityTier, ModelGroup};
    use crate::model_routing::mf::weights::MF_WEIGHTS_VERSION;
    use crate::types::{ChatCompletionRequest, Message};

    fn weights() -> MfWeights {
        MfWeights {
            version: MF_WEIGHTS_VERSION,
            embedding_model: "mock".into(),
            text_dim: 2,
            d: 2,
            num_classes: 2,
            strong_row: vec![1.0, 0.0],
            weak_row: vec![0.0, 1.0],
            use_proj: false,
            proj_w: None,
            proj_b: None,
            cls_w: vec![1.0, 0.0, 0.0, 1.0],
            cls_b: vec![0.0, 0.0],
            strong_class: 0,
            weak_class: 1,
        }
    }

    fn catalog() -> GroupCatalog {
        GroupCatalog::new(vec![
            ModelGroup::new("cheap", CapabilityTier::Small),
            ModelGroup::new("premium", CapabilityTier::Frontier),
        ])
    }

    #[tokio::test]
    async fn hard_prompt_routes_to_premium() {
        // Embedding heavy on dim 0 (strong) -> high hardness.
        let embedder = Arc::new(EmbeddingSource::mock(2, |_t| vec![5.0, 0.0]));
        let sel = MfSelector::new(embedder, weights(), TierPolicy::new(vec![0.5]), catalog());
        let req = ChatCompletionRequest::new("auto", vec![Message::user("hard")]);
        let ranked = sel.select(&SelectionContext::new(&req)).await.unwrap();
        assert_eq!(ranked[0].group, "premium");
    }

    #[tokio::test]
    async fn easy_prompt_routes_to_cheap() {
        // Embedding heavy on dim 1 (weak) -> low hardness.
        let embedder = Arc::new(EmbeddingSource::mock(2, |_t| vec![0.0, 5.0]));
        let sel = MfSelector::new(embedder, weights(), TierPolicy::new(vec![0.5]), catalog());
        let req = ChatCompletionRequest::new("auto", vec![Message::user("easy")]);
        let ranked = sel.select(&SelectionContext::new(&req)).await.unwrap();
        assert_eq!(ranked[0].group, "cheap");
    }
}
