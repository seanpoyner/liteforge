/**
 * Conversation Management Example
 *
 * Demonstrates ManagedConversation and CompactingConversation for
 * multi-turn conversation tracking with token management.
 *
 * Run: node examples/javascript/conversation.mjs
 */

import {
  ManagedConversation,
  CompactingConversation,
  ConversationConfig,
} from '@forge/sdk';

// --- ManagedConversation ---
console.log('=== Managed Conversation ===\n');

const conv = new ManagedConversation();

conv.setSystem('You are a helpful travel assistant.');
conv.addUserMessage('I want to visit Paris.');
conv.addAssistantMessage('Paris is wonderful! The Eiffel Tower, Louvre Museum, and Notre-Dame are must-sees.');
conv.addUserMessage('What about food?');
conv.addAssistantMessage('Try croissants, escargot, and crème brûlée. Visit Le Marais for great cafés.');

console.log(`Messages: ${conv.len()}`);
console.log(`Estimated tokens: ${conv.estimatedTokens()}`);

const messages = conv.messages();
for (const msg of messages) {
  const preview = (msg.content ?? '').substring(0, 60);
  console.log(`  [${msg.role}] ${preview}...`);
}

// --- CompactingConversation ---
console.log('\n=== Compacting Conversation ===\n');

const config = new ConversationConfig(
  500,    // maxTokens
  200,    // targetTokens
  2,      // preserveRecent
  'KeepRecent',
);

const compacting = new CompactingConversation(config);
compacting.setSystem('You are a knowledgeable guide.');

for (let i = 0; i < 20; i++) {
  compacting.addUserMessage(`Question ${i + 1}: Tell me about topic ${i + 1}.`);
  compacting.addAssistantMessage(`Here is information about topic ${i + 1}. `.repeat(3));
}

console.log(`Messages before compaction: ${compacting.messages().length}`);
console.log(`Estimated tokens: ${compacting.estimatedTokens()}`);
console.log(`Needs compaction: ${compacting.needsCompaction()}`);

if (compacting.needsCompaction()) {
  const result = compacting.compact();
  console.log(`\nCompaction result:`, JSON.stringify(result, null, 2));
  console.log(`Messages after compaction: ${compacting.messages().length}`);
  console.log(`Estimated tokens after: ${compacting.estimatedTokens()}`);
}
