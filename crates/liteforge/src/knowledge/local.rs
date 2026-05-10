//! Local in-memory knowledge backend for development and testing.

use super::client::KnowledgeClient;
use super::types::{Document, KnowledgeStats, ListOptions, SearchOptions, SearchResult};
use crate::error::ForgeError;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// An in-memory knowledge backend for development and testing.
///
/// Uses simple TF-IDF-like text matching for search.
/// Not suitable for production use with large document sets.
///
/// # Example
///
/// ```
/// use liteforge::knowledge::{LocalKnowledgeBackend, Document, SearchOptions, KnowledgeClient};
///
/// #[tokio::main]
/// async fn main() {
///     let backend = LocalKnowledgeBackend::new();
///
///     // Upload documents
///     let docs = vec![
///         Document::new("1", "Rust is a systems programming language"),
///         Document::new("2", "Python is great for data science"),
///     ];
///     backend.upload(docs).await.unwrap();
///
///     // Search
///     let results = backend.search("rust programming", SearchOptions::new().limit(5)).await.unwrap();
///     println!("Found {} results", results.len());
/// }
/// ```
pub struct LocalKnowledgeBackend {
    documents: RwLock<HashMap<String, Document>>,
}

impl LocalKnowledgeBackend {
    /// Create a new empty local backend.
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
        }
    }

    /// Create a local backend with initial documents.
    pub fn with_documents(documents: Vec<Document>) -> Self {
        let mut map = HashMap::new();
        for doc in documents {
            map.insert(doc.id.clone(), doc);
        }
        Self {
            documents: RwLock::new(map),
        }
    }

    /// Get the current timestamp.
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Simple text search scoring using term frequency.
    fn score_document(query: &str, document: &Document) -> f32 {
        let query_lower = query.to_lowercase();
        let query_terms: HashSet<&str> = query_lower.split_whitespace().collect();

        if query_terms.is_empty() {
            return 0.0;
        }

        let content_lower = document.content.to_lowercase();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        let mut matches = 0;
        let mut exact_matches = 0;

        for term in &query_terms {
            // Count occurrences
            let term_count = content_words.iter().filter(|w| w.contains(term)).count();
            matches += term_count;

            // Bonus for exact word matches
            if content_words.contains(term) {
                exact_matches += 1;
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Score based on:
        // 1. Number of matching terms
        // 2. Frequency of matches
        // 3. Document length normalization
        let term_coverage = exact_matches as f32 / query_terms.len() as f32;
        let frequency_score = matches as f32 / content_words.len().max(1) as f32;

        // Combine scores (term coverage weighted higher)
        term_coverage * 0.7 + frequency_score * 0.3
    }

    /// Extract highlighted snippets from document.
    fn extract_highlights(query: &str, document: &Document, max_highlights: usize) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        if query_terms.is_empty() {
            return Vec::new();
        }

        let sentences: Vec<&str> = document
            .content
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut scored_sentences: Vec<(f32, String)> = sentences
            .iter()
            .map(|s| {
                let sentence_lower = s.to_lowercase();
                let score = query_terms
                    .iter()
                    .filter(|term| sentence_lower.contains(*term))
                    .count() as f32;
                (score, s.trim().to_string())
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored_sentences.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored_sentences.truncate(max_highlights);

        scored_sentences.into_iter().map(|(_, s)| s).collect()
    }

    /// Check if a document matches the metadata filters.
    fn matches_filters(document: &Document, filters: &HashMap<String, serde_json::Value>) -> bool {
        for (key, expected) in filters {
            match document.metadata.get(key) {
                Some(actual) if actual == expected => continue,
                _ => return false,
            }
        }
        true
    }
}

impl Default for LocalKnowledgeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KnowledgeClient for LocalKnowledgeBackend {
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, ForgeError> {
        let documents = self
            .documents
            .read()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        let mut results: Vec<SearchResult> = documents
            .values()
            .filter(|doc| {
                // Filter by namespace
                if let Some(ref ns) = options.namespace {
                    if doc.namespace.as_ref() != Some(ns) {
                        return false;
                    }
                }
                // Filter by metadata
                if !Self::matches_filters(doc, &options.filters) {
                    return false;
                }
                true
            })
            .map(|doc| {
                let score = Self::score_document(query, doc);
                let highlights = if options.include_highlights {
                    Self::extract_highlights(query, doc, 3)
                } else {
                    Vec::new()
                };
                SearchResult {
                    document: doc.clone(),
                    score,
                    highlights,
                }
            })
            .filter(|r| {
                // Filter by minimum score
                if let Some(min) = options.min_score {
                    r.score >= min
                } else {
                    r.score > 0.0
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(limit) = options.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn upload(&self, documents: Vec<Document>) -> Result<Vec<String>, ForgeError> {
        let mut store = self
            .documents
            .write()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        let now = Self::now();
        let mut ids = Vec::with_capacity(documents.len());

        for mut doc in documents {
            // Set timestamps
            if doc.created_at.is_none() {
                doc.created_at = Some(now);
            }
            doc.updated_at = Some(now);

            ids.push(doc.id.clone());
            store.insert(doc.id.clone(), doc);
        }

        Ok(ids)
    }

    async fn get(&self, id: &str) -> Result<Option<Document>, ForgeError> {
        let documents = self
            .documents
            .read()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;
        Ok(documents.get(id).cloned())
    }

    async fn list(&self, options: ListOptions) -> Result<Vec<Document>, ForgeError> {
        let documents = self
            .documents
            .read()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        let mut docs: Vec<Document> = documents
            .values()
            .filter(|doc| {
                if let Some(ref ns) = options.namespace {
                    doc.namespace.as_ref() == Some(ns)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by ID for consistent ordering
        docs.sort_by(|a, b| a.id.cmp(&b.id));

        // Apply pagination
        let offset = options.offset.unwrap_or(0);
        if offset > 0 {
            docs = docs.into_iter().skip(offset).collect();
        }

        if let Some(limit) = options.limit {
            docs.truncate(limit);
        }

        Ok(docs)
    }

    async fn delete(&self, id: &str) -> Result<bool, ForgeError> {
        let mut documents = self
            .documents
            .write()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;
        Ok(documents.remove(id).is_some())
    }

    async fn update(&self, document: Document) -> Result<bool, ForgeError> {
        let mut store = self
            .documents
            .write()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        if store.contains_key(&document.id) {
            let mut doc = document;
            doc.updated_at = Some(Self::now());
            store.insert(doc.id.clone(), doc);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn stats(&self) -> Result<KnowledgeStats, ForgeError> {
        let documents = self
            .documents
            .read()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        let mut namespaces: HashSet<String> = HashSet::new();
        for doc in documents.values() {
            if let Some(ref ns) = doc.namespace {
                namespaces.insert(ns.clone());
            }
        }

        let mut ns_list: Vec<String> = namespaces.into_iter().collect();
        ns_list.sort();

        Ok(KnowledgeStats {
            document_count: documents.len(),
            namespace_count: ns_list.len(),
            namespaces: ns_list,
        })
    }

    async fn clear(&self, namespace: Option<&str>) -> Result<usize, ForgeError> {
        let mut documents = self
            .documents
            .write()
            .map_err(|e| ForgeError::internal(format!("Lock error: {}", e)))?;

        match namespace {
            Some(ns) => {
                let to_remove: Vec<String> = documents
                    .iter()
                    .filter(|(_, doc)| doc.namespace.as_deref() == Some(ns))
                    .map(|(id, _)| id.clone())
                    .collect();
                let count = to_remove.len();
                for id in to_remove {
                    documents.remove(&id);
                }
                Ok(count)
            }
            None => {
                let count = documents.len();
                documents.clear();
                Ok(count)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_upload_and_get() {
        let backend = LocalKnowledgeBackend::new();

        let doc = Document::new("doc1", "Hello world");
        backend.upload(vec![doc]).await.unwrap();

        let retrieved = backend.get("doc1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Hello world");
    }

    #[tokio::test]
    async fn test_search() {
        let backend = LocalKnowledgeBackend::new();

        let docs = vec![
            Document::new(
                "1",
                "Rust is a systems programming language focused on safety",
            ),
            Document::new("2", "Python is great for data science and machine learning"),
            Document::new(
                "3",
                "JavaScript runs in the browser and on servers with Node.js",
            ),
        ];
        backend.upload(docs).await.unwrap();

        let results = backend
            .search("rust programming", SearchOptions::new())
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "1");
    }

    #[tokio::test]
    async fn test_search_with_namespace() {
        let backend = LocalKnowledgeBackend::new();

        let docs = vec![
            Document::new("1", "Rust programming").namespace("tech"),
            Document::new("2", "Rust the game").namespace("games"),
        ];
        backend.upload(docs).await.unwrap();

        let results = backend
            .search("rust", SearchOptions::new().namespace("tech"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "1");
    }

    #[tokio::test]
    async fn test_search_with_highlights() {
        let backend = LocalKnowledgeBackend::new();

        let doc = Document::new("1", "Rust is fast. Rust is safe. Rust is fun to learn.");
        backend.upload(vec![doc]).await.unwrap();

        let results = backend
            .search("rust", SearchOptions::new().include_highlights(true))
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert!(!results[0].highlights.is_empty());
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let backend = LocalKnowledgeBackend::new();

        let docs: Vec<Document> = (0..10)
            .map(|i| Document::new(format!("doc{}", i), format!("Content {}", i)))
            .collect();
        backend.upload(docs).await.unwrap();

        let page1 = backend
            .list(ListOptions::new().limit(3).offset(0))
            .await
            .unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = backend
            .list(ListOptions::new().limit(3).offset(3))
            .await
            .unwrap();
        assert_eq!(page2.len(), 3);
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = LocalKnowledgeBackend::new();

        let doc = Document::new("doc1", "Hello");
        backend.upload(vec![doc]).await.unwrap();

        assert!(backend.delete("doc1").await.unwrap());
        assert!(!backend.delete("doc1").await.unwrap()); // Already deleted
        assert!(backend.get("doc1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let backend = LocalKnowledgeBackend::new();

        let doc = Document::new("doc1", "Original content");
        backend.upload(vec![doc]).await.unwrap();

        let updated = Document::new("doc1", "Updated content");
        assert!(backend.update(updated).await.unwrap());

        let retrieved = backend.get("doc1").await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated content");
    }

    #[tokio::test]
    async fn test_stats() {
        let backend = LocalKnowledgeBackend::new();

        let docs = vec![
            Document::new("1", "Content").namespace("ns1"),
            Document::new("2", "Content").namespace("ns1"),
            Document::new("3", "Content").namespace("ns2"),
        ];
        backend.upload(docs).await.unwrap();

        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.document_count, 3);
        assert_eq!(stats.namespace_count, 2);
    }

    #[tokio::test]
    async fn test_clear_namespace() {
        let backend = LocalKnowledgeBackend::new();

        let docs = vec![
            Document::new("1", "Content").namespace("ns1"),
            Document::new("2", "Content").namespace("ns2"),
        ];
        backend.upload(docs).await.unwrap();

        let deleted = backend.clear(Some("ns1")).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(backend.stats().await.unwrap().document_count, 1);
    }

    #[tokio::test]
    async fn test_metadata_filter() {
        let backend = LocalKnowledgeBackend::new();

        let docs = vec![
            Document::new("1", "Article about Rust").metadata("type", json!("article")),
            Document::new("2", "Tutorial about Rust").metadata("type", json!("tutorial")),
        ];
        backend.upload(docs).await.unwrap();

        let results = backend
            .search(
                "rust",
                SearchOptions::new().filter("type", json!("article")),
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "1");
    }
}
