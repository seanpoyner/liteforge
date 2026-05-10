# Knowledge Base API

In-memory document store with search capabilities.

## KnowledgeClient Trait

```rust
#[async_trait]
pub trait KnowledgeClient: Send + Sync {
    async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>>;
    async fn upload(&self, document: Document) -> Result<Document>;
    async fn get(&self, id: &str) -> Result<Option<Document>>;
    async fn list(&self, options: ListOptions) -> Result<Vec<Document>>;
    async fn delete(&self, id: &str) -> Result<bool>;
    async fn update(&self, document: Document) -> Result<Document>;
    async fn stats(&self) -> Result<KnowledgeStats>;
    async fn clear(&self) -> Result<()>;
}
```

## LocalKnowledgeBackend

In-memory implementation using TF-IDF-like scoring:

```rust
use liteforge::knowledge::LocalKnowledgeBackend;

let backend = LocalKnowledgeBackend::new();
// or with initial documents:
let backend = LocalKnowledgeBackend::with_documents(docs);
```

## SyncKnowledgeClient

Synchronous wrapper over any async `KnowledgeClient`:

```rust
let sync_client = SyncKnowledgeClient::new(backend);
```

## Document

```rust
use liteforge::knowledge::Document;

let doc = Document::new("doc-1", "Document content here")
    .namespace("wiki")
    .metadata(serde_json::json!({"author": "Alice"}))
    .source("https://example.com/article");
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique identifier |
| `content` | `String` | Document text |
| `namespace` | `Option<String>` | Logical grouping |
| `metadata` | `Option<Value>` | Arbitrary JSON metadata |
| `embedding` | `Option<Vec<f32>>` | Pre-computed embedding |
| `source` | `Option<String>` | Source URL or path |
| `created_at` | `Option<String>` | Creation timestamp |
| `updated_at` | `Option<String>` | Last update timestamp |

## SearchOptions

```rust
let options = SearchOptions::default()
    .limit(10)
    .namespace("wiki")
    .min_score(0.5)
    .include_highlights(true)
    .filters(serde_json::json!({"author": "Alice"}));
```

## SearchResult

| Field | Type |
|-------|------|
| `document` | `Document` |
| `score` | `f64` |
| `highlights` | `Vec<String>` |

## ListOptions

```rust
let options = ListOptions::default()
    .limit(50)
    .offset(0)
    .namespace("wiki");
```

## KnowledgeStats

| Field | Type |
|-------|------|
| `document_count` | `usize` |
| `namespace_count` | `usize` |
| `namespaces` | `Vec<String>` |
