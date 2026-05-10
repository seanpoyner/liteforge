//! Knowledge base for document storage and retrieval.
//!
//! This module provides a simple knowledge base API for storing and searching documents.
//! It's designed for RAG (Retrieval-Augmented Generation) pipelines.
//!
//! # Example
//!
//! ```
//! use liteforge::knowledge::{LocalKnowledgeBackend, Document, SearchOptions, KnowledgeClient};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a local backend
//!     let backend = LocalKnowledgeBackend::new();
//!
//!     // Upload documents
//!     let docs = vec![
//!         Document::new("1", "Rust is a systems programming language"),
//!         Document::new("2", "Python is great for data science"),
//!     ];
//!     backend.upload(docs).await.unwrap();
//!
//!     // Search for documents
//!     let results = backend.search("rust programming", SearchOptions::new().limit(5)).await.unwrap();
//!     for result in results {
//!         println!("Found: {} (score: {:.2})", result.document.id, result.score);
//!     }
//! }
//! ```

mod client;
mod local;
mod types;

pub use client::{KnowledgeClient, SyncKnowledgeClient};
pub use local::LocalKnowledgeBackend;
pub use types::{Document, KnowledgeStats, ListOptions, SearchOptions, SearchResult};
