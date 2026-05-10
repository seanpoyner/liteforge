/**
 * Streaming Example
 *
 * Demonstrates streaming chat completions using the async iterator pattern.
 *
 * Run: node examples/javascript/streaming.mjs
 *
 * Requires LITEFORGE_API_KEY environment variable.
 */

import {
  AsyncForgeClient,
  createMessageSystem,
  createMessageUser,
} from '@forge/sdk';

const client = new AsyncForgeClient();

const messages = [
  createMessageSystem('You are a creative storyteller.'),
  createMessageUser('Tell me a very short story about a robot learning to paint.'),
];

console.log('Streaming response:\n');

const stream = await client.completeStream(messages);

let chunk;
while ((chunk = await stream.next()) !== null) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) {
    process.stdout.write(content);
  }
}

console.log('\n\nStream complete.');
