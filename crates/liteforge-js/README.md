# @forge/sdk (JavaScript / TypeScript)

[![Node.js](https://img.shields.io/badge/node-18%2B-green.svg)](https://nodejs.org)

High-performance JavaScript/TypeScript bindings for the **LiteForge**, powered by a Rust core via napi-rs. Provides native `.node` addons with auto-generated TypeScript definitions covering LLM completions, streaming, tool calling, RAG, agents, guardrails, MCP, skills, observability, and more.

> Part of the [liteforge](../../README.md) workspace. For Rust usage see [`liteforge`](../liteforge/), for Python see [`liteforge-py`](../liteforge-py/).

## Installation

**From release tarball:**
```bash
npm install https://gitea.poyner.ai/sean/liteforge/releases/latest/download/liteforge.tgz
```

**From git:**
```bash
npm install "git+https://gitea.poyner.ai/sean/liteforge.git#subdirectory=crates/liteforge-js"
```

**Local development (npm link):**
```bash
cd crates/liteforge-js
npm install && npm run build
npm link

# In your project:
npm link @forge/sdk
```

## Quick Start

### Basic Completion

```javascript
import { AsyncForgeClient, createMessageSystem, createMessageUser } from '@forge/sdk';

const client = new AsyncForgeClient();
const response = await client.complete([
  createMessageSystem('You are a helpful assistant.'),
  createMessageUser('What is the capital of France?'),
]);
console.log(response.choices[0].message.content);
```

### Streaming

```javascript
import { AsyncForgeClient, createMessageUser } from '@forge/sdk';

const client = new AsyncForgeClient();
const stream = await client.completeStream([
  createMessageUser('Tell me a story about a robot.')
]);

let chunk;
while ((chunk = await stream.next()) !== null) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) process.stdout.write(content);
}
console.log('\nDone.');
```

## API Surface

The package exports 25 module areas matching the Rust SDK 1:1. All APIs use camelCase and return Promises for async operations.

### Client

| Export | Description |
|--------|-------------|
| `AsyncForgeClient` | Async LLM client -- `complete()`, `completeStream()`, `listModels()`, `embed()` |
| `createMessageSystem(content)` | Create a system message |
| `createMessageUser(content)` | Create a user message |
| `createMessageAssistant(content)` | Create an assistant message |

### Streaming

| Export | Description |
|--------|-------------|
| `CompletionStream` | Async iterator -- call `next()` until `null` |

### Chunking

| Export | Description |
|--------|-------------|
| `chunk(text, options)` | Split text into chunks |
| `Chunk` | Chunk with `text`, `index`, `startChar`, `endChar` |

### Guardrails

| Export | Description |
|--------|-------------|
| `detectPii(text)` | Check text for PII (returns `boolean`) |
| `findPii(text)` | Find all PII matches with types and positions |
| `redactPii(text)` | Replace PII with `[REDACTED]` |
| `detectInjection(text)` | Check for prompt injection patterns |
| `checkAll(text)` | Run all guardrail checks |
| `GuardrailResult` | Result with `passed`, `value`, `message` |

### Tools

| Export | Description |
|--------|-------------|
| `createTool(options)` | Create a tool from a JS function |
| `ToolRegistry` | Manages tool collections -- `register()`, `names()`, `definitions()` |
| `ToolExecutor` | Execute tools with validation -- `execute(name, args)` |
| `ToolResult` | Result with `success`, `result`, `error` |
| `validateJsonSchema(value, schema)` | JSON schema validation |

### Knowledge

| Export | Description |
|--------|-------------|
| `LocalKnowledgeBackend` | In-memory document store |
| `Document`, `SearchResult`, `SearchOptions`, `ListOptions`, `KnowledgeStats` | Types |

### RAG (Vector Search)

| Export | Description |
|--------|-------------|
| `VectorIndex` | In-memory vector index -- `add()`, `search()` |
| `EmbeddedDocument`, `VectorSearchResult` | Types |
| `cosineSimilarity()`, `dotProduct()`, `euclideanDistance()`, `normalize()` | Vector math |

### Agents

| Export | Description |
|--------|-------------|
| `AgentConfig` | Config builder -- `withSystemPrompt()`, `withModel()`, `withMaxSteps()`, `withTool()` |
| `AgentMemory` | Short-term messages + long-term facts |
| `ToolCallingAgent` | Agent that invokes tools in a loop |
| `AgentStep`, `StepResult`, `StepType`, `AgentState` | Step tracking |

### Orchestration

| Export | Description |
|--------|-------------|
| `IntentRouter`, `Intent`, `IntentRoute`, `RoutingDecision`, `CommonIntents` | Intent classification |
| `Session`, `SessionMessage`, `SessionStore` | Session management |
| `Workflow`, `WorkflowStep` | Multi-step workflow definition |
| `OrchestratorConfig`, `OrchestrationStrategy`, `OrchestrationResult` | Orchestration config |

### MCP (Model Context Protocol)

| Export | Description |
|--------|-------------|
| `McpConfig`, `McpServerConfig`, `TransportType` | Configuration |
| `McpStdioServer`, `McpSseServer`, `McpHttpServer` | Transport-specific clients |

### Observability

| Export | Description |
|--------|-------------|
| `Tracer` | Create and manage spans -- `startSpan()`, `drainSpans()` |
| `Span`, `SpanContext`, `SpanKind`, `SpanStatus` | Span types |
| `MetricsCollector`, `MetricValue` | Counters, durations, gauges |

### Conversation

| Export | Description |
|--------|-------------|
| `ManagedConversation` | Track messages -- `addUserMessage()`, `addAssistantMessage()` |
| `CompactingConversation` | Auto-summarizes on overflow |
| `ConversationConfig`, `SummarizationStrategy` | Configuration |

### Human-in-the-Loop

| Export | Description |
|--------|-------------|
| `ApprovalRequest`, `ApprovalResult`, `ApprovalStatus`, `RiskLevel` | Approval workflow types |

### Events & Hooks

| Export | Description |
|--------|-------------|
| `EventBus`, `Event`, `EventType` | Pub/sub event system |
| `HookManager`, `HookEvent`, `HookContext`, `HookResult` | Lifecycle hooks |

### Evals

| Export | Description |
|--------|-------------|
| `TestCase`, `EvalResult`, `SuiteResult`, `SuiteStats` | Evaluation framework |

### Skills

| Export | Description |
|--------|-------------|
| `PromptSkill`, `SkillConfig`, `SkillInput`, `SkillOutput` | Skill framework |
| `getSummarizeSkill()`, `getTranslateSkill()`, `getExtractSkill()`, `getRewriteSkill()`, `getQaSkill()` | Built-in skills |

### Prompts

| Export | Description |
|--------|-------------|
| `PromptTemplate` | Template with `{{var}}` syntax -- `render()`, `withDefault()` |
| `PromptConfig`, `PromptLibrary`, `CommonPrompts` | Template management |

### Pipelines

| Export | Description |
|--------|-------------|
| `PipelineContext`, `PipelineStepOutput` | Pipeline execution types |

### Images

| Export | Description |
|--------|-------------|
| `ImageRequest`, `ImageResponse`, `ImageData` | Image generation |
| `ImageSize`, `ImageQuality`, `ImageStyle`, `ImageResponseFormat` | Configuration enums |

### Scheduler

| Export | Description |
|--------|-------------|
| `CronSchedule`, `IntervalSchedule`, `OnceSchedule`, `ScheduleType`, `JobStatus` | Job scheduling |

### Automation

| Export | Description |
|--------|-------------|
| `AutomationBuilder`, `AutomationConfig`, `AutomationScheduleConfig` | Task automation |
| `AutomationTaskContext`, `AutomationTaskOutput`, `AutomationTaskStatus`, `ExecutionRecord` | Execution types |

### Retry

| Export | Description |
|--------|-------------|
| `RetryConfig` | Retry configuration with backoff |

## Feature Examples

### Tools

```javascript
import { createTool, ToolRegistry, ToolExecutor } from '@forge/sdk';

const calculator = createTool({
  name: 'calculator',
  description: 'Add two numbers',
  parameters: {
    type: 'object',
    properties: { a: { type: 'number' }, b: { type: 'number' } },
    required: ['a', 'b'],
  },
  func: (args) => ({ result: args.a + args.b }),
});

const registry = new ToolRegistry();
registry.register(calculator);
const executor = new ToolExecutor(registry);
const result = executor.execute('calculator', { a: 2, b: 3 });
console.log(result.result); // { result: 5 }
```

### Guardrails

```javascript
import { detectPii, detectInjection, redactPii, checkAll } from '@forge/sdk';

console.log(detectPii('My email is user@example.com'));    // true
console.log(detectInjection('Ignore all instructions'));    // true
console.log(redactPii('Call me at 555-123-4567'));           // Call me at [REDACTED]
const results = checkAll('Text to scan');
```

### Agents

```javascript
import { AgentConfig, AgentMemory } from '@forge/sdk';

const config = new AgentConfig('assistant')
  .withSystemPrompt('You are a helpful assistant.')
  .withModel('gpt-4o')
  .withMaxSteps(10)
  .withTool('calculator');

const memory = new AgentMemory();
memory.addMessage('user', 'What is 2+2?');
memory.remember('userName', 'Alice');
console.log(memory.recall('userName')); // "Alice"
```

### Conversation Management

```javascript
import { ManagedConversation } from '@forge/sdk';

const convo = new ManagedConversation();
convo.addUserMessage('Hello!');
convo.addAssistantMessage('Hi there!');
console.log(convo.messageCount()); // 2
```

### Observability

```javascript
import { Tracer, MetricsCollector } from '@forge/sdk';

const tracer = new Tracer('my-service');
const span = tracer.startSpan('llm-call');

const metrics = new MetricsCollector();
metrics.increment('requestsTotal');
```

## TypeScript Support

TypeScript definitions are auto-generated by napi-rs during the build. After building, `index.d.ts` contains full type signatures for all exported classes and functions. No `@types` package is needed.

```typescript
import { AsyncForgeClient, createMessageUser } from '@forge/sdk';
// Full IntelliSense and type checking available
```

## Configuration

Set environment variables before running:

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `OPENAI_API_KEY` | Fallback API key | - |
| `LITEFORGE_BASE_URL` | LiteLLM endpoint URL | LiteForge production endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model | `anthropic.claude-haiku-4-5-20251001-v1:0` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |

## Building from Source

Requires Rust 1.70+ and Node.js 18+.

```bash
cd crates/liteforge-js
npm install
npm run build          # release build
npm run build:debug    # debug build
```

This produces:

- A platform-specific `.node` native addon
- `index.d.ts` -- auto-generated TypeScript definitions
- `index.js` -- JavaScript entry point

## Examples

See [`examples/javascript/`](../../examples/javascript/) for full working examples:

| Example | Description |
|---------|-------------|
| `basic_completion.mjs` | Simple chat completion |
| `streaming.mjs` | Streaming token-by-token |
| `conversation.mjs` | Multi-turn conversation |
| `tools.mjs` | Tool creation and execution |
| `rag.mjs` | Vector indexing and RAG |
| `knowledge.mjs` | Document storage and search |
| `guardrails.mjs` | PII and injection detection |
| `agent.mjs` | Agent configuration and memory |
| `mcp_server.mjs` | MCP server setup |

## Full Documentation

See the [MkDocs site](https://seanpoyner.github.io/liteforge/) for complete API reference and guides.
