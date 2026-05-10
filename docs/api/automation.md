# Automation

Scheduled and event-driven task automation for LLM workflows.

## AutomationBuilder

Build automation configurations:

```rust
use liteforge::automation::{AutomationBuilder, PromptTask};

let automation = AutomationBuilder::new("daily-summary")
    .task(PromptTask::new("Summarize the day's events"))
    .schedule(ScheduleConfig::cron("0 9 * * *"))
    .build();
```

## AutomationConfig

```rust
pub struct AutomationConfig {
    pub id: String,
    pub tasks: Vec<AutomationTask>,
    pub schedule: Option<ScheduleConfig>,
}
```

## AutomationRunner

Execute automation workflows:

```rust
use liteforge::automation::AutomationRunner;

let runner = AutomationRunner::new(client);
let record = runner.run(&automation).await?;
println!("Status: {:?}", record.status);
```

## AutomationTask / PromptTask

```rust
pub struct PromptTask {
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
}
```

## TaskContext & TaskOutput

```rust
pub struct TaskContext {
    pub variables: HashMap<String, Value>,
}

pub struct TaskOutput {
    pub content: String,
    pub metadata: HashMap<String, Value>,
}
```

## TaskStatus

| Variant | Description |
|---------|-------------|
| `Pending` | Not yet started |
| `Running` | Currently executing |
| `Completed` | Finished successfully |
| `Failed` | Encountered an error |
| `Cancelled` | Cancelled before completion |

## ExecutionRecord

```rust
pub struct ExecutionRecord {
    pub id: String,
    pub status: TaskStatus,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub output: Option<TaskOutput>,
    pub error: Option<String>,
}
```

## ScheduleConfig

```rust
pub struct ScheduleConfig {
    pub cron: Option<String>,
    pub interval_secs: Option<u64>,
    pub run_once: bool,
}
```

## JavaScript / TypeScript

```javascript
import { AutomationBuilder } from '@forge/sdk';

const automation = new AutomationBuilder('daily-report');
```
