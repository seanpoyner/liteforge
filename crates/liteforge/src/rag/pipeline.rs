//! RAG pipeline for document retrieval and augmented generation.

use super::index::{EmbeddedDocument, VectorIndex, VectorSearchResult};
use crate::chunking::{chunk, ChunkingStrategy};
use crate::error::ForgeError;
use crate::types::EmbeddingRequest;
use crate::AsyncForgeClient;

/// Configuration for the RAG pipeline.
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// Model to use for embeddings.
    pub embedding_model: String,

    /// Chunking strategy for documents.
    pub chunking_strategy: ChunkingStrategy,

    /// Maximum chunk size.
    pub chunk_size: usize,

    /// Chunk overlap.
    pub chunk_overlap: usize,

    /// Number of results to retrieve.
    pub top_k: usize,

    /// Minimum similarity score threshold.
    pub min_score: Option<f32>,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            embedding_model: "text-embedding-ada-002".to_string(),
            chunking_strategy: ChunkingStrategy::Sentence,
            chunk_size: 512,
            chunk_overlap: 50,
            top_k: 5,
            min_score: None,
        }
    }
}

/// Result from a RAG retrieval operation.
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// The retrieved document chunk.
    pub content: String,

    /// Source document ID.
    pub document_id: String,

    /// Chunk index within the document.
    pub chunk_index: usize,

    /// Similarity score.
    pub score: f32,

    /// Optional metadata from the source document.
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Builder for creating a RAG pipeline.
pub struct RagPipelineBuilder {
    config: RagConfig,
    client: Option<AsyncForgeClient>,
}

impl RagPipelineBuilder {
    /// Create a new pipeline builder with default config.
    pub fn new() -> Self {
        Self {
            config: RagConfig::default(),
            client: None,
        }
    }

    /// Set the LiteForge client for embeddings.
    pub fn client(mut self, client: AsyncForgeClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Set the embedding model.
    pub fn embedding_model(mut self, model: impl Into<String>) -> Self {
        self.config.embedding_model = model.into();
        self
    }

    /// Set the chunking strategy.
    pub fn chunking_strategy(mut self, strategy: ChunkingStrategy) -> Self {
        self.config.chunking_strategy = strategy;
        self
    }

    /// Set the chunk size.
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.config.chunk_size = size;
        self
    }

    /// Set the chunk overlap.
    pub fn chunk_overlap(mut self, overlap: usize) -> Self {
        self.config.chunk_overlap = overlap;
        self
    }

    /// Set the number of results to retrieve.
    pub fn top_k(mut self, k: usize) -> Self {
        self.config.top_k = k;
        self
    }

    /// Set the minimum similarity score.
    pub fn min_score(mut self, score: f32) -> Self {
        self.config.min_score = Some(score);
        self
    }

    /// Build the pipeline.
    pub fn build(self) -> Result<RagPipeline, ForgeError> {
        let client = self
            .client
            .ok_or_else(|| ForgeError::config("RAG pipeline requires a LiteForge client for embeddings"))?;

        Ok(RagPipeline {
            config: self.config,
            client,
            index: VectorIndex::new(),
        })
    }
}

impl Default for RagPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A complete RAG pipeline for document retrieval.
///
/// Combines chunking, embedding, indexing, and retrieval.
///
/// # Example
///
/// ```no_run
/// use liteforge::{AsyncForgeClient, rag::{RagPipeline, RagPipelineBuilder}};
///
/// #[tokio::main]
/// async fn main() {
///     let client = AsyncForgeClient::new();
///     let mut pipeline = RagPipelineBuilder::new()
///         .client(client)
///         .top_k(3)
///         .build()
///         .unwrap();
///
///     // Index a document
///     pipeline.index_document("doc1", "Your document content here...").await.unwrap();
///
///     // Retrieve relevant chunks
///     let results = pipeline.retrieve("search query").await.unwrap();
///     for r in results {
///         println!("Score: {:.3}, Content: {}", r.score, r.content);
///     }
/// }
/// ```
pub struct RagPipeline {
    config: RagConfig,
    client: AsyncForgeClient,
    index: VectorIndex,
}

impl RagPipeline {
    /// Create a new pipeline builder.
    pub fn builder() -> RagPipelineBuilder {
        RagPipelineBuilder::new()
    }

