# Agents

LiteForge provides an agent framework for building autonomous tool-calling agents with memory, state management, and orchestration.

## ToolCallingAgent

The primary agent implementation that runs a step loop: think, call tools, observe results, repeat.

```rust
use liteforge::agents::ToolCallingAgent;
use liteforge::tools::ToolRegistry;
use liteforge::{AsyncForgeClient, Message};

let client = AsyncForgeClient::new();
let mut registry = ToolRegistry::new();
// ... register tools ...

let mut agent = ToolCallingAgent::new(client, registry);

let result = agent.run(vec![
    Message::user("What's the weather in Seattle and NYC?")
]).await?;

println!("{:?}", result);
```

## Agent Configuration

```rust
use liteforge::agents::AgentConfig;

let config = AgentConfig::new("my-agent")
    .with_system_prompt("You are a helpful research assistant.")
    .with_model("gpt-4")
    .with_max_steps(10)
    .with_temperature(0.7)
    .with_max_tokens(2000)
    .with_streaming(true)
    .with_tools(vec!["search", "calculator"]);

let agent = ToolCallingAgent::with_config(client, registry, config);
```

## Agent Trait

Build custom agents by implementing the `Agent` trait:

```rust
use liteforge::agents::{Agent, AgentContext, StepResult};
use async_trait::async_trait;

struct MyAgent {
    context: AgentContext,
}

#[async_trait]
impl Agent for MyAgent {
    fn name(&self) -> &str { "my-agent" }
    fn config(&self) -> &AgentConfig { /* ... */ }
    fn context_mut(&mut self) -> &mut AgentContext { &mut self.context }
    fn context(&self) -> &AgentContext { &self.context }

    async fn step(&mut self) -> Result<StepResult, AgentError> {
        // Custom step logic
        Ok(StepResult::Done("Result".to_string()))
    }

    async fn stop(&mut self) { /* cleanup */ }
    async fn reset(&mut self) { /* reset state */ }
}
```

The default `run()` implementation loops `step()` until `StepResult::Done`, `Error`, or max steps.

## Agent Memory

Agents have three memory tiers:

```rust
let memory = &mut agent.context_mut().memory;

// Short-term: conversation messages
memory.add_message(Message::user("Hello"));
let messages = memory.messages();

// Long-term: persistent key-value store
memory.remember("user_name", "Alice");
let name = memory.recall("user_name");

// Working: temporary scratchpad
memory.set_working("current_task", serde_json::json!("research"));
let task = memory.get_working("current_task");
memory.clear_working();
```

## Agent State Machine

| State | Description |
|-------|-------------|
| `Idle` | Not running |
| `Thinking` | Processing LLM response |
| `ExecutingTool` | Running a tool call |
| `WaitingForHuman` | Paused for human approval |
| `Completed` | Finished successfully |
| `Error` | Encountered an error |
| `Stopped` | Manually stopped |

## Step Results

| Variant | Meaning |
|---------|---------|
| `Continue` | Keep running the loop |
| `Done(String)` | Agent finished with a response |
| `ToolCalls(Vec<ToolCall>)` | Agent wants to call tools |
| `WaitForHuman(String)` | Needs human input |
| `Error(String)` | Step failed |

## Multi-Agent Orchestration

See the [Orchestration API](../api/orchestration.md) for running multiple agents together with intent routing, sessions, and workflows.

## Human-in-the-Loop

Gate tool calls on human approval:

```rust
use liteforge::hitl::{ApprovalRequest, RiskBasedHandler, RiskLevel};

let handler = RiskBasedHandler::new(RiskLevel::Medium);
let request = ApprovalRequest::new("delete_file", "Delete user data")
    .context(serde_json::json!({"file": "/data/users.csv"}));

let result = handler.request_approval(request).await;
if result.approved {
    // proceed
}
```

Available handlers:

| Handler | Behavior |
|---------|----------|
| `AutoApprovalHandler` | Approves everything |
| `DenyAllHandler` | Denies everything |
| `QueueApprovalHandler` | Queues for async review |
| `RiskBasedHandler` | Approves below risk threshold |
| `TimeoutApprovalHandler` | Auto-approves after timeout |

## JavaScript / TypeScript

### Agent Configuration & Memory

```javascript
import { JsAgentConfig, JsAgentMemory } from '@forge/sdk';

const config = new JsAgentConfig('travel-assistant');
config.withSystemPrompt('You are an expert travel planner.');
config.withModel('gpt-4');
config.withMaxSteps(10);
config.withTemperature(0.7);
config.withTool('search');
config.withTool('calculator');
```

### Agent Memory

Agents have three memory tiers in JS as well:

```javascript
const memory = new JsAgentMemory();

// Short-term: conversation messages
memory.addMessage('user', 'I want to visit Japan.');
memory.addMessage('assistant', 'When are you planning to go?');
console.log(`Messages: ${memory.messageCount()}`);

// Long-term: persistent key-value store
memory.remember('destination', JSON.stringify('Japan'));
const dest = memory.recall('destination'); // '"Japan"'

// Working: temporary scratchpad
memory.setWorking('current_task', JSON.stringify('planning'));
memory.clearWorking();
```

### Tool-Calling Agent

```javascript
import { AsyncForgeClient, ToolRegistry, ToolCallingAgent, JsAgentConfig } from '@forge/sdk';

const client = new AsyncForgeClient();
const registry = new ToolRegistry();
// ... register tools ...

const config = new JsAgentConfig('my-agent');
config.withMaxSteps(5);

const agent = new ToolCallingAgent(client);
const result = await agent.run([createMessageUser('What is the weather?')]);
```
