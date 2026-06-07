# Tools & Function Calling

LiteForge provides a complete framework for defining, registering, and executing tools that LLMs can invoke.

## Defining a Tool

Implement the `Tool` trait:

```rust
use liteforge::tools::{Tool, ToolDefinition, ToolParameters};
use serde_json::Value;

struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &str { "get_weather" }

    fn description(&self) -> &str {
        "Get the current weather for a location"
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }))
    }

    fn execute(&self, args: Value) -> Result<String, String> {
        let location = args["location"].as_str().unwrap_or("unknown");
        Ok(format!("Weather in {location}: 72°F, sunny"))
    }
}
```

## Function-Based Tools

For simpler tools, use `FnTool`:

```rust
use liteforge::tools::FnTool;

let tool = FnTool::new(
    "calculator",
    "Evaluate a math expression",
    Some(serde_json::json!({
        "type": "object",
        "properties": {
            "expression": { "type": "string" }
        },
        "required": ["expression"]
    })),
    |args| {
        let expr = args["expression"].as_str().unwrap_or("");
        Ok(format!("Result: {expr}"))
    },
);
```

## Tool Registry

Manage collections of tools:

```rust
use liteforge::tools::ToolRegistry;

let mut registry = ToolRegistry::new();
registry.register(Box::new(WeatherTool));
registry.register(Box::new(calculator_tool));

// Query the registry
assert!(registry.contains("get_weather"));
let names: Vec<&str> = registry.names().collect();
let definitions = registry.definitions(); // For LLM tool parameter

// Merge registries
let mut other = ToolRegistry::new();
registry.merge(other);

// Filter tools
let filtered = registry.filter(&["get_weather"]);
```

## Tool Executor

Execute tool calls with validation and timeout:

```rust
use liteforge::tools::ToolExecutor;

let executor = ToolExecutor::new(registry);

// Execute a single tool call
let result = executor.execute("get_weather", serde_json::json!({
    "location": "Seattle"
}));

// Execute from LLM tool_calls
let results = executor.execute_calls(&tool_calls);

// Check results
for result in &results {
    if result.success {
        println!("{}: {}", result.name, result.result);
    } else {
        eprintln!("{}: {}", result.name, result.error.as_deref().unwrap_or("unknown"));
    }
}
```

## Schema Validation

Validate tool arguments against JSON Schema:

```rust
use liteforge::tools::validate_json_schema;

let schema = serde_json::json!({
    "type": "object",
    "properties": {
        "name": { "type": "string", "minLength": 1 },
        "age": { "type": "integer", "minimum": 0 }
    },
    "required": ["name"]
});

let value = serde_json::json!({ "name": "Alice", "age": 30 });
match validate_json_schema(&value, &schema) {
    Ok(()) => println!("Valid"),
    Err(errors) => {
        for e in errors {
            eprintln!("{}: {}", e.path, e.message);
        }
    }
}
```

Supported validations: `type`, `required`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `minItems`/`maxItems`, and nested object/array schemas.

## Tool Calling Agent Loop

See the [Agents guide](agents.md) for using tools with the agent framework's automatic tool-calling loop.

## JavaScript / TypeScript

The JS bindings expose a `ToolRegistry` with a callback-based `register` API:

```javascript
import { ToolRegistry, ToolExecutor, validateJsonSchema } from '@seanpoyner/liteforge';

const registry = new ToolRegistry();

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
    return JSON.stringify({ city: args.city, temperature: 72, conditions: 'Sunny' });
  }
);

// Execute tools
const executor = new ToolExecutor(registry);
const result = executor.execute('get_weather', { city: 'Paris' });
console.log(result.result); // {"city":"Paris","temperature":72,"conditions":"Sunny"}

// Schema validation
const errors = validateJsonSchema(
  { type: 'object', properties: { name: { type: 'string' } }, required: ['name'] },
  { age: 30 }
);
// errors: [{ path: '.name', message: 'missing required field' }]
```

Tool definitions from `registry.definitions()` can be passed directly to `client.chatCompletions()` for LLM function calling.
