//! Embedding-head selector: route over frozen bge-m3 embeddings with a tiny learned
//! head, the design our RouterBench study found is the only small router that beats
//! random. One embedding call, then a couple of matrix multiplies.
//!
//! The head spec (`router-head.json`, produced by `scripts/eval/train_router_head.py`)
//! carries a quality head (predicts route-to-strong propensity) and a task head
//! (qa/code/math), each as a stack of dense layers. We fuse the quality "hardness",
//! the task class, and structured codebase-context features into capability groups.

use crate::error::{ForgeError, Result};
use crate::model_routing::cache::{decision_key, DecisionCache};
use crate::model_routing::embedder::EmbeddingSource;
use crate::model_routing::features::{extract_features, norm_struct, Features};
use crate::routing::{ModelSelector, ScoredGroup, SelectionContext};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;

/// One dense layer: `out = act(W x + b)`, `W` row-major `[out][in]`.
#[derive(Debug, Clone, Deserialize)]
pub struct Layer {
    #[serde(rename = "W")]
    pub w: Vec<Vec<f32>>,
    pub b: Vec<f32>,
    pub activation: String,
}

impl Layer {
    fn forward(&self, x: &[f32]) -> Vec<f32> {
        let mut out: Vec<f32> = self
            .w
            .iter()
            .zip(self.b.iter())
            .map(|(row, bias)| row.iter().zip(x).map(|(a, b)| a * b).sum::<f32>() + bias)
            .collect();
        if self.activation == "relu" {
            for v in &mut out {
                if *v < 0.0 {
                    *v = 0.0;
                }
            }
        }
        out
    }
}

/// A small classifier head (stack of dense layers + class labels).
#[derive(Debug, Clone, Deserialize)]
pub struct Head {
    pub classes: Vec<String>,
    pub layers: Vec<Layer>,
}

impl Head {
    fn probs(&self, x: &[f32]) -> Vec<f32> {
        let mut v = x.to_vec();
        for layer in &self.layers {
            v = layer.forward(&v);
        }
        softmax(&v)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct StructSpec {
    #[allow(dead_code)]
    features: Vec<String>,
}

/// Parsed `router-head.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct HeadSpec {
    pub embedding_model: String,
    pub text_dim: usize,
    pub use_struct: bool,
    #[serde(default)]
    struct_: Option<StructSpec>,
    pub quality: Head,
    pub task: Head,
    #[serde(default)]
    pub groups: Vec<String>,
}

impl HeadSpec {
    /// Load and validate a head spec from JSON.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| {
            ForgeError::config(format!("could not read head spec {:?}: {e}", path.as_ref()))
        })?;
        let spec: HeadSpec = serde_json::from_slice(&bytes)?;
        let _ = &spec.struct_;
        Ok(spec)
    }
}

fn softmax(v: &[f32]) -> Vec<f32> {
    let m = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = v.iter().map(|x| (x - m).exp()).collect();
    let s: f32 = exps.iter().sum();
    if s > 0.0 {
        exps.iter().map(|x| x / s).collect()
    } else {
        vec![1.0 / v.len() as f32; v.len()]
    }
}

/// Quality head -> scalar route-to-strong "hardness" in [0,1].
fn hardness(quality: &Head, probs: &[f32]) -> f32 {
    let idx = |name: &str| quality.classes.iter().position(|c| c == name);
    // tier3: P(strong) + 0.5 P(mid)
    if let Some(si) = idx("strong") {
        let mut h = probs[si];
        if let Some(mi) = idx("mid") {
            h += 0.5 * probs[mi];
        }
        return h.clamp(0.0, 1.0);
    }
    // binary: classes "0"/"1" where "1" = weak correct -> hardness = P(weak wrong) = P("0")
    if let Some(zero) = idx("0") {
        return probs[zero].clamp(0.0, 1.0);
    }
    // fallback: probability mass on the last (highest) class
    *probs.last().unwrap_or(&0.5)
}

/// Fuse the signals into a ranked capability-group list. Policy: the difficulty axis
/// (hardness) is benchmark-validated; the task / context / triviality axes are
/// interpretable policy (the quality head is RouterBench-trained, so it has no notion
/// of trivial chit-chat: a cheap triviality rule covers that).
fn fuse(hardness: f32, task: &str, f: &Features, ctx_tokens_norm: f32) -> Vec<ScoredGroup> {
    let context_high = f.n_files >= 4 || ctx_tokens_norm >= 0.6;
    let is_code = matches!(task, "code") || f.has_code || f.has_diff;
    let is_qa = matches!(task, "qa");
    // Very short, code-free prompts are trivial chat (greetings, thanks, tiny asks).
    let trivial = f.ctx_tokens < 8 && !f.has_code && !f.has_diff && !f.has_error;

    // chat only for clearly-trivial prompts: the RouterBench-trained quality head is
    // unreliable on open-ended/agentic prompts, so we do NOT send merely-low-hardness
    // work to the cheapest tier; ambiguous prompts default to `general` (mid tier).
    let _ = is_qa;
    let code = if is_code { 0.9 } else { 0.1 };
    let reasoning = if hardness > 0.6 { 0.55 + 0.45 * hardness } else { 0.1 * hardness };
    let chat = if trivial { 0.95 } else { 0.1 };
    let long_context = if context_high { 0.82 } else { 0.1 };
    let general = 0.5;

    let mut groups = vec![
        ScoredGroup::new("code", code).with_reason(format!("task={task}")),
        ScoredGroup::new("reasoning", reasoning).with_reason(format!("hardness={hardness:.3}")),
        ScoredGroup::new("chat", chat).with_reason("trivial/easy qa"),
        ScoredGroup::new("long_context", long_context).with_reason(format!("files={}", f.n_files)),
        ScoredGroup::new("general", general).with_reason("default"),
    ];
    groups.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    groups
}

