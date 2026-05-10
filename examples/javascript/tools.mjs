/**
 * Tools Example
 *
 * Demonstrates defining tools, registering them in a registry,
 * and executing them through the ToolExecutor.
 *
 * Run: node examples/javascript/tools.mjs
 */

import {
  ToolRegistry,
  ToolExecutor,
  validateJsonSchema,
} from '@forge/sdk';

// --- Create a Tool Registry ---
const registry = new ToolRegistry();

// Register a calculator tool
registry.register(
  'calculator',
  'Performs basic arithmetic operations',
  {
    type: 'object',
    properties: {
      operation: { type: 'string', enum: ['add', 'subtract', 'multiply', 'divide'] },
      a: { type: 'number' },
      b: { type: 'number' },
    },
    required: ['operation', 'a', 'b'],
  },
  (argsJson) => {
    const args = JSON.parse(argsJson);
    let result;
    switch (args.operation) {
      case 'add': result = args.a + args.b; break;
      case 'subtract': result = args.a - args.b; break;
      case 'multiply': result = args.a * args.b; break;
      case 'divide': result = args.b !== 0 ? args.a / args.b : 'Error: division by zero'; break;
      default: result = 'Unknown operation';
    }
    return JSON.stringify({ result });
  }
);

// Register a weather tool
registry.register(
  'get_weather',
  'Gets the current weather for a city',
  {
    type: 'object',
    properties: {
      city: { type: 'string', description: 'City name' },
    },
    required: ['city'],
  },
  (argsJson) => {
    const args = JSON.parse(argsJson);
    return JSON.stringify({
      city: args.city,
      temperature: 72,
      conditions: 'Sunny',
      humidity: 45,
    });
  }
);

console.log(`Registered tools: ${registry.names().join(', ')}`);
console.log(`Tool count: ${registry.len()}`);
console.log(`\nTool definitions:`);
console.log(JSON.stringify(registry.definitions(), null, 2));

// --- Execute Tools ---
const executor = new ToolExecutor(registry);

console.log('\n=== Executing Tools ===\n');

const calcResult = executor.execute('calculator', { operation: 'multiply', a: 7, b: 6 });
console.log(`Calculator: ${calcResult.result} (success: ${calcResult.success})`);

const weatherResult = executor.executeWithId('call-1', 'get_weather', { city: 'Paris' });
console.log(`Weather: ${weatherResult.result} (id: ${weatherResult.toolCallId})`);

// --- Schema Validation ---
console.log('\n=== Schema Validation ===\n');

const schema = {
  type: 'object',
  properties: {
    name: { type: 'string' },
    age: { type: 'number', minimum: 0 },
  },
  required: ['name'],
};

const validData = { name: 'Alice', age: 30 };
const invalidData = { age: -5 };

const validErrors = validateJsonSchema(schema, validData);
console.log(`Valid data errors: ${validErrors.length}`);

const invalidErrors = validateJsonSchema(schema, invalidData);
console.log(`Invalid data errors: ${invalidErrors.length}`);
for (const err of invalidErrors) {
  console.log(`  ${err.path}: ${err.message}`);
}
