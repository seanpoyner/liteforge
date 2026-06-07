/**
 * Knowledge Base Example
 *
 * Demonstrates the LocalKnowledgeBackend for document storage and search.
 *
 * Run: node examples/javascript/knowledge.mjs
 */

import {
  LocalKnowledgeBackend,
  SearchOptions,
  ListOptions,
} from '@seanpoyner/liteforge';

const backend = new LocalKnowledgeBackend();

// --- Upload Documents ---
console.log('=== Uploading Documents ===\n');

const documents = [
  { id: 'ai-1', content: 'Artificial intelligence enables machines to learn from experience.', namespace: 'ai', source: 'textbook', metadata: { topic: 'ai', level: 'intro' } },
  { id: 'ai-2', content: 'Neural networks are inspired by the biological brain.', namespace: 'ai', source: 'textbook', metadata: { topic: 'ai', level: 'intermediate' } },
  { id: 'ai-3', content: 'Deep learning uses multiple layers of neural networks.', namespace: 'ai', source: 'paper', metadata: { topic: 'deep-learning', level: 'advanced' } },
  { id: 'travel-1', content: 'Paris is known for the Eiffel Tower and world-class cuisine.', namespace: 'travel', source: 'guide', metadata: { region: 'europe' } },
  { id: 'travel-2', content: 'Tokyo offers a blend of traditional temples and modern technology.', namespace: 'travel', source: 'guide', metadata: { region: 'asia' } },
];

const count = backend.upload(documents);
console.log(`Uploaded ${count} documents`);

// --- Search ---
console.log('\n=== Searching ===\n');

const searchOpts = new SearchOptions();
searchOpts.limit(3);
searchOpts.namespace('ai');
searchOpts.includeHighlights(true);

const results = backend.search('neural networks learning', searchOpts);
console.log(`Found ${results.length} results in 'ai' namespace:`);
for (const r of results) {
  console.log(`  [${r.score.toFixed(3)}] ${r.document.id}: ${r.document.content.substring(0, 60)}...`);
  if (r.highlights.length > 0) {
    console.log(`    Highlights: ${r.highlights.join(', ')}`);
  }
}

// --- Get & Update ---
console.log('\n=== Get & Update ===\n');

const doc = backend.get('travel-1');
if (doc) {
  console.log(`Retrieved: ${doc.id} - ${doc.content}`);
}

backend.update({
  id: 'travel-1',
  content: 'Paris, the City of Light, is famous for the Eiffel Tower, Louvre, and cuisine.',
  namespace: 'travel',
  source: 'guide-v2',
  metadata: { region: 'europe', updated: 'true' },
});

const updated = backend.get('travel-1');
if (updated) {
  console.log(`Updated: ${updated.content}`);
}

// --- List ---
console.log('\n=== Listing ===\n');

const listOpts = new ListOptions();
listOpts.namespace('ai');
listOpts.limit(10);

const aiDocs = backend.list(listOpts);
console.log(`AI namespace documents: ${aiDocs.length}`);
for (const d of aiDocs) {
  console.log(`  ${d.id}: ${d.content.substring(0, 50)}...`);
}

// --- Stats ---
console.log('\n=== Stats ===\n');

const stats = backend.stats();
console.log(`Total documents: ${stats.totalDocuments}`);
console.log(`Namespaces: ${stats.namespaces.join(', ')}`);
console.log(`Total size: ${stats.totalSizeBytes} bytes`);

// --- Delete & Clear ---
backend.delete('ai-3');
console.log(`\nAfter deleting ai-3: ${backend.stats().totalDocuments} documents`);

const cleared = backend.clear('travel');
console.log(`Cleared ${cleared} travel documents`);
console.log(`Final count: ${backend.stats().totalDocuments} documents`);
