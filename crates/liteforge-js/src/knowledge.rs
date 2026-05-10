use napi::bindgen_prelude::*;
use std::collections::HashMap;
use liteforge::knowledge::{
    Document as RustDocument, KnowledgeClient, ListOptions as RustListOptions,
    LocalKnowledgeBackend as RustLocalKnowledgeBackend, SearchOptions as RustSearchOptions,
    SearchResult as RustSearchResult,
};

#[napi(object)]
pub struct JsDocument {
    pub id: String,
    pub content: String,
    pub namespace: Option<String>,
    pub source: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

fn js_doc_to_rust(doc: &JsDocument) -> RustDocument {
    let mut d = RustDocument::new(&doc.id, &doc.content);
    if let Some(ref ns) = doc.namespace {
        d = d.namespace(ns);
    }
    if let Some(ref src) = doc.source {
        d = d.source(src);
    }
    for (k, v) in &doc.metadata {
        d = d.metadata(k, v.clone());
    }
    d
}

fn rust_doc_to_js(doc: &RustDocument) -> JsDocument {
    JsDocument {
        id: doc.id.clone(),
        content: doc.content.clone(),
        namespace: doc.namespace.clone(),
        source: doc.source.clone(),
        metadata: doc.metadata.clone(),
    }
}

#[napi(object)]
pub struct JsSearchResult {
    pub document: JsDocument,
    pub score: f64,
    pub highlights: Vec<String>,
}

fn rust_search_result_to_js(r: &RustSearchResult) -> JsSearchResult {
    JsSearchResult {
        document: rust_doc_to_js(&r.document),
        score: r.score as f64,
        highlights: r.highlights.clone(),
    }
}

#[napi(object)]
pub struct JsKnowledgeStats {
    pub document_count: u32,
    pub namespace_count: u32,
    pub namespaces: Vec<String>,
}

#[napi]
pub struct SearchOptions {
    inner: RustSearchOptions,
}

#[napi]
impl SearchOptions {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustSearchOptions::new(),
        }
    }

    #[napi]
    pub fn limit(&mut self, limit: u32) -> &Self {
        self.inner = self.inner.clone().limit(limit as usize);
        self
    }

    #[napi]
    pub fn namespace(&mut self, namespace: String) -> &Self {
        self.inner = self.inner.clone().namespace(namespace);
        self
    }

    #[napi]
    pub fn filter(&mut self, key: String, value: String) -> &Self {
        self.inner = self
            .inner
            .clone()
            .filter(key, serde_json::Value::String(value));
        self
    }

    #[napi]
    pub fn include_highlights(&mut self, include: bool) -> &Self {
        self.inner = self.inner.clone().include_highlights(include);
        self
    }
}

#[napi]
pub struct ListOptions {
    inner: RustListOptions,
}

#[napi]
impl ListOptions {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustListOptions::new(),
        }
    }

    #[napi]
    pub fn limit(&mut self, limit: u32) -> &Self {
        self.inner = self.inner.clone().limit(limit as usize);
        self
    }

    #[napi]
    pub fn offset(&mut self, offset: u32) -> &Self {
        self.inner = self.inner.clone().offset(offset as usize);
        self
    }

    #[napi]
    pub fn namespace(&mut self, namespace: String) -> &Self {
        self.inner = self.inner.clone().namespace(namespace);
        self
    }
}

#[napi]
pub struct LocalKnowledgeBackend {
    inner: RustLocalKnowledgeBackend,
    runtime: tokio::runtime::Runtime,
}

#[napi]
impl LocalKnowledgeBackend {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::from_reason(format!("Failed to create runtime: {}", e)))?;
        Ok(Self {
            inner: RustLocalKnowledgeBackend::new(),
            runtime,
        })
    }

    #[napi]
    pub fn search(
        &self,
        query: String,
        options: Option<&SearchOptions>,
    ) -> Result<Vec<JsSearchResult>> {
        let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
        let results = self
            .runtime
            .block_on(self.inner.search(&query, opts))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(results.iter().map(rust_search_result_to_js).collect())
    }

    #[napi]
    pub fn upload(&self, documents: Vec<JsDocument>) -> Result<u32> {
        let docs: Vec<RustDocument> = documents.iter().map(js_doc_to_rust).collect();
        let ids = self
            .runtime
            .block_on(self.inner.upload(docs))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(ids.len() as u32)
    }

    #[napi]
    pub fn get(&self, id: String) -> Result<Option<JsDocument>> {
        let doc = self
            .runtime
            .block_on(self.inner.get(&id))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(doc.as_ref().map(rust_doc_to_js))
    }

    #[napi]
    pub fn delete(&self, id: String) -> Result<bool> {
        self.runtime
            .block_on(self.inner.delete(&id))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn update(&self, document: JsDocument) -> Result<bool> {
        let doc = js_doc_to_rust(&document);
        self.runtime
            .block_on(self.inner.update(doc))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn list(&self, options: Option<&ListOptions>) -> Result<Vec<JsDocument>> {
        let opts = options.map(|o| o.inner.clone()).unwrap_or_default();
        let docs = self
            .runtime
            .block_on(self.inner.list(opts))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(docs.iter().map(rust_doc_to_js).collect())
    }

    #[napi]
    pub fn stats(&self) -> Result<JsKnowledgeStats> {
        let stats = self
            .runtime
            .block_on(self.inner.stats())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(JsKnowledgeStats {
            document_count: stats.document_count as u32,
            namespace_count: stats.namespace_count as u32,
            namespaces: stats.namespaces.clone(),
        })
    }

    #[napi]
    pub fn clear(&self, namespace: Option<String>) -> Result<u32> {
        let count = self
            .runtime
            .block_on(self.inner.clear(namespace.as_deref()))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }
}
