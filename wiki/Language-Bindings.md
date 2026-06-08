# Language Bindings

LiteForge is one Rust engine with thin native bindings. The bindings are **not** reimplementations —
they call straight into the same core, so streaming, tools, guardrails, retries, and config behave
identically everywhere. That's also why CPU‑bound helpers (chunking, guardrails) are **2–16× faster**
than an equivalent pure‑Python implementation: the work runs in Rust, not the interpreter.

| Language | Package | Import | Mechanism | Status |
|---|---|---|---|---|
| Rust | [`liteforge`](https://crates.io/crates/liteforge) | `use liteforge::…` | native | stable |
| Python | [`liteforge`](https://pypi.org/project/liteforge/) | `import liteforge` | PyO3 / maturin | stable |
| JS/TS | [`@seanpoyner/liteforge`](https://www.npmjs.com/package/@seanpoyner/liteforge) | `import … from '@seanpoyner/liteforge'` | napi‑rs | stable |
| Java | `com.liteforge` | `import com.liteforge.*` | JNI | experimental |

Install commands are on **[Installation](Installation)**.

## Rust

The reference surface — sync and async clients, full type system, every module.

```rust
use liteforge::{ForgeClient, Message};

let client = ForgeClient::new();
let r = client.complete(vec![Message::user("Hi")]).unwrap();
println!("{}", r.content().unwrap_or(""));
```

Full API on [docs.rs/liteforge](https://docs.rs/liteforge). Public re‑exports include
`AsyncForgeClient`, `ForgeClient`, `ForgeClientBuilder`, `ForgeConfig`, `Message`,
`ChatCompletion`, `chunk`, the `guardrails::*` and `retry::*` helpers, and the `tools`, `agents`,
`rag`, `mcp`, `orchestration`, `observability` (and more) modules.

## Python

```python
from liteforge import ForgeClient, AsyncForgeClient

client = ForgeClient()
resp = client.complete([{"role": "user", "content": "Hi"}])
print(resp["choices"][0]["message"]["content"])
```

- Messages are plain dicts (`{"role": ..., "content": ...}`); responses are dicts shaped like the
  OpenAI API.
- Async client mirrors the sync one with `await`.
- Tools use `create_tool(...)`, `ToolRegistry`, `ToolExecutor` (see [Tools and Agents](Tools-and-Agents)).
- Wheels: CPython 3.10–3.12 on Linux (manylinux2014), macOS arm64, Windows.

More: [`docs/python/index.md`](https://github.com/seanpoyner/liteforge/blob/main/docs/python/index.md).

## JavaScript / TypeScript

```javascript
import { AsyncForgeClient, createMessageUser } from '@seanpoyner/liteforge';

const client = new AsyncForgeClient();
const resp = await client.complete([createMessageUser('Hi')]);
console.log(resp.choices[0].message.content);
```

- Ships prebuilt natives for linux‑x64‑gnu, darwin‑arm64, win32‑x64‑msvc, plus bundled `.d.ts`
  types.
- Helpers `createMessageSystem` / `createMessageUser` build message objects; streaming uses
  `completeStream(...)` with a `.next()` iterator.

More: [`docs/javascript/index.md`](https://github.com/seanpoyner/liteforge/blob/main/docs/javascript/index.md).

## Java (experimental)

```java
import com.liteforge.ForgeClient;
import com.liteforge.Message;
import java.util.Arrays;

ForgeClient client = ForgeClient.create();
ChatCompletion resp = client.complete(Arrays.asList(Message.user("Hi")));
```

JNI bindings package the core as a cdylib; assemble the JAR via Gradle. The surface is narrower than
the other bindings and the API may change — treat it as preview.

## Feature parity

All three primary bindings expose the same module areas — client (sync/async), streaming, tools,
agents, orchestration, knowledge, RAG, guardrails, MCP, conversation, evals, hooks, skills, events,
HITL, automation, images, pipelines, prompts, scheduler, observability, retry. Differences are
idiomatic, not functional:

- **Streaming** is an async iterator in Python/JS; a `Stream` in Rust.
- **Messages** are dicts in Python, objects via `createMessage*` in JS, `Message::*` constructors in
  Rust.
- **Errors** are `ForgeError` in Rust, exceptions in Python/Java, thrown `Error`s in JS.

## Performance

Guardrails and chunking benchmarks (Rust‑backed bindings vs a pure‑Python baseline) show roughly:

| Operation | Speedup |
|---|---|
| Text chunking | ~15× |
| PII detection | ~3–4× |
| PII redaction | ~3–4× |
| Injection detection | ~2–3× |

Numbers vary by input and host; reproduce with
[`benchmarks/python_sdk_bench.py`](https://github.com/seanpoyner/liteforge/blob/main/benchmarks/python_sdk_bench.py).

Related: **[Quickstart](Quickstart)** · **[Architecture](Architecture)**