/// Routes over bge-m3 embeddings with a learned quality + task head.
pub struct EmbeddingHeadSelector {
    embedder: Arc<EmbeddingSource>,
    spec: HeadSpec,
    cache: Option<Arc<DecisionCache>>,
    sig: String,
}

impl EmbeddingHeadSelector {
    /// Build from an in-memory spec.
    pub fn new(embedder: Arc<EmbeddingSource>, spec: HeadSpec) -> Self {
        let sig = format!("ehead:{}:{}", spec.embedding_model, spec.groups.join(","));
        Self {
            embedder,
            spec,
            cache: None,
            sig,
        }
    }

    /// Build from a head-spec JSON file.
    pub fn from_file(weights_path: impl AsRef<Path>, embedder: Arc<EmbeddingSource>) -> Result<Self> {
        let spec = HeadSpec::load(weights_path)?;
        if spec.text_dim != embedder.dimensions() as usize {
            return Err(ForgeError::config(format!(
                "head text_dim {} != embedding dimensions {}",
                spec.text_dim,
                embedder.dimensions()
            )));
        }
        Ok(Self::new(embedder, spec))
    }

    /// Attach a decision cache.
    pub fn with_cache(mut self, cache: Arc<DecisionCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    fn decide(&self, embedding: &[f32], text: &str) -> Vec<ScoredGroup> {
        let feats = extract_features(text);
        let ns = norm_struct(&feats);
        let x: Vec<f32> = if self.spec.use_struct {
            embedding.iter().copied().chain(ns.iter().copied()).collect()
        } else {
            embedding.to_vec()
        };
        let h = hardness(&self.spec.quality, &self.spec.quality.probs(&x));
        let task_probs = self.spec.task.probs(&x);
        let task = task_probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| self.spec.task.classes[i].as_str())
            .unwrap_or("qa");
        fuse(h, task, &feats, ns[0])
    }
}

#[async_trait]
impl ModelSelector for EmbeddingHeadSelector {
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
        let text = ctx.full_text();
        let key = decision_key("embedding-head", &text, &self.sig);
        if let Some(c) = &self.cache {
            if let Some(hit) = c.get(key) {
                return Ok(hit);
            }
        }
        let embedding = self.embedder.embed(&text).await?;
        let ranked = self.decide(&embedding, &text);
        if let Some(c) = &self.cache {
            c.put(key, ranked.clone());
        }
        Ok(ranked)
    }

    fn name(&self) -> &str {
        "embedding-head"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_head(classes: &[&str], rows: Vec<Vec<f32>>, b: Vec<f32>) -> Head {
        Head {
            classes: classes.iter().map(|s| s.to_string()).collect(),
            layers: vec![Layer {
                w: rows,
                b,
                activation: "none".into(),
            }],
        }
    }

    #[test]
    fn binary_hardness_uses_p_class_zero() {
        // classes ["0","1"]; logits favor class 0 -> high hardness (P(weak wrong)).
        let q = linear_head(&["0", "1"], vec![vec![5.0], vec![0.0]], vec![0.0, 0.0]);
        let p = q.probs(&[1.0]);
        assert!(hardness(&q, &p) > 0.9);
    }

    #[test]
    fn relu_layer_zeroes_negatives() {
        let l = Layer {
            w: vec![vec![1.0], vec![-1.0]],
            b: vec![0.0, 0.0],
            activation: "relu".into(),
        };
        assert_eq!(l.forward(&[2.0]), vec![2.0, 0.0]);
    }

    fn feat(ctx_tokens: usize, n_files: usize, has_code: bool) -> Features {
        Features {
            ctx_tokens,
            n_files,
            has_code,
            has_diff: false,
            has_error: false,
        }
    }

    #[test]
    fn fuse_routes_each_capability() {
        assert_eq!(fuse(0.5, "code", &feat(50, 0, true), 0.0)[0].group, "code");
        assert_eq!(fuse(0.95, "math", &feat(50, 0, false), 0.0)[0].group, "reasoning");
        // non-trivial easy qa defaults to general (not the cheapest tier)
        assert_eq!(fuse(0.2, "qa", &feat(50, 0, false), 0.0)[0].group, "general");
        assert_eq!(fuse(0.4, "qa", &feat(50, 6, false), 0.2)[0].group, "long_context");
        // trivial greeting -> chat regardless of an uninformed mid hardness
        assert_eq!(fuse(0.55, "qa", &feat(2, 0, false), 0.0)[0].group, "chat");
    }
}
