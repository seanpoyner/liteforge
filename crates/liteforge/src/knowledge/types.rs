//! Knowledge types for document storage and retrieval.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A document stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document.
    pub id: String,

    /// The text content of the document.
    pub content: String,

    /// Optional namespace for organizing documents.
    #[serde(default)]
    pub namespace: Option<String>,

    /// Arbitrary metadata as key-value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// Optional embedding vector for semantic search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Source of the document (e.g., filename, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Unix timestamp when the document was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,

    /// Unix timestamp when the document was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

impl Document {
    /// Create a new document with the given ID and content.
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            namespace: None,
            metadata: HashMap::new(),
            embedding: None,
            source: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the namespace.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the embedding vector.
    pub fn embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the creation timestamp.
    pub fn created_at(mut self, timestamp: i64) -> Self {
        self.created_at = Some(timestamp);
        self
    }
}

/// A search result containing a document and its relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching document.
    pub document: Document,

    /// Relevance score (higher is better).
    pub score: f32,

    /// Optional highlighted snippets from the document.
    #[serde(default)]
    pub highlights: Vec<String>,
}

impl SearchResult {
    /// Create a new search result.
    pub fn new(document: Document, score: f32) -> Self {
        Self {
            document,
            score,
            highlights: Vec::new(),
        }
    }

    /// Add highlighted snippets.
    pub fn with_highlights(mut self, highlights: Vec<String>) -> Self {
        self.highlights = highlights;
        self
    }
}

/// Options for searching the knowledge base.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Namespace to search within.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Minimum score threshold for results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f32>,

    /// Whether to include highlights in results.
    #[serde(default)]
    pub include_highlights: bool,

    /// Metadata filters as key-value pairs.
    #[serde(default)]
    pub filters: HashMap<String, Value>,
}

impl SearchOptions {
    /// Create new search options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the namespace to search within.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the minimum score threshold.
    pub fn min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }

    /// Enable or disable highlights.
    pub fn include_highlights(mut self, include: bool) -> Self {
        self.include_highlights = include;
        self
    }

    /// Add a metadata filter.
    pub fn filter(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }
}

/// Options for listing documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    /// Maximum number of documents to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Offset for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,

    /// Namespace to list from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl ListOptions {
    /// Create new list options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of documents.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the pagination offset.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set the namespace.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }
}

/// Statistics about a namespace or the entire knowledge base.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeStats {
    /// Total number of documents.
    pub document_count: usize,

    /// Total number of namespaces.
    pub namespace_count: usize,

    /// List of namespace names.
    pub namespaces: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_document_builder() {
        let doc = Document::new("doc1", "Hello world")
            .namespace("test")
            .metadata("author", json!("alice"))
            .source("test.txt");

        assert_eq!(doc.id, "doc1");
        assert_eq!(doc.content, "Hello world");
        assert_eq!(doc.namespace, Some("test".to_string()));
        assert_eq!(doc.metadata.get("author"), Some(&json!("alice")));
        assert_eq!(doc.source, Some("test.txt".to_string()));
    }

    #[test]
    fn test_search_options() {
        let opts = SearchOptions::new()
            .limit(10)
            .namespace("docs")
            .min_score(0.5)
            .include_highlights(true)
            .filter("type", json!("article"));

        assert_eq!(opts.limit, Some(10));
        assert_eq!(opts.namespace, Some("docs".to_string()));
        assert_eq!(opts.min_score, Some(0.5));
        assert!(opts.include_highlights);
        assert_eq!(opts.filters.get("type"), Some(&json!("article")));
    }

    #[test]
    fn test_document_serialization() {
        let doc = Document::new("doc1", "Test content")
            .namespace("test")
            .metadata("key", json!("value"));

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, doc.id);
        assert_eq!(parsed.content, doc.content);
        assert_eq!(parsed.namespace, doc.namespace);
    }
}
