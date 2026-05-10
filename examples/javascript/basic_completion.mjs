/**
 * Basic Completion Example
 *
 * Demonstrates how to create a client and make a simple chat completion request.
 *
 * Run: node examples/javascript/basic_completion.mjs
 *
 * Requires LITEFORGE_API_KEY environment variable.
 */

import {
  AsyncForgeClient,
  createMessageSystem,
  createMessageUser,
} from '@forge/sdk';

const client = new AsyncForgeClient();
console.log(`Using model: ${client.model}`);
console.log(`Base URL: ${client.baseUrl}`);

const messages = [
  createMessageSystem('You are a helpful assistant. Keep responses brief.'),
  createMessageUser('What is the capital of France?'),
];

const response = await client.complete(messages);
const content = response.choices[0]?.message?.content ?? 'No response';
console.log(`\nResponse: ${content}`);

if (response.usage) {
  console.log(`\nTokens used:`);
  console.log(`  Prompt: ${response.usage.promptTokens}`);
  console.log(`  Completion: ${response.usage.completionTokens}`);
  console.log(`  Total: ${response.usage.totalTokens}`);
}
