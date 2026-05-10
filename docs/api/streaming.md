# Streaming API

Low-level SSE (Server-Sent Events) parsing for chat completion streams.

## Functions

### `parse_sse_line`

```rust
pub fn parse_sse_line(line: &str) -> Result<Option<ChatCompletionChunk>>
```

Parse a single SSE `data:` line into a `ChatCompletionChunk`. Returns `Ok(None)` for `[DONE]` sentinel or non-data lines.

### `parse_sse_stream`

```rust
pub fn parse_sse_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>>
) -> impl Stream<Item = Result<ChatCompletionChunk>>
```

Transform a raw byte stream (from `reqwest`) into a stream of `ChatCompletionChunk` values. Handles line splitting, `data:` prefix stripping, and `[DONE]` detection.

## Usage

Typically accessed through `AsyncForgeClient` methods rather than directly:

```rust
let mut stream = client.complete_stream(messages).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(text) = chunk.content() {
        print!("{text}");
    }
}
```

See the [Streaming Guide](../guides/streaming.md) for higher-level usage patterns.
