//! Remote classifier selector: route via a classifier served behind LiteLLM
//! (e.g. a RouteLLM BERT/causal model running on hal). The classifier returns
//! per-label scores (or a single label) which are mapped onto model groups.

use crate::client::AsyncForgeClient;
use crate::error::{ForgeError, Result};
use crate::model_routing::cache::{decision_key, DecisionCache};
use crate::routing::{ModelSelector, ScoredGroup, SelectionContext};
use crate::types::{ChatCompletionRequest, Message};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// How the classifier is reached.
#[derive(Debug, Clone)]
pub enum ClassifierEndpoint {
    /// A LiteLLM chat model that returns JSON in its message content.
    Chat {
        /// The classifier model id.
        model: String,
    },
    /// A custom HTTP path that returns the classifier JSON directly.
    Custom {
        /// Path relative to the client base URL.
        path: String,
    },
}

/// Lenient classifier response: accepts `{scores:{label:score}}`,
/// `{group, score}`, or `{label}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClassifierResponse {
    /// Per-label scores.
    #[serde(default)]
    pub scores: Option<HashMap<String, f32>>,
    /// Single chosen group/label.
    #[serde(default)]
    pub group: Option<String>,
    /// Single label (alias for `group`).
    #[serde(default)]
    pub label: Option<String>,
    /// Score for the single label.
    #[serde(default)]
    pub score: Option<f32>,
}

impl ClassifierResponse {
    /// Normalize into a label -> score map.
    pub fn into_label_scores(self) -> HashMap<String, f32> {
        if let Some(scores) = self.scores {
            return scores;
        }
        let mut out = HashMap::new();
        if let Some(g) = self.group.or(self.label) {
            out.insert(g, self.score.unwrap_or(1.0));
        }
        out
    }
}

/// Parse a classifier response from possibly-wrapped model output (extracts the
/// first `{...}` JSON object if there is surrounding text).
pub fn parse_classifier_json(content: &str) -> Result<ClassifierResponse> {
    if let Ok(r) = serde_json::from_str::<ClassifierResponse>(content.trim()) {
        return Ok(r);
    }
    if let (Some(start), Some(end)) = (content.find('{'), content.rfind('}')) {
        if end > start {
            let slice = &content[start..=end];
            return serde_json::from_str::<ClassifierResponse>(slice)
                .map_err(|e| ForgeError::internal(format!("classifier JSON parse failed: {e}")));
        }
    }
    Err(ForgeError::internal(
        "classifier response was not valid JSON",
    ))
}

const SYSTEM_PROMPT: &str = "You are a routing classifier. Given the user message, respond with ONLY a JSON object of the form {\"scores\": {\"<label>\": <0..1>, ...}} ranking how well each routing label fits. Do not include any other text.";

/// Routes by calling a remote classifier and mapping its labels to groups.
pub struct RemoteClassifierSelector {
    client: AsyncForgeClient,
    endpoint: ClassifierEndpoint,
    label_to_group: HashMap<String, String>,
    cache: Option<Arc<DecisionCache>>,
}

impl RemoteClassifierSelector {
    /// Create a remote classifier selector.
    pub fn new(
        client: AsyncForgeClient,
        endpoint: ClassifierEndpoint,
        label_to_group: HashMap<String, String>,
    ) -> Self {
        Self {
            client,
            endpoint,
            label_to_group,
            cache: None,
        }
    }

    /// Attach a decision cache.
    pub fn with_cache(mut self, cache: Arc<DecisionCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    fn map_to_groups(&self, scores: HashMap<String, f32>) -> Vec<ScoredGroup> {
        let mut out: Vec<ScoredGroup> = scores
            .into_iter()
            .filter_map(|(label, score)| {
                self.label_to_group
                    .get(&label)
                    .map(|group| ScoredGroup::new(group, score).with_reason(format!("classifier label '{label}'")))
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }
}

#[async_trait]
impl ModelSelector for RemoteClassifierSelector {
    async fn select(&self, ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
        let text = ctx.prompt_text();
        let sig = {
            let mut labels: Vec<&str> = self.label_to_group.keys().map(String::as_str).collect();
            labels.sort_unstable();
            labels.join(",")
        };
        let key = decision_key("remote-classifier", &text, &sig);
        if let Some(cache) = &self.cache {
            if let Some(hit) = cache.get(key) {
                return Ok(hit);
            }
        }

        let resp = match &self.endpoint {
            ClassifierEndpoint::Chat { model } => {
                let req = ChatCompletionRequest::new(
                    model,
                    vec![Message::system(SYSTEM_PROMPT), Message::user(text.clone())],
                )
                .temperature(0.0);
                let completion = self.client.chat_completions(req).await?;
                let content = completion
                    .content()
                    .ok_or_else(|| ForgeError::internal("classifier returned no content"))?;
                parse_classifier_json(content)?
            }
            ClassifierEndpoint::Custom { path } => {
                let body = serde_json::json!({ "input": text });
                self.client.post::<_, ClassifierResponse>(path, &body).await?
            }
        };

        let ranked = self.map_to_groups(resp.into_label_scores());
        if let Some(cache) = &self.cache {
            cache.put(key, ranked.clone());
        }
        Ok(ranked)
    }

    fn name(&self) -> &str {
        "remote-classifier"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scores_object() {
        let r = parse_classifier_json(r#"{"scores":{"hard":0.8,"easy":0.2}}"#).unwrap();
        let m = r.into_label_scores();
        assert_eq!(m.get("hard"), Some(&0.8));
    }

    #[test]
    fn parses_single_label_with_surrounding_text() {
        let r = parse_classifier_json("Here you go: {\"group\":\"premium\",\"score\":0.9} done")
            .unwrap();
        let m = r.into_label_scores();
        assert_eq!(m.get("premium"), Some(&0.9));
    }

    #[test]
    fn maps_labels_to_groups_and_drops_unmapped() {
        let mut l2g = HashMap::new();
        l2g.insert("hard".to_string(), "premium".to_string());
        l2g.insert("easy".to_string(), "cheap".to_string());
        let sel = RemoteClassifierSelector::new(
            AsyncForgeClient::with_config(crate::config::ForgeConfig {
                api_key: Some("k".into()),
                default_model: "m".into(),
                base_url: "http://x/v1".into(),
                timeout: std::time::Duration::from_secs(1),
                default_headers: Default::default(),
                default_metadata: Default::default(),
                otel: None,
            }),
            ClassifierEndpoint::Chat {
                model: "bert".into(),
            },
            l2g,
        );
        let mut scores = HashMap::new();
        scores.insert("hard".to_string(), 0.7);
        scores.insert("easy".to_string(), 0.3);
        scores.insert("unknown".to_string(), 0.9);
        let ranked = sel.map_to_groups(scores);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].group, "premium");
    }
}
