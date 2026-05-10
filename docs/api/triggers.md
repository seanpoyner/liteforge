# Triggers

Event-driven triggers for starting agent execution based on external events.

## Trigger Trait

```rust
pub trait Trigger: Send + Sync {
    fn id(&self) -> &str;
    fn trigger_type(&self) -> &str;
    fn status(&self) -> TriggerStatus;
    fn start(&mut self) -> Result<(), TriggerError>;
    fn stop(&mut self) -> Result<(), TriggerError>;
    fn pause(&mut self) -> Result<(), TriggerError>;
    fn resume(&mut self) -> Result<(), TriggerError>;
    fn poll(&self) -> Option<TriggerEvent>;
    fn has_pending(&self) -> bool;
}
```

## TriggerStatus

| Variant | Description |
|---------|-------------|
| `Stopped` | Not running (default) |
| `Running` | Actively listening for events |
| `Paused` | Temporarily paused |
| `Error` | In error state |

## TriggerEvent

```rust
pub struct TriggerEvent {
    pub trigger_id: String,
    pub event_type: String,
    pub payload: Value,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}
```

| Method | Description |
|--------|-------------|
| `new(trigger_id, event_type, payload)` | Create event |
| `with_metadata(key, value)` | Add metadata |
| `payload_as::<T>()` | Deserialize payload |

## Built-in Triggers

### FileWatchTrigger

Watch filesystem for changes:

```rust
use liteforge::triggers::FileWatchTrigger;

let mut trigger = FileWatchTrigger::new("watcher-1", "/path/to/watch");
trigger.start()?;
if let Some(event) = trigger.poll() {
    println!("File event: {:?}", event);
}
```

### WebhookTrigger

Listen for incoming HTTP webhooks:

```rust
use liteforge::triggers::{WebhookTrigger, WebhookConfig};

let config = WebhookConfig {
    port: 8080,
    path: "/webhook".to_string(),
    secret: Some("my-secret".to_string()),
};
let mut trigger = WebhookTrigger::new("webhook-1", config);
trigger.start()?;
```

### QueueTrigger

Poll a message queue:

```rust
use liteforge::triggers::QueueTrigger;

let mut trigger = QueueTrigger::new("queue-1", "my-queue-url");
trigger.start()?;
```

### ScheduleTrigger

Trigger on a schedule (cron or interval):

```rust
use liteforge::triggers::ScheduleTrigger;

let mut trigger = ScheduleTrigger::new("schedule-1", "0 */5 * * * *"); // Every 5 minutes
trigger.start()?;
```

## TriggerManager

Manage multiple triggers:

```rust
use liteforge::triggers::TriggerManager;

let mut manager = TriggerManager::new();
let handle = manager.add(Box::new(file_trigger));
manager.start_all()?;

// Poll for events from all triggers
let events = manager.poll_all();
```

## TriggerError

| Method | Description |
|--------|-------------|
| `config(msg)` | Configuration error |
| `runtime(msg)` | Runtime error |
| `already_running()` | Trigger already running |
| `not_running()` | Trigger not running |

### TriggerErrorKind

| Variant | Description |
|---------|-------------|
| `Configuration` | Invalid configuration |
| `Runtime` | Runtime failure |
| `AlreadyRunning` | Trigger is already active |
| `NotRunning` | Trigger is not active |
| `Io` | I/O error |
| `Network` | Network error |