    /// Index a document by chunking and embedding it.
    ///
    /// The document will be split into chunks and each chunk will be embedded
    /// and added to the index.
    pub async fn index_document(&mut self, id: &str, content: &str) -> Result<usize, ForgeError> {
        self.index_document_with_metadata(id, content, std::collections::HashMap::new())
            .await
    }

    /// Index a document with metadata.
    pub async fn index_document_with_metadata(
        &mut self,
        id: &str,
        content: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<usize, ForgeError> {
        // Chunk the document
        let chunks = chunk(
            content,
            self.config.chunk_size,
            self.config.chunk_overlap,
            self.config.chunking_strategy,
        );

        if chunks.is_empty() {
            return Ok(0);
        }

        // Get embeddings for all chunks
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = self.embed_batch(&chunk_texts).await?;

        // Add to index
        for (i, (chunk_obj, embedding)) in chunks.iter().zip(embeddings).enumerate() {
            let chunk_id = format!("{}#{}", id, i);
            let mut doc = EmbeddedDocument::new(&chunk_id, &chunk_obj.text, embedding);
            doc.metadata = metadata.clone();
            doc.metadata
                .insert("source_id".to_string(), serde_json::json!(id));
            doc.metadata
                .insert("chunk_index".to_string(), serde_json::json!(i));
            self.index.add(doc);
        }

        Ok(chunks.len())
    }

    /// Remove a document and all its chunks from the index.
    pub fn remove_document(&mut self, id: &str) -> usize {
        let prefix = format!("{}#", id);
        let ids_to_remove: Vec<String> = self
            .index
            .ids()
            .iter()
            .filter(|doc_id| doc_id.starts_with(&prefix))
            .map(|s| s.to_string())
            .collect();

        let count = ids_to_remove.len();
        for doc_id in ids_to_remove {
            self.index.remove(&doc_id);
        }
        count
    }

    /// Retrieve relevant document chunks for a query.
    pub async fn retrieve(&self, query: &str) -> Result<Vec<RetrievalResult>, ForgeError> {
        // Embed the query
        let query_embedding = self.embed(query).await?;

        // Search
        let results = if let Some(min_score) = self.config.min_score {
            self.index
                .search_with_threshold(&query_embedding, self.config.top_k, min_score)
        } else {
            self.index.search(&query_embedding, self.config.top_k)
        };

        // Convert to RetrievalResult
        Ok(results
            .into_iter()
            .map(|r| {
                let document_id = r
                    .document
                    .metadata
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&r.document.id)
                    .to_string();

                let chunk_index = r
                    .document
                    .metadata
                    .get("chunk_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                RetrievalResult {
                    content: r.document.content,
                    document_id,
                    chunk_index,
                    score: r.score,
                    metadata: r.document.metadata,
                }
            })
            .collect())
    }

    /// Get embeddings for a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, ForgeError> {
        let request = EmbeddingRequest::new(&self.config.embedding_model, text);
        let response = self.client.embeddings(request).await?;
        response
            .embedding()
            .map(|e| e.to_vec())
            .ok_or_else(|| ForgeError::internal("No embedding returned"))
    }

    /// Get embeddings for multiple texts.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ForgeError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = EmbeddingRequest::batch(&self.config.embedding_model, texts.to_vec());
        let response = self.client.embeddings(request).await?;
        Ok(response
            .embeddings()
            .into_iter()
            .map(|e| e.to_vec())
            .collect())
    }

    /// Get the number of chunks in the index.
    pub fn chunk_count(&self) -> usize {
        self.index.len()
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.index.clear();
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &RagConfig {
        &self.config
    }

    /// Add pre-embedded documents directly to the index.
    ///
    /// Useful when you have embeddings from an external source.
    pub fn add_embedded(&mut self, documents: Vec<EmbeddedDocument>) {
        self.index.add_batch(documents);
    }

    /// Search with a pre-computed query embedding.
    ///
    /// Useful when you want to reuse a query embedding.
    pub fn search_with_embedding(&self, embedding: &[f32]) -> Vec<VectorSearchResult> {
        if let Some(min_score) = self.config.min_score {
            self.index
                .search_with_threshold(embedding, self.config.top_k, min_score)
        } else {
            self.index.search(embedding, self.config.top_k)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.top_k, 5);
        assert_eq!(config.chunk_size, 512);
    }

    #[test]
    fn test_builder_requires_client() {
        let result = RagPipelineBuilder::new().build();
        assert!(result.is_err());
    }
}
