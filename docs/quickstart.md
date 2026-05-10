# Quick Start

## Rust (Synchronous)

```rust
use liteforge::{ForgeClient, Message};

fn main() {
    let client = ForgeClient::new();

    let response = client.complete(vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is the capital of France?"),
    ]).unwrap();

    println!("{}", response.content().unwrap_or(""));
}
```

## Rust (Async)

```rust
use liteforge::{AsyncForgeClient, Message};

#[tokio::main]
async fn main() {
    let client = AsyncForgeClient::new();

    let response = client.complete(vec![
        Message::user("Hello!")
    ]).await.unwrap();

    println!("{}", response.content().unwrap_or(""));
}
```

## Rust (Streaming)

```rust
use liteforge::{AsyncForgeClient, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let client = AsyncForgeClient::new();

    let mut stream = client.complete_stream(vec![
        Message::user("Tell me a story")
    ]).await.unwrap();

    while let Some(chunk) = stream.next().await {
        if let Ok(chunk) = chunk {
            if let Some(text) = chunk.content() {
                print!("{text}");
            }
        }
    }
}
```

## Python (Sync)

```python
from liteforge import ForgeClient

client = ForgeClient()
response = client.complete([{"role": "user", "content": "Hello!"}])
print(response["choices"][0]["message"]["content"])
```

## Python (Async)

```python
import asyncio
from liteforge import AsyncForgeClient

async def main():
    client = AsyncForgeClient()
    response = await client.complete([
        {"role": "user", "content": "Hello!"}
    ])
    print(response["choices"][0]["message"]["content"])

asyncio.run(main())
```

## JavaScript / TypeScript

```javascript
import { AsyncForgeClient, createMessageUser } from '@forge/sdk';

const client = new AsyncForgeClient();
const response = await client.complete([
  createMessageUser('What is the capital of France?'),
]);
console.log(response.choices[0].message.content);
```

## JavaScript (Streaming)

```javascript
import { AsyncForgeClient, createMessageUser } from '@forge/sdk';

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

## Builder Pattern

For more control over client configuration:

```rust
let client = ForgeClient::builder()
    .api_key("your-key")
    .default_model("gpt-4")
    .base_url("https://api.example.com")
    .timeout_secs(30)
    .build();
```

## Running Examples

The repository includes runnable examples:

```bash
# Basic synchronous completion
cargo run --example basic_completion

# Async streaming completion
cargo run --example streaming

# JavaScript examples (after building crates/liteforge-js)
node examples/javascript/basic_completion.mjs
node examples/javascript/streaming.mjs
node examples/javascript/tools.mjs
node examples/javascript/agent.mjs
```
