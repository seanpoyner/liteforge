use std::collections::HashMap;
use liteforge::rag::{
    cosine_similarity as rust_cosine_similarity, dot_product as rust_dot_product,
    euclidean_distance as rust_euclidean_distance, normalize as rust_normalize,
    EmbeddedDocument as RustEmbeddedDocument, VectorIndex as RustVectorIndex,
};

#[napi(object)]
pub struct JsEmbeddedDocument {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

fn js_embedded_to_rust(doc: &JsEmbeddedDocument) -> RustEmbeddedDocument {
    let mut d = RustEmbeddedDocument::new(
        &doc.id,
        &doc.content,
        doc.embedding.iter().map(|&v| v as f32).collect(),
    );
    for (k, v) in &doc.metadata {
        d = d.metadata(k, serde_json::Value::String(v.clone()));
    }
    d
}

fn rust_embedded_to_js(doc: &RustEmbeddedDocument) -> JsEmbeddedDocument {
    JsEmbeddedDocument {
        id: doc.id.clone(),
        content: doc.content.clone(),
        embedding: doc.embedding.iter().map(|&v| v as f64).collect(),
        metadata: doc
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
    }
}

#[napi(object)]
pub struct JsVectorSearchResult {
    pub document: JsEmbeddedDocument,
    pub score: f64,
}

#[napi]
pub struct VectorIndex {
    inner: RustVectorIndex,
}

#[napi]
impl VectorIndex {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustVectorIndex::new(),
        }
    }

    #[napi]
    pub fn add(&mut self, document: JsEmbeddedDocument) {
        self.inner.add(js_embedded_to_rust(&document));
    }

    #[napi]
    pub fn add_batch(&mut self, documents: Vec<JsEmbeddedDocument>) {
        let docs: Vec<_> = documents.iter().map(js_embedded_to_rust).collect();
        self.inner.add_batch(docs);
    }

    #[napi]
    pub fn remove(&mut self, id: String) -> bool {
        self.inner.remove(&id)
    }

    #[napi]
    pub fn get(&self, id: String) -> Option<JsEmbeddedDocument> {
        self.inner.get(&id).map(rust_embedded_to_js)
    }

    #[napi]
    pub fn search(&self, query_embedding: Vec<f64>, top_k: u32) -> Vec<JsVectorSearchResult> {
        let query: Vec<f32> = query_embedding.iter().map(|&v| v as f32).collect();
        self.inner
            .search(&query, top_k as usize)
            .iter()
            .map(|r| JsVectorSearchResult {
                document: rust_embedded_to_js(&r.document),
                score: r.score as f64,
            })
            .collect()
    }

    #[napi]
    pub fn search_with_threshold(
        &self,
        query_embedding: Vec<f64>,
        top_k: u32,
        min_score: f64,
    ) -> Vec<JsVectorSearchResult> {
        let query: Vec<f32> = query_embedding.iter().map(|&v| v as f32).collect();
        self.inner
            .search_with_threshold(&query, top_k as usize, min_score as f32)
            .iter()
            .map(|r| JsVectorSearchResult {
                document: rust_embedded_to_js(&r.document),
                score: r.score as f64,
            })
            .collect()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[napi]
    pub fn ids(&self) -> Vec<String> {
        self.inner
            .ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}

#[napi]
pub fn cosine_similarity(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a_f32: Vec<f32> = a.iter().map(|&v| v as f32).collect();
    let b_f32: Vec<f32> = b.iter().map(|&v| v as f32).collect();
    rust_cosine_similarity(&a_f32, &b_f32) as f64
}

#[napi]
pub fn dot_product(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a_f32: Vec<f32> = a.iter().map(|&v| v as f32).collect();
    let b_f32: Vec<f32> = b.iter().map(|&v| v as f32).collect();
    rust_dot_product(&a_f32, &b_f32) as f64
}

#[napi]
pub fn euclidean_distance(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a_f32: Vec<f32> = a.iter().map(|&v| v as f32).collect();
    let b_f32: Vec<f32> = b.iter().map(|&v| v as f32).collect();
    rust_euclidean_distance(&a_f32, &b_f32) as f64
}

#[napi]
pub fn normalize(v: Vec<f64>) -> Vec<f64> {
    let v_f32: Vec<f32> = v.iter().map(|&val| val as f32).collect();
    rust_normalize(&v_f32)
        .iter()
        .map(|&val| val as f64)
        .collect()
}
