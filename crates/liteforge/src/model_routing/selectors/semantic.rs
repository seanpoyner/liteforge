//! Semantic selector: route by content category via embedding similarity.
//!
//! Each route maps a set of example utterances to a target group. At build time
//! the utterances are embedded and averaged into a per-group centroid; at query
//! time the prompt embedding is compared (cosine) against every centroid and the
//! best-matching groups above a threshold are returned. This is natively N-way.

use crate::error::Result;
use crate::model_routing::cache::{decision_key, DecisionCache};
use crate::model_routing::embedder::EmbeddingSource;
use crate::rag::{cosine_similarity, normalize};
use crate::routing::{ModelSelector, ScoredGroup, SelectionContext};
use async_trait::async_trait;
use std::sync::Arc;

/// One semantic route: a category of prompts mapped to a model group.
#[derive(Debug, Clone)]
pub struct SemanticRoute {
    /// Target model group.
    pub group: String,
    /// Example utterances representing this category.
    pub utterances: Vec<String>,
}

impl SemanticRoute {
    /// Create a route.
    pub fn new(group: impl Into<String>, utterances: Vec<String>) -> Self {
        Self {
            group: group.into(),
            utterances,
        }
    }
}

/// Embedding-similarity selector over per-group centroids.
pub struct SemanticSelector {
    embedder: Arc<EmbeddingSource>,
    centroids: Vec<(String, Vec<f32>)>,
    default_group: Option<String>,
    threshold: f32,
    cache: Option<Arc<DecisionCache>>,
    catalog_sig: String,
}

impl SemanticSelector {
    /// Build a semantic selector, embedding each route's utterances into a
    /// normalized centroid. Returns an error if a route has no utterances.
    pub async fn build(
        embedder: Arc<EmbeddingSource>,
        routes: Vec<SemanticRoute>,
        default_group: Option<String>,
        threshold: f32,
    ) -> Result<Self> {
        let mut centroids = Vec::with_capacity(routes.len());
        for route in &routes {
            if route.utterances.is_empty() {
                return Err(crate::error::ForgeError::config(format!(
                    "semantic route for group '{}' has no utterances",
                    route.group
                )));
            }
            let dim = embedder.dimensions() as usize;
            let mut acc = vec![0.0f32; dim];
            for u in &route.utterances {
                let v = embedder.embed(u).await?;
                let vn = normalize(&v);
                for (a, x) in acc.iter_mut().zip(vn.iter()) {
                    *a += x;
                }
            }
            let centroid = normalize(&acc);
            centroids.push((route.group.clone(), centroid));
        }
        let catalog_sig = centroids
            .iter()
            .map(|(g, _)| g.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Ok(Self {
            embedder,
            centroids,
            default_group,
            threshold,
            cache: None,
            catalog_sig,
        })
    }

    /// Attach a decision cache.
    pub fn with_cache(mut self, cache: Arc<DecisionCache>) -> Self {
        self.cache = Some(cache);
        self
    }
}

#[async_trait]
impl ModelSelector for SemanticSelector {
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
        let text = ctx.prompt_text();
        let key = decision_key("semantic", &text, &self.catalog_sig);
        if let Some(cache) = &self.cache {
            if let Some(hit) = cache.get(key) {
                return Ok(hit);
            }
        }

        let q = self.embedder.embed(&text).await?;
        let mut scored: Vec<ScoredGroup> = self
            .centroids
            .iter()
            .map(|(group, centroid)| {
                let sim = cosine_similarity(&q, centroid);
                ScoredGroup::new(group, sim).with_reason(format!("semantic cosine {sim:.3}"))
            })
            .filter(|sg| sg.score >= self.threshold)
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Always offer the default group as a tail fallback if configured.
        if let Some(def) = &self.default_group {
            if !scored.iter().any(|sg| &sg.group == def) {
                scored.push(ScoredGroup::new(def, 0.0).with_reason("semantic default"));
            }
        }

        if let Some(cache) = &self.cache {
            cache.put(key, scored.clone());
        }
        Ok(scored)
    }

    fn name(&self) -> &str {
        "semantic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatCompletionRequest, Message};

    // A deterministic 3-dim mock embedding: maps a few keywords to basis-ish
    // vectors so we can assert routing without a network.
    fn mock_embedder() -> Arc<EmbeddingSource> {
        Arc::new(EmbeddingSource::mock(3, |t| {
            let t = t.to_lowercase();
            if t.contains("code") {
                vec![1.0, 0.0, 0.0]
            } else if t.contains("poem") {
                vec![0.0, 1.0, 0.0]
            } else {
                vec![0.0, 0.0, 1.0]
            }
        }))
    }

    #[tokio::test]
    async fn routes_prompt_to_matching_category() {
        let embedder = mock_embedder();
        let sel = SemanticSelector::build(
            embedder,
            vec![
                SemanticRoute::new("coder", vec!["write code".into()]),
                SemanticRoute::new("writer", vec!["write a poem".into()]),
            ],
            Some("writer".into()),
            0.5,
        )
        .await
        .unwrap();

        let req = ChatCompletionRequest::new("auto", vec![Message::user("help me code this")]);
        let ranked = sel.select(&SelectionContext::new(&req)).await.unwrap();
        assert_eq!(ranked[0].group, "coder");
    }

    #[tokio::test]
    async fn unmatched_prompt_falls_back_to_default() {
        let embedder = mock_embedder();
        let sel = SemanticSelector::build(
            embedder,
            vec![SemanticRoute::new("coder", vec!["write code".into()])],
            Some("general".into()),
            0.5,
        )
        .await
        .unwrap();

        // "hello" maps to [0,0,1], orthogonal to the coder centroid [1,0,0].
        let req = ChatCompletionRequest::new("auto", vec![Message::user("hello there")]);
        let ranked = sel.select(&SelectionContext::new(&req)).await.unwrap();
        assert_eq!(ranked.last().unwrap().group, "general");
    }
}
