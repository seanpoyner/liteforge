/**
 * RAG (Retrieval-Augmented Generation) Example
 *
 * Demonstrates vector indexing, similarity search, and the math utilities.
 *
 * Run: node examples/javascript/rag.mjs
 */

import {
  VectorIndex,
  cosineSimilarity,
  dotProduct,
  euclideanDistance,
  normalize,
} from '@forge/sdk';

// --- Vector Math Utilities ---
console.log('=== Vector Math ===\n');

const a = [1.0, 2.0, 3.0];
const b = [4.0, 5.0, 6.0];

console.log(`Cosine similarity: ${cosineSimilarity(a, b).toFixed(4)}`);
console.log(`Dot product: ${dotProduct(a, b).toFixed(4)}`);
console.log(`Euclidean distance: ${euclideanDistance(a, b).toFixed(4)}`);

const normalized = normalize(a);
console.log(`Normalized [1,2,3]: [${normalized.map(v => v.toFixed(4)).join(', ')}]`);

// --- Vector Index ---
console.log('\n=== Vector Index ===\n');

const index = new VectorIndex();

const documents = [
  { id: 'doc1', content: 'Paris is the capital of France', embedding: [0.1, 0.2, 0.8, 0.1], metadata: { topic: 'geography' } },
  { id: 'doc2', content: 'The Eiffel Tower is 330 meters tall', embedding: [0.15, 0.25, 0.75, 0.15], metadata: { topic: 'landmarks' } },
  { id: 'doc3', content: 'Python is a programming language', embedding: [0.8, 0.1, 0.1, 0.8], metadata: { topic: 'technology' } },
  { id: 'doc4', content: 'Machine learning uses neural networks', embedding: [0.7, 0.2, 0.15, 0.7], metadata: { topic: 'technology' } },
  { id: 'doc5', content: 'The Louvre Museum is in Paris', embedding: [0.12, 0.22, 0.78, 0.12], metadata: { topic: 'landmarks' } },
];

index.addBatch(documents);
console.log(`Index size: ${index.len()}`);
console.log(`Document IDs: ${index.ids().join(', ')}`);

// Search for Paris-related content
const queryEmbedding = [0.11, 0.21, 0.79, 0.11];
const results = index.search(queryEmbedding, 3);

console.log('\nSearch results (top 3):');
for (const result of results) {
  console.log(`  [${result.score.toFixed(4)}] ${result.document.id}: ${result.document.content}`);
}

// Search with threshold
const filtered = index.searchWithThreshold(queryEmbedding, 5, 0.99);
console.log(`\nResults with score >= 0.99: ${filtered.length}`);

// Get a specific document
const doc = index.get('doc3');
if (doc) {
  console.log(`\nRetrieved doc3: ${doc.content}`);
  console.log(`  Metadata: ${JSON.stringify(doc.metadata)}`);
}

// Remove a document
index.remove('doc3');
console.log(`\nAfter removal, index size: ${index.len()}`);
