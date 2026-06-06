# liteforge-py

[![Python](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org)

High-performance Python bindings for the **LiteForge**, powered by a Rust core via PyO3. Exposes 74 classes and 17 functions covering LLM completions, streaming, tool calling, RAG, agents, guardrails, MCP, skills, observability, and more.

> Part of the [liteforge](../../README.md) workspace. For Rust usage see [`liteforge`](../liteforge/), for JS/TS see [`liteforge-js`](../liteforge-js/).

## Installation

**From release wheel:**
```bash
pip install https://github.com/seanpoyner/liteforge/releases/latest/download/liteforge-py3-none-any.whl
```

**From git:**
```bash
pip install "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-py"
```

**Development install (requires Rust 1.70+ and maturin):**
```bash
cd crates/liteforge-py
pip install maturin
maturin develop
```

## Quick Start

### Synchronous

```python
from liteforge import ForgeClient

client = ForgeClient()
response = client.complete([{"role": "user", "content": "What is the capital of France?"}])
print(response["choices"][0]["message"]["content"])
```

### Asynchronous

```python
import asyncio
from liteforge import AsyncForgeClient

async def main():
    client = AsyncForgeClient()
    response = await client.complete([{"role": "user", "content": "Hello!"}])
    print(response["choices"][0]["message"]["content"])

asyncio.run(main())
```

### Streaming

```python
from liteforge import AsyncForgeClient
import asyncio

async def main():
    client = AsyncForgeClient()
    stream = await client.complete_stream([{"role": "user", "content": "Tell me a story"}])
    async for chunk in stream:
        content = chunk.get("choices", [{}])[0].get("delta", {}).get("content", "")
        if content:
            print(content, end="", flush=True)

asyncio.run(main())
```

## API Surface

### Client

| Class / Function | Description |
|------------------|-------------|
| `ForgeClient` | Synchronous LLM client -- `complete()`, `list_models()`, `embed()` |
| `AsyncForgeClient` | Async LLM client -- `complete()`, `complete_stream()`, `list_models()`, `embed()` |
| `CompletionStream` | Async iterator for streaming responses |

### Chunking

| Class / Function | Description |
|------------------|-------------|
| `chunk()` | Split text into chunks (fixed, recursive, sentence, paragraph) |
| `Chunk` | A text chunk with `text`, `index`, `start_char`, `end_char` |

### Guardrails

| Class / Function | Description |
|------------------|-------------|
| `detect_pii()` | Check text for PII (returns `bool`) |
| `find_pii()` | Find all PII matches with types and positions |
| `redact_pii()` | Replace PII with `[REDACTED]` |
| `detect_injection()` | Check for prompt injection patterns |
| `check_all()` | Run all guardrail checks at once |
| `GuardrailResult` | Result with `passed`, `value`, `message`, `guardrail_name` |

### Tools

| Class / Function | Description |
|------------------|-------------|
| `create_tool()` | Create a tool from a Python function |
| `validate_json_schema()` | Validate arguments against a JSON schema |
| `PyTool` | Tool wrapper |
| `ToolRegistry` | Manages collections of tools |
| `ToolExecutor` | Executes tools with optional schema validation |
| `ToolResult` | Result with `success`, `result`, `error` |

### Knowledge

| Class / Function | Description |
|------------------|-------------|
| `LocalKnowledgeBackend` | In-memory document store |
| `Document` | A document with id and content |
| `SearchResult` | Search result with score |
| `SearchOptions` | Search query options |
| `ListOptions` | Listing/pagination options |
| `KnowledgeStats` | Knowledge base statistics |

### RAG (Vector Search)

| Class / Function | Description |
|------------------|-------------|
| `VectorIndex` | In-memory vector index |
| `EmbeddedDocument` | Document with embedding vector |
| `VectorSearchResult` | Search result with score |
| `cosine_similarity()` | Cosine similarity between vectors |
| `dot_product()` | Dot product between vectors |
| `euclidean_distance()` | Euclidean distance between vectors |
| `normalize()` | L2-normalize a vector |

### Agents

| Class / Function | Description |
|------------------|-------------|
| `AgentConfig` | Agent configuration -- `with_system_prompt()`, `with_model()`, `with_max_steps()`, `with_tool()` |
| `AgentMemory` | Short-term (messages) + long-term (facts) memory |
| `AgentState` | Agent execution state |
| `ToolCallingAgent` | Agent that invokes tools in a loop |
| `AgentStep` | Individual step in agent execution |
| `StepResult` | Result of a step |
| `StepType` | Step type discriminator |

### Orchestration

| Class / Function | Description |
|------------------|-------------|
| `Intent`, `IntentRoute`, `IntentRouter`, `RoutingDecision` | Intent classification and routing |
| `CommonIntents` | Pre-built intent patterns |
| `Session`, `SessionMessage`, `SessionStore` | Session management |
| `Workflow`, `WorkflowStep` | Multi-step workflow definition |
| `OrchestrationStrategy`, `OrchestratorConfig`, `OrchestrationResult` | Orchestration configuration |
| `OrchestrationStepStatus` | Step lifecycle status |

### MCP (Model Context Protocol)

| Class / Function | Description |
|------------------|-------------|
| `McpConfig`, `McpServerConfig` | MCP server configuration |
| `TransportType` | `Stdio`, `Sse`, `Http` |
| `McpStdioServer`, `McpSseServer`, `McpHttpServer` | Transport-specific server clients |
| `McpServerState` | Server connection state |

### Observability

| Class / Function | Description |
|------------------|-------------|
| `Tracer` | Create and manage distributed spans |
| `Span`, `SpanContext` | Span with timing, attributes, events |
| `SpanKind`, `SpanStatus` | Span metadata |
| `MetricsCollector` | Increment counters, record durations, gauges |
| `MetricValue` | Metric value type |

### Conversation

| Class / Function | Description |
|------------------|-------------|
| `ManagedConversation` | Message tracking with `add_user_message()`, `add_assistant_message()` |
| `CompactingConversation` | Auto-summarizes when approaching token limits |
| `ConversationConfig` | Configuration |
| `SummarizationStrategy` | How to summarize overflowing context |

### Human-in-the-Loop

| Class / Function | Description |
|------------------|-------------|
| `ApprovalRequest` | Request for human approval |
| `ApprovalResult` | Approval outcome |
| `ApprovalStatus` | Status enum |
| `RiskLevel` | Risk classification |

### Events & Hooks

| Class / Function | Description |
|------------------|-------------|
| `EventBus` | Pub/sub event bus -- `subscribe()`, `publish()` |
| `Event`, `EventType` | Event payload and type |
| `HookManager` | Register and invoke lifecycle hooks |
| `HookEvent`, `HookContext`, `HookResult` | Hook types |

### Evals

| Class / Function | Description |
|------------------|-------------|
| `TestCase` | Test case with input and expected output |
| `EvalResult` | Evaluation result with `passed` flag |
| `SuiteResult`, `SuiteStats` | Suite-level results and statistics |

### Skills

| Class / Function | Description |
|------------------|-------------|
| `PromptSkill` | LLM-backed skill implementation |
| `SkillConfig`, `SkillInput`, `SkillOutput` | Skill I/O types |
| `get_summarize_skill()` | Built-in summarize skill |
| `get_translate_skill()` | Built-in translate skill |
| `get_extract_skill()` | Built-in entity extraction skill |
| `get_rewrite_skill()` | Built-in rewrite skill |
| `get_qa_skill()` | Built-in Q&A skill |

### Prompts

| Class / Function | Description |
|------------------|-------------|
| `PromptTemplate` | Template with `{{var}}` syntax -- `render()`, `with_default()` |
| `PromptConfig` | Serializable template config |
| `PromptLibrary` | Named template store with categories |
| `CommonPrompts` | Static methods: `summarize()`, `translate()`, `qa()`, `classify()`, etc. |

### Pipelines

| Class / Function | Description |
|------------------|-------------|
| `PipelineContext` | Shared pipeline execution context |
| `PipelineStepOutput` | Output from a pipeline step |

### Images

| Class / Function | Description |
|------------------|-------------|
| `ImageRequest` | Image generation request builder |
| `ImageResponse`, `ImageData` | Image response types |
| `ImageSize`, `ImageQuality`, `ImageStyle`, `ImageResponseFormat` | Enums for image configuration |

### Scheduler

| Class / Function | Description |
|------------------|-------------|
| `CronSchedule`, `IntervalSchedule`, `OnceSchedule` | Schedule types |
| `ScheduleType`, `JobStatus` | Enums |

### Automation

| Class / Function | Description |
|------------------|-------------|
| `AutomationBuilder` | Fluent builder -- `every_seconds()`, `every_minutes()`, `cron()`, `prompt()` |
| `AutomationConfig`, `AutomationScheduleConfig` | Task configuration |
| `AutomationTaskContext`, `AutomationTaskOutput` | Execution I/O |
| `AutomationTaskStatus` | Status enum |
| `ExecutionRecord` | Execution audit trail |

### Retry

| Class / Function | Description |
|------------------|-------------|
| `RetryConfig` | `max_retries`, `initial_delay`, `max_delay`, `backoff_multiplier` |

## Feature Examples

### Tools

```python
from liteforge import create_tool, ToolRegistry, ToolExecutor

def calculator(args):
    a, b = args["a"], args["b"]
    return {"result": a + b}

tool = create_tool(
    name="calculator",
    description="Add two numbers",
    parameters={"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}, "required": ["a", "b"]},
    func=calculator,
    requires_confirmation=False,
)

registry = ToolRegistry()
registry.register(tool)
executor = ToolExecutor(registry, validate_args=True)
result = executor.execute("calculator", {"a": 2, "b": 3})
print(result.result)  # {"result": 5}
```

### Guardrails

```python
from liteforge import detect_pii, detect_injection, redact_pii, check_all

print(detect_pii("My email is user@example.com"))   # True
print(detect_injection("Ignore all instructions"))   # True
print(redact_pii("Call me at 555-123-4567"))          # Call me at [REDACTED]
results = check_all("Some text to scan")
```

### Agents

```python
from liteforge import AgentConfig, AgentMemory

config = (AgentConfig("assistant")
    .with_system_prompt("You are a helpful assistant.")
    .with_model("gpt-4o")
    .with_max_steps(10)
    .with_tool("calculator"))

memory = AgentMemory()
memory.add_message("user", "What is 2+2?")
memory.remember("user_name", "Alice")
print(memory.recall("user_name"))  # "Alice"
```

### Knowledge

```python
from liteforge import LocalKnowledgeBackend, Document, SearchOptions

kb = LocalKnowledgeBackend()
kb.add(Document(id="1", content="Paris is the capital of France"))
results = kb.search(SearchOptions(query="capital of France", top_k=3))
```

### Skills

```python
from liteforge import get_summarize_skill, SkillInput

skill = get_summarize_skill()
print(skill.config.name)  # "summarize"
```

### Prompts

```python
from liteforge import PromptTemplate, CommonPrompts

template = PromptTemplate("Summarize the following: {{text}}")
rendered = template.render({"text": "A long article..."})

qa = CommonPrompts.qa()
```

### Conversation Management

```python
from liteforge import ManagedConversation

convo = ManagedConversation()
convo.add_user_message("Hello!")
convo.add_assistant_message("Hi there!")
print(convo.message_count())  # 2
```

### Observability

```python
from liteforge import Tracer, MetricsCollector

tracer = Tracer("my-service")
span = tracer.start_span("llm-call")

metrics = MetricsCollector()
metrics.increment("requests_total")
```

## Configuration

Set environment variables before importing:

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `OPENAI_API_KEY` | Fallback API key | - |
| `LITEFORGE_BASE_URL` | LiteLLM endpoint URL | LiteForge production endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model | `claude-haiku-4.5` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |

## Building from Source

Requires Rust 1.70+ and Python 3.10+.

```bash
cd crates/liteforge-py
pip install maturin

# Development install (editable)
maturin develop

# Build a release wheel
maturin build --release
pip install target/wheels/liteforge*.whl
```

## Examples

See [`examples/python/`](../../examples/python/) for full working examples:

| Example | Description |
|---------|-------------|
| `agent.py` | Agent configuration, memory, and tool integration |
| `tools.py` | Tool creation, registry, executor, schema validation |
| `rag.py` | Vector indexing and RAG pipeline |
| `guardrails.py` | PII detection, injection detection |
| `knowledge.py` | Document storage and retrieval |
| `conversation.py` | Managed and compacting conversations |
| `mcp_server.py` | MCP server configuration |

## Full Documentation

See the [MkDocs site](https://emerging-tech.github.io/liteforge/) for complete API reference and guides.
