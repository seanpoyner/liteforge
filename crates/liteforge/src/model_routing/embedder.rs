//! Embedding source for selectors.
//!
//! Selectors that reason about prompt content (semantic, MF) need an embedding
//! of the prompt. Embeddings are fetched over HTTP from an OpenAI-compatible
//! endpoint (e.g. a local `bge-m3` served by LiteLLM); the SDK does no local ML
//! inference. A mock backend is provided for tests.

use crate::client::AsyncForgeClient;
use crate::config::ForgeConfig;
use crate::error::{ForgeError, Result};
use crate::types::EmbeddingRequest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the embedding model a selector uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelConfig {
    /// OpenAI-compatible base URL (e.g. `https://litellm.poyner.ai/v1`).
    pub base_url: String,
    /// API key; when `None`, falls back to the environment (`LITEFORGE_API_KEY`).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Embedding model id (e.g. `bge-m3`).
    pub model: String,
    /// Expected embedding dimensionality (validated against responses and MF weights).
    pub dimensions: u32,
    /// Optional request timeout in seconds.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

type EmbedFn = Arc<dyn Fn(&str) -> Vec<f32> + Send + Sync>;

// The Live variant holds an HTTP client (one per selector), the Mock variant a
// closure; the size gap is intentional and there is exactly one per selector.
#[allow(clippy::large_enum_variant)]
enum Backend {
    Live {
        client: AsyncForgeClient,
        model: String,
    },
    Mock(EmbedFn),
}

/// Produces embedding vectors for selector inputs.
pub struct EmbeddingSource {
    backend: Backend,
    dimensions: u32,
}

impl EmbeddingSource {
    /// Build a live embedding source from config (constructs an internal client).
    pub fn new(cfg: &EmbeddingModelConfig) -> Result<Self> {
        if cfg.base_url.is_empty() {
            return Err(ForgeError::config("embedding base_url is empty"));
        }
        if cfg.dimensions == 0 {
            return Err(ForgeError::config("embedding dimensions must be > 0"));
        }
        // Start from env config so api_key falls back to LITEFORGE_API_KEY, then
        // override endpoint/model/timeout for the embedding backend.
        let mut fc = ForgeConfig::from_env();
        fc.base_url = cfg.base_url.clone();
        fc.default_model = cfg.model.clone();
        if let Some(k) = &cfg.api_key {
            fc.api_key = Some(k.clone());
        }
        if let Some(secs) = cfg.timeout_secs {
            fc.timeout = Duration::from_secs(secs);
        }
        Ok(Self {
            backend: Backend::Live {
                client: AsyncForgeClient::with_config(fc),
                model: cfg.model.clone(),
            },
            dimensions: cfg.dimensions,
        })
    }

    /// Build a mock embedding source backed by a closure (for tests).
    pub fn mock<F>(dimensions: u32, f: F) -> Self
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        Self {
            backend: Backend::Mock(Arc::new(f)),
            dimensions,
        }
    }

    /// Expected embedding dimensionality.
    pub fn dimensions(&self) -> u32 {
        self.dimensions
    }

    /// Embed a single text into a `dimensions`-length vector.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let v = match &self.backend {
            Backend::Live { client, model } => {
                let req = EmbeddingRequest::new(model, text).dimensions(self.dimensions);
                let resp = client.embeddings(req).await?;
                resp.embedding()
                    .ok_or_else(|| ForgeError::internal("embedding response had no data"))?
                    .to_vec()
            }
            Backend::Mock(f) => f(text),
        };
        if v.len() != self.dimensions as usize {
            return Err(ForgeError::internal(format!(
                "embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                v.len()
            )));
        }
        Ok(v)
    }
}

impl std::fmt::Debug for EmbeddingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingSource")
            .field("dimensions", &self.dimensions)
            .field(
                "backend",
                &match &self.backend {
                    Backend::Live { model, .. } => format!("live({model})"),
                    Backend::Mock(_) => "mock".to_string(),
                },
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embed_returns_vector() {
        let src = EmbeddingSource::mock(3, |_t| vec![0.1, 0.2, 0.3]);
        let v = src.embed("hello").await.unwrap();
        assert_eq!(v, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn mock_embed_dim_mismatch_errors() {
        let src = EmbeddingSource::mock(4, |_t| vec![0.1, 0.2, 0.3]);
        assert!(src.embed("hello").await.is_err());
    }
}
