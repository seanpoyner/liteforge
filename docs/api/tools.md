# Tools API

Framework for defining, registering, validating, and executing tools.

## Tool Trait

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Option<Value>;
    fn execute(&self, args: Value) -> Result<String, String>;
    fn requires_confirmation(&self) -> bool; // default: false
    fn to_definition(&self) -> ToolDefinition;
}
```

## FnTool

Function-based tool for quick definitions:

```rust
pub struct FnTool<F>
where F: Fn(Value) -> Result<String, String> + Send + Sync;
```

| Method | Description |
|--------|-------------|
| `new(name, description, schema, func)` | Create a new function tool |
| `requires_confirmation(self, bool)` | Set confirmation requirement |

## ToolRegistry

Collection of named tools:

| Method | Returns | Description |
|--------|---------|-------------|
| `new()` | `ToolRegistry` | Empty registry |
| `register(tool)` | -- | Add a `Box<dyn Tool>` |
| `register_arc(tool)` | -- | Add an `Arc<dyn Tool>` |
| `unregister(name)` | `Option<...>` | Remove by name |
| `get(name)` | `Option<&dyn Tool>` | Look up by name |
| `contains(name)` | `bool` | Check existence |
| `len()` | `usize` | Number of tools |
| `is_empty()` | `bool` | Check if empty |
| `names()` | `impl Iterator<Item = &str>` | All tool names |
| `tools()` | `impl Iterator<Item = &dyn Tool>` | All tools |
| `definitions()` | `Vec<ToolDefinition>` | OpenAI-format definitions |
| `merge(other)` | -- | Merge another registry |
| `filter(names)` | `ToolRegistry` | Subset by names |

## ToolExecutor

Executes tool calls with validation:

| Method | Returns | Description |
|--------|---------|-------------|
| `new(registry)` | `ToolExecutor` | Create executor |
| `validate_args(name, args)` | `Result<()>` | Validate against schema |
| `timeout()` | `Option<Duration>` | Get timeout |
| `execute(name, args)` | `ToolResult` | Execute by name |
| `execute_with_id(id, name, args)` | `ToolResult` | Execute with call ID |
| `execute_call(tool_call)` | `ToolResult` | Execute a `ToolCall` |
| `execute_calls(calls)` | `Vec<ToolResult>` | Execute multiple calls |
| `has_tool(name)` | `bool` | Check tool exists |
| `registry()` | `&ToolRegistry` | Access registry |
| `registry_mut()` | `&mut ToolRegistry` | Mutable registry access |

## ToolResult

```rust
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
    pub execution_time_ms: Option<u64>,
}
```

| Method | Description |
|--------|-------------|
| `success(id, name, result)` | Create success result |
| `error(id, name, error)` | Create error result |
| `with_execution_time(ms)` | Attach timing |
| `to_message_content()` | Format for LLM message |

## Schema Validation

```rust
pub fn validate_json_schema(
    value: &Value,
    schema: &Value
) -> Result<(), Vec<SchemaValidationError>>
```

Validates JSON values against JSON Schema. Supports: `type`, `required`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `minItems`/`maxItems`, nested objects and arrays.
