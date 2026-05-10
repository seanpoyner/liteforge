# Orchestration API

Multi-agent orchestration with intent routing, sessions, and workflows.

## AgentOrchestrator

Coordinates multiple agents:

```rust
use liteforge::orchestration::{AgentOrchestrator, OrchestratorConfig, OrchestrationStrategy};

let config = OrchestratorConfig {
    strategy: OrchestrationStrategy::IntentBased,
    // ...
};

let orchestrator = AgentOrchestrator::new(config);
```

### OrchestrationStrategy

- Route messages to agents based on intent, round-robin, or custom logic.

### OrchestratedAgent / ToolCallingAgentWrapper

Wrappers that make agents compatible with the orchestrator.

## Intent Router

Route user messages to the appropriate agent:

```rust
use liteforge::orchestration::{IntentRouter, IntentRoute, CommonIntents};

let mut router = IntentRouter::new();

router.add_route(IntentRoute {
    intent: CommonIntents::QUESTION,
    agent: "qa-agent",
    // ...
});

let decision: RoutingDecision = router.route("What is Rust?");
```

### CommonIntents

Pre-defined intent constants for common use cases.

## Sessions

Manage conversation sessions across multiple interactions:

```rust
use liteforge::orchestration::{Session, SessionStore, get_or_create};

let store = SessionStore::new();
let session = get_or_create(&store, "user-123");

session.add_message(SessionMessage { /* ... */ });
```

## Workflows

Define multi-step workflows:

```rust
use liteforge::orchestration::{Workflow, WorkflowStep, WorkflowExecutor};

let workflow = Workflow::new("onboarding")
    .add_step(WorkflowStep { /* ... */ });

let executor = WorkflowExecutor::new();
let result: WorkflowResult = executor.execute(workflow).await?;
```

### StepExecutor Trait

```rust
pub trait StepExecutor: Send + Sync {
    async fn execute(&self, context: ExecutionContext) -> StepExecutionResult;
}
```

Built-in: `EchoExecutor` (echoes input back).

### WorkflowError

| Variant | Description |
|---------|-------------|
| `StepFailed` | A step returned an error |
| `Timeout` | Step or workflow timed out |
| `Cancelled` | Workflow was cancelled |
