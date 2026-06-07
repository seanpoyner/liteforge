# RAG Pipeline

LiteForge includes a complete Retrieval-Augmented Generation pipeline with text chunking, vector indexing, and retrieval.

## Overview

The RAG pipeline flow:

1. **Chunk** documents into smaller pieces
2. **Embed** chunks using the LiteForge embeddings API
3. **Index** embedded chunks in a vector store
4. **Retrieve** relevant chunks for a query
5. **Generate** a response with retrieved context

## Text Chunking

Split text into chunks using various strategies:

```rust
use liteforge::chunking::{chunk, ChunkingStrategy};

let text = "Your long document text here...";

let chunks = chunk(text, 512, 50, ChunkingStrategy::Recursive);

for c in &chunks {
    println!("Chunk {}: {} chars ({}..{})",
        c.index, c.len(), c.start_char, c.end_char);
}
```

### Chunking Strategies

| Strategy | Description |
|----------|-------------|
| `Fixed` | Fixed-size character chunks |
| `Recursive` | Splits on paragraphs, then sentences, then words |
| `Sentence` | Splits on sentence boundaries |
| `Paragraph` | Splits on paragraph boundaries |

## Embeddings

Generate embeddings via the LiteForge API:

```rust
let client = AsyncForgeClient::new();

// Single text
let response = client.embed("Hello world").await?;
let embedding: &[f32] = &response.embedding().unwrap();

// Batch
let response = client.embed_batch(vec![
    "First document",
    "Second document",
]).await?;
let embeddings = response.embeddings();
```

## Vector Index

Store and search embedded documents:

```rust
use liteforge::rag::{VectorIndex, EmbeddedDocument};

let mut index = VectorIndex::new();

index.add(EmbeddedDocument::new(
    "doc-1",
    "Rust is a systems programming language",
    vec![0.1, 0.2, 0.3, /* ... */],
));

// Search by embedding vector
let results = index.search(&query_embedding, 5);
for result in results {
    println!("{}: {:.3}", result.document.id, result.score);
}

// Search with minimum score threshold
let results = index.search_with_threshold(&query_embedding, 5, 0.7);
```

## Vector Utilities

```rust
use liteforge::rag::vector::{cosine_similarity, dot_product, normalize, euclidean_distance};

let similarity = cosine_similarity(&vec_a, &vec_b);
let distance = euclidean_distance(&vec_a, &vec_b);
```

## Full RAG Pipeline

The `RagPipeline` orchestrates the entire flow:

```rust
use liteforge::rag::{RagPipeline, RagConfig};
use liteforge::chunking::ChunkingStrategy;

let pipeline = RagPipeline::builder()
    .client(client)
    .embedding_model("text-embedding-3-small")
    .chunking_strategy(ChunkingStrategy::Recursive)
    .chunk_size(512)
    .chunk_overlap(50)
    .top_k(5)
    .min_score(0.7)
    .build();

// Index a document
pipeline.index_document("doc-1", "Your document text...").await;

// Index with metadata
pipeline.index_document_with_metadata(
    "doc-2",
    "Another document...",
    serde_json::json!({"source": "wiki", "date": "2024-01-01"}),
).await;

// Retrieve relevant chunks
let results = pipeline.retrieve("What is Rust?").await;
for result in results {
    println!("[{:.2}] {}", result.score, result.content);
}
```

## Python Usage

```python
from liteforge import VectorIndex, EmbeddedDocument, chunk

# Chunk text
chunks = chunk("Your document...", chunk_size=512, overlap=50, strategy="recursive")

# Build index
index = VectorIndex()
index.add(EmbeddedDocument("doc-1", "content", [0.1, 0.2, 0.3]))

# Search
results = index.search(query_embedding, top_k=5)
```

## JavaScript / TypeScript Usage

```javascript
import {
  VectorIndex, cosineSimilarity, dotProduct,
  euclideanDistance, normalize, chunk,
} from '@seanpoyner/liteforge';

// Chunk text
const chunks = chunk('Your long document text...', 512, 50, 'recursive');

// Vector index
const index = new VectorIndex();
index.addBatch([
  { id: 'doc1', content: 'Paris is the capital of France', embedding: [0.1, 0.2, 0.8, 0.1] },
  { id: 'doc2', content: 'The Eiffel Tower is 330m tall', embedding: [0.15, 0.25, 0.75, 0.15] },
]);

// Search by embedding
const results = index.search([0.11, 0.21, 0.79, 0.11], 3);
for (const r of results) {
  console.log(`[${r.score.toFixed(4)}] ${r.document.id}: ${r.document.content}`);
}

// Vector utilities
const similarity = cosineSimilarity([1, 2, 3], [4, 5, 6]);
```
