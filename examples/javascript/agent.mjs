/**
 * Agent Example
 *
 * Demonstrates the agent framework: AgentConfig, AgentMemory, and AgentContext.
 *
 * Run: node examples/javascript/agent.mjs
 */

import {
  JsAgentConfig,
  JsAgentMemory,
  createMessageUser,
  createMessageAssistant,
} from '@seanpoyner/liteforge';

// --- Agent Configuration ---
console.log('=== Agent Configuration ===\n');

const config = new JsAgentConfig('travel-assistant');
config.withSystemPrompt('You are an expert travel planner.');
config.withModel('gpt-4');
config.withMaxSteps(10);
config.withTemperature(0.7);
config.withMaxTokens(2048);
config.withTool('search');
config.withTool('calculator');

console.log(`Agent: ${config.name}`);
console.log(`Model: ${config.model}`);
console.log(`Max steps: ${config.maxSteps}`);
console.log(`System prompt: ${config.systemPrompt}`);

// --- Agent Memory ---
console.log('\n=== Agent Memory ===\n');

const memory = new JsAgentMemory();

memory.addMessage('user', 'I want to visit Japan.');
memory.addMessage('assistant', 'Japan is a wonderful destination! When are you planning to go?');
memory.addMessage('user', 'Next spring, during cherry blossom season.');

console.log(`Short-term messages: ${memory.messageCount()}`);

const msgs = memory.messages();
for (const msg of msgs) {
  console.log(`  [${msg.role}] ${msg.content}`);
}

// Long-term memory
memory.remember('destination', JSON.stringify('Japan'));
memory.remember('season', JSON.stringify('spring'));
memory.remember('interests', JSON.stringify(['cherry blossoms', 'temples', 'food']));

console.log(`\nRecalled destination: ${memory.recall('destination')}`);
console.log(`Recalled season: ${memory.recall('season')}`);

// Working memory
memory.setWorking('current_task', JSON.stringify('planning itinerary'));
memory.setWorking('budget', JSON.stringify({ amount: 5000, currency: 'USD' }));

console.log(`\nWorking task: ${memory.getWorking('current_task')}`);
console.log(`Working budget: ${memory.getWorking('budget')}`);

// Clear working memory
memory.clearWorking();
console.log(`After clearing working memory: ${memory.getWorking('current_task')}`);

// Forget long-term memory
memory.forget('season');
console.log(`After forgetting season: ${memory.recall('season')}`);
