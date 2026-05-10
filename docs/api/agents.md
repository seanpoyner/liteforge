# Agents API

Agent framework for autonomous tool-calling loops with memory and state.

## AgentConfig

```rust
let config = AgentConfig::new("my-agent")
    .with_system_prompt("You are helpful.")
    .with_model("gpt-4")
    .with_max_steps(10)
    .with_temperature(0.7)
    .with_max_tokens(2000)
    .with_streaming(true)
    .with_tool("search")
    .with_tools(vec!["calc", "weather"]);
```

## Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn config(&self) -> &AgentConfig;
    fn context_mut(&mut self) -> &mut AgentContext;
    fn context(&self) -> &AgentContext;
    async fn run(&mut self, messages: Vec<Message>) -> Result<String, AgentError>;
    async fn step(&mut self) -> Result<StepResult, AgentError>;
    async fn stop(&mut self);
    async fn reset(&mut self);
}
```

The default `run()` implementation loops `step()` until `Done`, `Error`, or max steps exceeded.

## ToolCallingAgent

Built-in agent that automatically calls tools and feeds results back to the LLM:

| Method | Description |
|--------|-------------|
| `new(client, registry)` | Create with defaults |
| `with_config(client, registry, config)` | Create with config |
| `with_system_prompt(client, registry, prompt)` | Create with system prompt |
| `with_max_steps(client, registry, max)` | Create with step limit |
| `history()` | Get conversation history |
| `clear_history()` | Clear history |

## AgentContext

| Method | Description |
|--------|-------------|
| `new(name)` | Create context |
| `can_continue()` | Check step limit |
| `increment_step()` | Advance step counter |
| `get_messages()` | Get all messages |
| `reset()` | Reset state |

## AgentMemory

| Method | Description |
|--------|-------------|
| `add_message(msg)` | Add to short-term |
| `messages()` | Get short-term messages |
| `clear_short_term()` | Clear messages |
| `remember(key, value)` | Store in long-term |
| `recall(key)` | Retrieve from long-term |
| `forget(key)` | Remove from long-term |
| `set_working(key, value)` | Set working memory |
| `get_working(key)` | Get working memory |
| `clear_working()` | Clear working memory |

## AgentState

`Idle` | `Thinking` | `ExecutingTool` | `WaitingForHuman` | `Completed` | `Error` | `Stopped`

## StepResult

| Variant | Description |
|---------|-------------|
| `Continue` | Keep looping |
| `Done(String)` | Final response |
| `ToolCalls(Vec<ToolCall>)` | Wants to call tools |
| `WaitForHuman(String)` | Needs human input |
| `Error(String)` | Step failed |

## AgentStep

Record of a single agent step with `step_number`, `step_type`, `input`, `output`, `result`, `duration`, `tokens`, and `metadata`.

## TokenUsage

```rust
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```
