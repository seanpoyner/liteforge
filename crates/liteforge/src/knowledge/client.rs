//! Knowledge client trait for document storage and retrieval.

use super::types::{Document, KnowledgeStats, ListOptions, SearchOptions, SearchResult};
use crate::error::ForgeError;
use async_trait::async_trait;

/// A client for interacting with a knowledge base.
///
/// This trait defines the interface for document storage and retrieval.
/// Implementations can use different backends (local, LiteForge API, vector DBs, etc.).
#[async_trait]
pub trait KnowledgeClient: Send + Sync {
    /// Search for documents matching a query.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `options` - Search options (limit, namespace, filters, etc.)
    ///
    /// # Returns
    /// A list of search results sorted by relevance score (highest first).
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, ForgeError>;

    /// Upload documents to the knowledge base.
    ///
    /// # Arguments
    /// * `documents` - The documents to upload
    ///
    /// # Returns
    /// The IDs of the uploaded documents.
    async fn upload(&self, documents: Vec<Document>) -> Result<Vec<String>, ForgeError>;

    /// Get a document by its ID.
    ///
    /// # Arguments
    /// * `id` - The document ID
    ///
    /// # Returns
    /// The document if found, None otherwise.
    async fn get(&self, id: &str) -> Result<Option<Document>, ForgeError>;

    /// List documents in the knowledge base.
    ///
    /// # Arguments
    /// * `options` - List options (limit, offset, namespace)
    ///
    /// # Returns
    /// A list of documents.
    async fn list(&self, options: ListOptions) -> Result<Vec<Document>, ForgeError>;

    /// Delete a document by its ID.
    ///
    /// # Arguments
    /// * `id` - The document ID
    ///
    /// # Returns
    /// True if the document was deleted, false if it wasn't found.
    async fn delete(&self, id: &str) -> Result<bool, ForgeError>;

    /// Update an existing document.
    ///
    /// # Arguments
    /// * `document` - The updated document (must have an existing ID)
    ///
    /// # Returns
    /// True if the document was updated, false if it wasn't found.
    async fn update(&self, document: Document) -> Result<bool, ForgeError>;

    /// Get statistics about the knowledge base.
    async fn stats(&self) -> Result<KnowledgeStats, ForgeError>;

    /// Delete all documents in a namespace.
    ///
    /// # Arguments
    /// * `namespace` - The namespace to clear (None clears all documents)
    ///
    /// # Returns
    /// The number of documents deleted.
    async fn clear(&self, namespace: Option<&str>) -> Result<usize, ForgeError>;
}

/// Synchronous wrapper for KnowledgeClient operations.
pub struct SyncKnowledgeClient<C: KnowledgeClient> {
    inner: C,
    runtime: tokio::runtime::Runtime,
}

impl<C: KnowledgeClient> SyncKnowledgeClient<C> {
    /// Create a new sync wrapper around an async client.
    pub fn new(client: C) -> Result<Self, ForgeError> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| ForgeError::internal(format!("Failed to create runtime: {}", e)))?;
        Ok(Self {
            inner: client,
            runtime,
        })
    }

    /// Search for documents.
    pub fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, ForgeError> {
        self.runtime.block_on(self.inner.search(query, options))
    }

    /// Upload documents.
    pub fn upload(&self, documents: Vec<Document>) -> Result<Vec<String>, ForgeError> {
        self.runtime.block_on(self.inner.upload(documents))
    }

    /// Get a document by ID.
    pub fn get(&self, id: &str) -> Result<Option<Document>, ForgeError> {
        self.runtime.block_on(self.inner.get(id))
    }

    /// List documents.
    pub fn list(&self, options: ListOptions) -> Result<Vec<Document>, ForgeError> {
        self.runtime.block_on(self.inner.list(options))
    }

    /// Delete a document.
    pub fn delete(&self, id: &str) -> Result<bool, ForgeError> {
        self.runtime.block_on(self.inner.delete(id))
    }

    /// Update a document.
    pub fn update(&self, document: Document) -> Result<bool, ForgeError> {
        self.runtime.block_on(self.inner.update(document))
    }

    /// Get statistics.
    pub fn stats(&self) -> Result<KnowledgeStats, ForgeError> {
        self.runtime.block_on(self.inner.stats())
    }

    /// Clear documents.
    pub fn clear(&self, namespace: Option<&str>) -> Result<usize, ForgeError> {
        self.runtime.block_on(self.inner.clear(namespace))
    }

    /// Get a reference to the inner async client.
    pub fn inner(&self) -> &C {
        &self.inner
    }
}
