//! In-memory vector index for similarity search.

use super::vector::cosine_similarity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A document with its embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedDocument {
    /// Unique document identifier.
    pub id: String,

    /// The document content/text.
    pub content: String,

    /// The embedding vector.
    pub embedding: Vec<f32>,

    /// Optional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EmbeddedDocument {
    /// Create a new embedded document.
    pub fn new(id: impl Into<String>, content: impl Into<String>, embedding: Vec<f32>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            embedding,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the document.
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Result from a vector similarity search.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// The matched document.
    pub document: EmbeddedDocument,

    /// Similarity score (0.0 to 1.0 for cosine similarity).
    pub score: f32,
}

/// An in-memory vector index for similarity search.
///
/// Uses brute-force cosine similarity search. For production use with
/// large datasets, consider using a dedicated vector database.
///
/// # Example
///
/// ```
/// use liteforge::rag::{VectorIndex, EmbeddedDocument};
///
/// let mut index = VectorIndex::new();
///
/// // Add documents
/// index.add(EmbeddedDocument::new("1", "Hello world", vec![1.0, 0.0, 0.0]));
/// index.add(EmbeddedDocument::new("2", "Goodbye world", vec![0.0, 1.0, 0.0]));
///
/// // Search
/// let query = vec![0.9, 0.1, 0.0];
/// let results = index.search(&query, 1);
/// assert_eq!(results[0].document.id, "1");
/// ```
pub struct VectorIndex {
    documents: Vec<EmbeddedDocument>,
    id_to_index: HashMap<String, usize>,
}

impl VectorIndex {
    /// Create a new empty vector index.
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            id_to_index: HashMap::new(),
        }
    }

    /// Create an index with initial documents.
    pub fn with_documents(documents: Vec<EmbeddedDocument>) -> Self {
        let mut index = Self::new();
        for doc in documents {
            index.add(doc);
        }
        index
    }

    /// Add a document to the index.
    ///
    /// If a document with the same ID exists, it will be replaced.
    pub fn add(&mut self, document: EmbeddedDocument) {
        if let Some(&idx) = self.id_to_index.get(&document.id) {
            // Replace existing
            self.documents[idx] = document;
        } else {
            // Add new
            let idx = self.documents.len();
            self.id_to_index.insert(document.id.clone(), idx);
            self.documents.push(document);
        }
    }

    /// Add multiple documents to the index.
    pub fn add_batch(&mut self, documents: Vec<EmbeddedDocument>) {
        for doc in documents {
            self.add(doc);
        }
    }

    /// Remove a document from the index.
    ///
    /// Returns true if the document was found and removed.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(idx) = self.id_to_index.remove(id) {
            // Swap remove for efficiency
            self.documents.swap_remove(idx);

            // Update the index of the swapped document
            if idx < self.documents.len() {
                let swapped_id = &self.documents[idx].id;
                self.id_to_index.insert(swapped_id.clone(), idx);
            }
            true
        } else {
            false
        }
    }

    /// Get a document by ID.
    pub fn get(&self, id: &str) -> Option<&EmbeddedDocument> {
        self.id_to_index.get(id).map(|&idx| &self.documents[idx])
    }

    /// Search for similar documents using cosine similarity.
    ///
    /// Returns up to `limit` documents sorted by similarity (highest first).
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<VectorSearchResult> {
        let mut results: Vec<VectorSearchResult> = self
            .documents
            .iter()
            .map(|doc| {
                let score = cosine_similarity(query, &doc.embedding);
                VectorSearchResult {
                    document: doc.clone(),
                    score,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top results
        results.truncate(limit);
        results
    }

    /// Search with a minimum score threshold.
    pub fn search_with_threshold(
        &self,
        query: &[f32],
        limit: usize,
        min_score: f32,
    ) -> Vec<VectorSearchResult> {
        let mut results: Vec<VectorSearchResult> = self
            .documents
            .iter()
            .filter_map(|doc| {
                let score = cosine_similarity(query, &doc.embedding);
                if score >= min_score {
                    Some(VectorSearchResult {
                        document: doc.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Get the number of documents in the index.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) {
        self.documents.clear();
        self.id_to_index.clear();
    }

    /// Get all document IDs.
    pub fn ids(&self) -> Vec<&str> {
        self.documents.iter().map(|d| d.id.as_str()).collect()
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: &str, embedding: Vec<f32>) -> EmbeddedDocument {
        EmbeddedDocument::new(id, format!("Content for {}", id), embedding)
    }

    #[test]
    fn test_add_and_get() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0, 0.0]));

        let doc = index.get("1").unwrap();
        assert_eq!(doc.id, "1");
        assert_eq!(doc.embedding, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_add_replaces_existing() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0, 0.0]));
        index.add(make_doc("1", vec![0.0, 1.0, 0.0]));

        assert_eq!(index.len(), 1);
        assert_eq!(index.get("1").unwrap().embedding, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_remove() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0, 0.0]));
        index.add(make_doc("2", vec![0.0, 1.0, 0.0]));

        assert!(index.remove("1"));
        assert!(!index.remove("1")); // Already removed
        assert_eq!(index.len(), 1);
        assert!(index.get("2").is_some());
    }

    #[test]
    fn test_search() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0, 0.0])); // Most similar to query
        index.add(make_doc("2", vec![0.0, 1.0, 0.0])); // Orthogonal
        index.add(make_doc("3", vec![0.7, 0.7, 0.0])); // Somewhat similar

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document.id, "1");
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_search_with_threshold() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0, 0.0]));
        index.add(make_doc("2", vec![0.0, 1.0, 0.0]));
        index.add(make_doc("3", vec![0.9, 0.1, 0.0]));

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search_with_threshold(&query, 10, 0.5);

        // Only doc 1 and 3 should match (doc 2 is orthogonal)
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.score >= 0.5));
    }

    #[test]
    fn test_with_documents() {
        let docs = vec![make_doc("1", vec![1.0, 0.0]), make_doc("2", vec![0.0, 1.0])];
        let index = VectorIndex::with_documents(docs);

        assert_eq!(index.len(), 2);
        assert!(index.get("1").is_some());
        assert!(index.get("2").is_some());
    }

    #[test]
    fn test_clear() {
        let mut index = VectorIndex::new();
        index.add(make_doc("1", vec![1.0, 0.0]));
        index.add(make_doc("2", vec![0.0, 1.0]));

        index.clear();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }
}
