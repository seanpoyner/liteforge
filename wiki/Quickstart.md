# Quickstart

Your first completion in Rust, Python, and JavaScript. Five minutes, three languages.

## 1. Set credentials

LiteForge talks to any OpenAI‑compatible endpoint. Point it at one with two environment variables
(see [Configuration](Configuration) for the full resolution order):

```bash
export LITEFORGE_API_KEY="sk-…"
export LITEFORGE_BASE_URL="https://api.openai.com/v1"   # or a LiteLLM proxy, Ollama, etc.
export LITEFORGE_DEFAULT_MODEL="gpt-4o-mini"            # optional
```

> Running models locally? See **[LiteLLM and Ollama](LiteLLM-and-Ollama)** to point LiteForge at a
> local proxy or `http://localhost:11434/v1` with no API key.

## 2. Make a call

### Rust (sync)

```rust
use liteforge::{ForgeClient, Message};

fn main() {
    let client = ForgeClient::new(); // reads env / .env

    let response = client.complete(vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is the capital of France?"),
    ]).unwrap();

    println!("{}", response.content().unwrap_or(""));
}
```

### Rust (async)

```rust
use liteforge::{AsyncForgeClient, Message};

#[tokio::main]
async fn main() {
    let client = AsyncForgeClient::new();
    let response = client
        .complete(vec![Message::user("Hello!")])
        .await
        .unwrap();
    println!("{}", response.content().unwrap_or(""));
}
```

### Python

```python
from liteforge import ForgeClient

client = ForgeClient()  # reads env / .env
response = client.complete([
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the capital of France?"},
])
print(response["choices"][0]["message"]["content"])
```

Async variant:

```python
import asyncio
from liteforge import AsyncForgeClient

async def main():
    client = AsyncForgeClient()
    response = await client.complete([{"role": "user", "content": "Hello!"}])
    print(response["choices"][0]["message"]["content"])

asyncio.run(main())
```

### JavaScript / TypeScript

```javascript
import { AsyncForgeClient, createMessageSystem, createMessageUser } from '@seanpoyner/liteforge';

const client = new AsyncForgeClient();
const response = await client.complete([
  createMessageSystem('You are a helpful assistant.'),
  createMessageUser('What is the capital of France?'),
]);
console.log(response.choices[0].message.content);
```

## 3. Configure the client explicitly

Prefer not to use env vars? Use the builder:

```rust
let client = ForgeClient::builder()
    .api_key("sk-…")
    .base_url("https://api.openai.com/v1")
    .default_model("gpt-4o-mini")
    .timeout_secs(30)
    .build();
```

```javascript
const client = AsyncForgeClient.withConfig(
  'sk-…',                       // apiKey
  'gpt-4o-mini',                // defaultModel
  'https://api.openai.com/v1',  // baseUrl
  30,                           // timeoutSecs
);
```

## 4. Or just use the CLI

```bash
forge chat "What is the capital of France?"
forge chat --stream "Tell me a story"
forge chat -i                  # interactive REPL
```

## Run the bundled examples

```bash
git clone https://github.com/seanpoyner/liteforge.git && cd liteforge
cargo run --example basic_completion
cargo run --example streaming
node examples/javascript/basic_completion.mjs
python examples/python/hello.py
```

## Where next

- Stream tokens incrementally → **[Streaming](Streaming)**
- Let the model call your functions → **[Tools and Agents](Tools-and-Agents)**
- Answer from your own documents → **[RAG](RAG)**
- Full config & secrets → **[Configuration](Configuration)**
