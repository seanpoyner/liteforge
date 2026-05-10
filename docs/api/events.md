# Events & Hooks API

Publish-subscribe event system and lifecycle hooks.

## EventBus

```rust
use liteforge::events::{EventBus, Event, EventType};

let bus = EventBus::new();
// or with channel capacity:
let bus = EventBus::with_capacity(1000);
```

| Method | Description |
|--------|-------------|
| `subscribe_all()` | Subscribe to all events |
| `subscribe(event_type)` | Subscribe to one type |
| `subscribe_many(types)` | Subscribe to multiple types |
| `publish(event)` | Publish an event |
| `publish_batch(events)` | Publish multiple events |
| `subscriber_count()` | Total subscriber count |
| `subscriber_count_for(type)` | Subscribers for a type |

## EventType

| Type | Category |
|------|----------|
| `AgentStart`, `AgentEnd`, `AgentStep`, `AgentError` | Agent |
| `ToolCall`, `ToolResult`, `ToolError` | Tool |
| `LlmRequest`, `LlmResponse`, `LlmStream`, `LlmError` | LLM |
| `KnowledgeSearch`, `KnowledgeUpload`, `KnowledgeDelete` | Knowledge |
| `Custom` | User-defined |

Category helpers: `is_agent_event()`, `is_tool_event()`, `is_llm_event()`.

## Event

```rust
let event = Event::new(EventType::Custom)
    .with_data(EventData::String("hello".into()));

// Factory methods
Event::agent_start(agent_name);
Event::tool_call(tool_name, args);
Event::llm_request(model, message_count);
Event::custom(data);
```

| Field | Type |
|-------|------|
| `id` | `String` |
| `event_type` | `EventType` |
| `data` | `EventData` |
| `timestamp` | `u64` |
| `source` | `Option<String>` |
| `correlation_id` | `Option<String>` |
| `metadata` | `HashMap<String, Value>` |

## Subscription / FilteredSubscription

```rust
let sub = bus.subscribe(EventType::ToolCall);

// Async receive
let event = sub.recv().await;

// Non-blocking try
if let Some(event) = sub.try_recv() { /* ... */ }
```

---

## HookManager

```rust
use liteforge::hooks::{HookManager, Hook, HookEvent, HookContext, HookResult};

let mut manager = HookManager::new();
manager.register(Box::new(MyHook));

let result = manager.run(HookEvent::BeforeToolCall, &context);
```

| Method | Description |
|--------|-------------|
| `register(hook)` | Add a hook |
| `unregister(name)` | Remove by name |
| `run(event, context)` | Run hooks (immutable context) |
| `run_mut(event, context)` | Run hooks (mutable context) |
| `len()` | Number of hooks |
| `hook_names()` | List hook names |
| `clear()` | Remove all hooks |

## Hook Trait

```rust
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 { 0 }

    fn before_tool_call(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn after_tool_call(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn before_llm_request(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn after_llm_response(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    // ... 6 more event-specific methods
}
```

## HookResult

| Variant | Effect |
|---------|--------|
| `Continue` | Proceed normally |
| `ContinueWith(Value)` | Proceed with modified data |
| `Skip` | Skip the operation |
| `SkipWith(Value)` | Skip and return replacement |
| `Abort(String)` | Abort with error message |

## HookEvent

`BeforeAgentStart` | `AfterAgentEnd` | `BeforeAgentStep` | `AfterAgentStep` | `BeforeToolCall` | `AfterToolCall` | `BeforeLlmRequest` | `AfterLlmResponse` | `BeforeKnowledgeSearch` | `AfterKnowledgeSearch`
