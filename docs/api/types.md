# Types

Request and response types for the LiteForge API, modeled after the OpenAI API.

## Chat Types

### Message

Represents a chat message:

```rust
use liteforge::Message;

let msg = Message::user("Hello!");
let msg = Message::system("You are helpful.");
let msg = Message::assistant("Hi there!");
let msg = Message::tool("tool-call-id", "result data");
```

| Field | Type | Description |
|-------|------|-------------|
| `role` | `String` | `"system"`, `"user"`, `"assistant"`, or `"tool"` |
| `content` | `Option<String>` | Message text |
| `name` | `Option<String>` | Participant name |
| `tool_calls` | `Option<Vec<ToolCall>>` | Tool calls (assistant messages) |
| `tool_call_id` | `Option<String>` | ID of the tool call being responded to |

### ChatCompletionRequest

```rust
use liteforge::ChatCompletionRequest;

let request = ChatCompletionRequest::new("gpt-4", vec![
    Message::user("Hello"),
])
.temperature(0.7)
.max_tokens(1000)
.top_p(0.9)
.stop(vec!["END"])
.tools(tool_definitions);
```

| Field | Type | Default |
|-------|------|---------|
| `model` | `String` | Required |
| `messages` | `Vec<Message>` | Required |
| `temperature` | `Option<f64>` | -- |
| `max_tokens` | `Option<u32>` | -- |
| `stream` | `Option<bool>` | -- |
| `tools` | `Option<Vec<ToolDefinition>>` | -- |
| `top_p` | `Option<f64>` | -- |
| `stop` | `Option<Vec<String>>` | -- |
| `presence_penalty` | `Option<f64>` | -- |
| `frequency_penalty` | `Option<f64>` | -- |
| `user` | `Option<String>` | -- |

### ChatCompletion

Response from a chat completion request:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique completion ID |
| `object` | `String` | Always `"chat.completion"` |
| `created` | `u64` | Unix timestamp |
| `model` | `String` | Model used |
| `choices` | `Vec<Choice>` | Response choices |
| `usage` | `Option<Usage>` | Token usage |

Convenience methods:

- `content() -> Option<&str>` -- first choice's message content
- `message() -> Option<&Message>` -- first choice's full message

### Choice

| Field | Type | Description |
|-------|------|-------------|
| `index` | `u32` | Choice index |
| `message` | `Message` | Response message |
| `finish_reason` | `Option<String>` | `"stop"`, `"tool_calls"`, `"length"` |

### Usage

| Field | Type |
|-------|------|
| `prompt_tokens` | `u32` |
| `completion_tokens` | `u32` |
| `total_tokens` | `u32` |

## Streaming Types

### ChatCompletionChunk

| Field | Type |
|-------|------|
| `id` | `String` |
| `object` | `String` |
| `created` | `u64` |
| `model` | `String` |
| `choices` | `Vec<StreamChoice>` |

- `content() -> Option<&str>` -- first delta's content

### StreamChoice / ChoiceDelta

`StreamChoice` contains `delta: ChoiceDelta` with optional `role`, `content`, and `tool_calls`.

## Model Types

### Model

| Field | Type |
|-------|------|
| `id` | `String` |
| `object` | `String` |
| `created` | `u64` |
| `owned_by` | `String` |

### ModelList

| Method | Returns |
|--------|---------|
| `ids()` | `Vec<&str>` |
| `find(id)` | `Option<&Model>` |

## Tool Types

### ToolCall

| Field | Type |
|-------|------|
| `id` | `String` |
| `call_type` | `String` |
| `function` | `FunctionCall` |

### FunctionCall

| Field | Type |
|-------|------|
| `name` | `String` |
| `arguments` | `String` (JSON) |

- `parse_arguments() -> Result<Value>` -- parse JSON arguments

### ToolDefinition

| Field | Type |
|-------|------|
| `tool_type` | `String` |
| `function` | `FunctionDefinition` |

## Embedding Types

### EmbeddingRequest

```rust
use liteforge::EmbeddingRequest;

let req = EmbeddingRequest::new("text-embedding-3-small", "Hello world");
let req = EmbeddingRequest::batch("text-embedding-3-small", vec!["a", "b"]);
```

### EmbeddingResponse

| Method | Returns |
|--------|---------|
| `embedding()` | `Option<&Vec<f32>>` -- first embedding |
| `embeddings()` | `Vec<&Vec<f32>>` -- all embeddings |

### EmbeddingData

| Field | Type |
|-------|------|
| `object` | `String` |
| `embedding` | `Vec<f32>` |
| `index` | `u32` |
