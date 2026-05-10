# JavaScript / TypeScript Bindings

The LiteForge provides native JavaScript/TypeScript bindings via [napi-rs](https://napi.rs), giving you high-performance access to the full SDK from Node.js with auto-generated TypeScript definitions.

## Installation

```bash
cd crates/liteforge-js
npm install
npm run build
```

This produces a native `.node` addon and auto-generated `index.d.ts` type definitions.

## Configuration

The JS bindings read the same environment variables as the Rust core:

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `OPENAI_API_KEY` | Fallback API key | - |
| `LITEFORGE_BASE_URL` | Custom API endpoint | LiteForge endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model | Claude Haiku 4.5 |
| `LITEFORGE_TIMEOUT` | Request timeout (seconds) | 60 |

## Quick Start

```javascript
import { AsyncForgeClient, createMessageUser, createMessageSystem } from '@forge/sdk';

const client = new AsyncForgeClient();

const response = await client.complete([
  createMessageSystem('You are a helpful assistant.'),
  createMessageUser('What is the capital of France?'),
]);

console.log(response.choices[0].message.content);
```

### Builder Pattern

```javascript
const client = AsyncForgeClient.withConfig(
  'your-api-key',     // apiKey
  'gpt-4',            // defaultModel
  'https://api.example.com', // baseUrl
  30,                  // timeoutSecs
);
```

## Streaming

```javascript
const stream = await client.completeStream([
  createMessageUser('Tell me a story'),
]);

let chunk;
while ((chunk = await stream.next()) !== null) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) process.stdout.write(content);
}
```

## API Reference

### Client

- `new AsyncForgeClient()` - Create client from env vars
- `AsyncForgeClient.withConfig(apiKey?, model?, baseUrl?, timeoutSecs?)` - Create with options
- `client.complete(messages)` - Chat completion (returns Promise)
- `client.completeWithModel(model, messages)` - With specific model
- `client.chatCompletions(model, messages, ...)` - Full options
- `client.completeStream(messages)` - Streaming completion
- `client.listModels()` - List available models
- `client.embed(text)` - Single embedding
- `client.embedBatch(texts)` - Batch embeddings

### Messages

- `createMessageUser(content)` - Create user message
- `createMessageSystem(content)` - Create system message
- `createMessageAssistant(content)` - Create assistant message
- `createMessageTool(toolCallId, content)` - Create tool response

### Tools

- `new ToolRegistry()` - Create registry
- `registry.register(name, description, parameters, callback)` - Register tool
- `new ToolExecutor(registry)` - Create executor
- `executor.execute(name, args)` - Execute tool
- `validateJsonSchema(schema, value)` - Validate JSON against schema

### Knowledge

- `new LocalKnowledgeBackend()` - In-memory knowledge base
- `backend.upload(documents)` - Upload documents
- `backend.search(query, options?)` - Search documents
- `backend.get(id)` / `backend.delete(id)` / `backend.update(doc)` / `backend.list(options?)`
- `backend.stats()` / `backend.clear(namespace?)`

### RAG

- `new VectorIndex()` - Create vector index
- `index.add(doc)` / `index.addBatch(docs)` - Add documents
- `index.search(embedding, topK)` - Search by embedding
- `index.searchWithThreshold(embedding, topK, minScore)` - With score filter
- `cosineSimilarity(a, b)` / `dotProduct(a, b)` / `euclideanDistance(a, b)` / `normalize(v)`

### Guardrails

- `detectPii(text)` - Check for PII
- `redactPii(text)` - Redact PII from text
- `findPii(text)` - Find specific PII items
- `detectInjection(text)` - Check for prompt injection
- `checkAll(text)` - Run all guardrail checks

### Conversation

- `new ManagedConversation()` - Track multi-turn conversations
- `new CompactingConversation(config)` - Auto-compacting conversations
- `new ConversationConfig(maxTokens?, targetTokens?, preserveRecent?, strategy?)`

### Agents

- `new JsAgentConfig(name)` - Configure an agent
- `new JsAgentMemory()` - Agent memory (short-term, long-term, working)
- `new ToolCallingAgent(client)` - Tool-calling agent

### Orchestration

- `new IntentRouter()` - Route by intent
- `CommonIntents.greeting(agent)` / `.question()` / `.code()` / `.search()` / `.task()`
- `new SessionStore()` - Manage sessions

### Events & Hooks

- `new EventBus()` - Publish/subscribe events
- `EventType.agentStart()` / `.toolCall()` / `.llmRequest()` / etc.
- `new HookManager()` - Register lifecycle hooks

### Observability

- `new Tracer(serviceName)` - Distributed tracing
- `new MetricsCollector()` - Metrics collection

### MCP

- `McpServerConfig.stdio(name, command)` / `.sse(name, url)` / `.http(name, url)`
- `new McpConfig()` - Manage MCP server configurations

### Prompts

- `new PromptTemplate(template)` - Template with variable substitution
- `new PromptLibrary()` - Store and retrieve templates
- `CommonPrompts.summarize()` / `.translate()` / `.qa()` / `.codeReview()` / etc.

### Other Modules

- **Chunking**: `chunk(text, chunkSize, overlap, strategy)`
- **Retry**: `new RetryConfig(maxRetries)`
- **Scheduler**: `new CronSchedule(expr)` / `new IntervalSchedule(secs)` / `new OnceSchedule()`
- **Images**: `new ImageRequest(prompt)`, `generateImage(client, request)`
- **Evals**: `new EvalSuite(name)`, `suite.addTest(name, input, expected)`
- **Automation**: `new AutomationBuilder(id)`
- **Pipelines**: `new PipelineContext()`
- **Skills**: `getSummarizeSkill()` / `getTranslateSkill()` / `getExtractSkill()` / etc.
- **HITL**: `createApprovalRequest(action, description, riskLevel)`
