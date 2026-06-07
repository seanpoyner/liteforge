# Streaming

LiteForge supports Server-Sent Events (SSE) for real-time token-by-token responses.

## Async Streaming

Use `AsyncForgeClient` for streaming completions:

```rust
use liteforge::{AsyncForgeClient, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), liteforge::ForgeError> {
    let client = AsyncForgeClient::new();

    let mut stream = client.complete_stream(vec![
        Message::system("You are a helpful assistant."),
        Message::user("Write a haiku about Rust programming"),
    ]).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if let Some(text) = chunk.content() {
                    print!("{text}");
                }
            }
            Err(e) => eprintln!("\nStream error: {e}"),
        }
    }
    println!();
    Ok(())
}
```

## Stream with Model Selection

```rust
let request = ChatCompletionRequest::new("gpt-4", vec![
    Message::user("Explain quantum computing"),
])
.temperature(0.7)
.max_tokens(500);

let mut stream = client.chat_completions_stream(request).await?;
```

## ChatCompletionChunk

Each streamed chunk contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Completion identifier |
| `object` | `String` | Always `"chat.completion.chunk"` |
| `created` | `u64` | Unix timestamp |
| `model` | `String` | Model used |
| `choices` | `Vec<StreamChoice>` | Partial choices |

Each `StreamChoice` contains a `ChoiceDelta` with optional `role`, `content`, and `tool_calls` fields.

Use `chunk.content()` as a convenience method to extract the first choice's content.

## SSE Parsing

The SDK provides low-level SSE parsing utilities:

```rust
use liteforge::streaming::{parse_sse_line, parse_sse_stream};

// Parse a single SSE line
let chunk = parse_sse_line("data: {\"id\":\"chatcmpl-...\", ...}")?;

// Parse a byte stream into ChatCompletionChunk stream
let chunk_stream = parse_sse_stream(byte_stream);
```

## Python Streaming

```python
from liteforge import AsyncForgeClient
import asyncio

async def main():
    client = AsyncForgeClient()
    stream = await client.complete_stream([
        {"role": "user", "content": "Tell me a story"}
    ])
    async for chunk in stream:
        if chunk.get("content"):
            print(chunk["content"], end="", flush=True)

asyncio.run(main())
```

## JavaScript / TypeScript Streaming

```javascript
import { AsyncForgeClient, createMessageUser } from '@seanpoyner/liteforge';

const client = new AsyncForgeClient();
const stream = await client.completeStream([
  createMessageUser('Tell me a story'),
]);

let chunk;
while ((chunk = await stream.next()) !== null) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) process.stdout.write(content);
}
```

The `completeStream()` method returns a `CompletionStream` object. Call `.next()` in a loop — it resolves to a `ChatCompletionChunk` or `null` when the stream ends.
