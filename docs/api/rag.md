# RAG API

Vector search, text chunking, and retrieval-augmented generation pipeline.

## Chunking

### ChunkingStrategy

```rust
pub enum ChunkingStrategy {
    Fixed,
    Recursive,
    Sentence,
    Paragraph,
}
```

Implements `FromStr` for parsing from strings (`"fixed"`, `"recursive"`, `"sentence"`, `"paragraph"`).

### Chunk

```rust
pub struct Chunk {
    pub text: String,
    pub index: usize,
    pub start_char: usize,
    pub end_char: usize,
}
```

| Method | Returns |
|--------|---------|
| `new(text, index, start, end)` | `Chunk` |
| `len()` | `usize` |
| `is_empty()` | `bool` |

### chunk()

```rust
pub fn chunk(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    strategy: ChunkingStrategy,
) -> Vec<Chunk>
```

## Vector Math

```rust
use liteforge::rag::vector;

vector::cosine_similarity(&a, &b)    // -> f32
vector::dot_product(&a, &b)          // -> f32
vector::normalize(&v)                // -> Vec<f32>
vector::euclidean_distance(&a, &b)   // -> f32
```

## VectorIndex

Brute-force cosine similarity search:

| Method | Description |
|--------|-------------|
| `new()` | Create empty index |
| `with_documents(docs)` | Create with initial documents |
| `add(doc)` | Add a document |
| `add_batch(docs)` | Add multiple documents |
| `remove(id)` | Remove by ID |
| `get(id)` | Get by ID |
| `search(embedding, top_k)` | Top-k search |
| `search_with_threshold(embedding, top_k, min_score)` | Search with score cutoff |
| `len()` | Document count |
| `is_empty()` | Check if empty |
| `clear()` | Remove all documents |
| `ids()` | List all document IDs |

### EmbeddedDocument

```rust
pub struct EmbeddedDocument {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: Option<Value>,
}
```

### VectorSearchResult

```rust
pub struct VectorSearchResult {
    pub document: EmbeddedDocument,
    pub score: f32,
}
```

## RagPipeline

End-to-end RAG orchestration:

```rust
let pipeline = RagPipeline::builder()
    .client(async_client)
    .embedding_model("text-embedding-3-small")
    .chunking_strategy(ChunkingStrategy::Recursive)
    .chunk_size(512)
    .chunk_overlap(50)
    .top_k(5)
    .min_score(0.7)
    .build();
```

| Method | Description |
|--------|-------------|
| `builder()` | Create builder |
| `index_document(id, text)` | Chunk, embed, and index |
| `index_document_with_metadata(id, text, meta)` | Index with metadata |
| `remove_document(id)` | Remove from index |
| `retrieve(query)` | Retrieve relevant chunks |
| `chunk_count()` | Total indexed chunks |
| `clear()` | Clear index |
| `config()` | Get `RagConfig` |

### RetrievalResult

```rust
pub struct RetrievalResult {
    pub content: String,
    pub document_id: String,
    pub chunk_index: usize,
    pub score: f32,
    pub metadata: Option<Value>,
}
```
