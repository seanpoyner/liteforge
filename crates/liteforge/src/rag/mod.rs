//! RAG (Retrieval-Augmented Generation) pipeline for LiteForge.
//!
//! This module provides utilities for building RAG pipelines:
//! - Vector similarity search
//! - Document embedding and indexing
//! - Retrieval pipeline integration
//!
//! # Example
//!
//! ```no_run
//! use liteforge::rag::{cosine_similarity, EmbeddedDocument, VectorIndex};
//!
//! // Compute similarity between vectors
//! let v1 = vec![1.0, 0.0, 0.0];
//! let v2 = vec![0.707, 0.707, 0.0];
//! let similarity = cosine_similarity(&v1, &v2);
//! println!("Similarity: {:.3}", similarity);
//! ```

mod index;
mod pipeline;
mod vector;

pub use index::{EmbeddedDocument, VectorIndex, VectorSearchResult};
pub use pipeline::{RagConfig, RagPipeline, RagPipelineBuilder, RetrievalResult};
pub use vector::{cosine_similarity, dot_product, euclidean_distance, normalize};
