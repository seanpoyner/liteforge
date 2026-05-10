//! Embeddings types for LiteForge.

use serde::{Deserialize, Serialize};

/// Request for creating embeddings.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    /// Input text or array of texts to embed.
    pub input: EmbeddingInput,

    /// Model to use for embedding.
    pub model: String,

    /// Optional encoding format (defaults to "float").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,

    /// Optional dimensions for the embedding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,

    /// Optional user identifier for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Per-request metadata serialised as the top-level `metadata` field.
    /// See [`crate::ChatCompletionRequest::metadata`] for full semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl EmbeddingRequest {
    /// Create a new embedding request for a single text.
    pub fn new(model: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            input: EmbeddingInput::Single(input.into()),
            model: model.into(),
            encoding_format: None,
            dimensions: None,
            user: None,
            metadata: None,
        }
    }

    /// Create a new embedding request for multiple texts.
    pub fn batch(model: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            input: EmbeddingInput::Batch(inputs),
            model: model.into(),
            encoding_format: None,
            dimensions: None,
            user: None,
            metadata: None,
        }
    }

    /// Set the encoding format.
    pub fn encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Set the dimensions.
    pub fn dimensions(mut self, dims: u32) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Set the user identifier.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set per-request metadata. Merged with
    /// [`crate::ForgeConfig::default_metadata`] at transport time.
    pub fn metadata(
        mut self,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Input for embedding request - single text or batch.
#[derive(Debug, Clone)]
pub enum EmbeddingInput {
    /// Single text input.
    Single(String),
    /// Batch of text inputs.
    Batch(Vec<String>),
}

impl Serialize for EmbeddingInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            EmbeddingInput::Single(s) => serializer.serialize_str(s),
            EmbeddingInput::Batch(v) => v.serialize(serializer),
        }
    }
}

/// Response from the embeddings endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Object type, always "list".
    pub object: String,

    /// The embeddings.
    pub data: Vec<EmbeddingData>,

    /// Model used for the embedding.
    pub model: String,

    /// Usage statistics.
    pub usage: EmbeddingUsage,
}

impl EmbeddingResponse {
    /// Get the first embedding vector.
    pub fn embedding(&self) -> Option<&[f32]> {
        self.data.first().map(|d| d.embedding.as_slice())
    }

    /// Get all embedding vectors.
    pub fn embeddings(&self) -> Vec<&[f32]> {
        self.data.iter().map(|d| d.embedding.as_slice()).collect()
    }
}

/// Individual embedding data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// Object type, always "embedding".
    pub object: String,

    /// The embedding vector.
    pub embedding: Vec<f32>,

    /// Index in the input array.
    pub index: usize,
}

/// Usage statistics for embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_request_single() {
        let req = EmbeddingRequest::new("text-embedding-ada-002", "Hello, world!");
        assert_eq!(req.model, "text-embedding-ada-002");
        match req.input {
            EmbeddingInput::Single(s) => assert_eq!(s, "Hello, world!"),
            _ => panic!("Expected single input"),
        }
    }

    #[test]
    fn test_embedding_request_batch() {
        let req = EmbeddingRequest::batch(
            "text-embedding-ada-002",
            vec!["Hello".to_string(), "World".to_string()],
        );
        match req.input {
            EmbeddingInput::Batch(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected batch input"),
        }
    }

    #[test]
    fn test_embedding_request_builder() {
        let req = EmbeddingRequest::new("model", "text")
            .dimensions(1536)
            .encoding_format("float")
            .user("user-123");

        assert_eq!(req.dimensions, Some(1536));
        assert_eq!(req.encoding_format, Some("float".to_string()));
        assert_eq!(req.user, Some("user-123".to_string()));
    }
}
