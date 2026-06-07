# Human-in-the-Loop (HITL)

Gate agent actions on human approval with configurable approval handlers.

## ApprovalRequest

```rust
use liteforge::hitl::{ApprovalRequest, RiskLevel};

let request = ApprovalRequest::new("delete_file", "Delete user data file")
    .risk_level(RiskLevel::High)
    .context(serde_json::json!({"file": "/data/users.csv"}));
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `action` | `String` | Action being requested |
| `description` | `String` | Human-readable description |
| `risk_level` | `RiskLevel` | Risk classification |
| `context` | `Option<Value>` | Additional context data |

## RiskLevel

| Variant | Description |
|---------|-------------|
| `Low` | Low-risk action (e.g., read-only) |
| `Medium` | Medium-risk action |
| `High` | High-risk action (e.g., destructive) |
| `Critical` | Critical action requiring explicit approval |

## ApprovalResult

```rust
pub struct ApprovalResult {
    pub approved: bool,
    pub reason: Option<String>,
    pub approved_by: Option<String>,
}
```

## ApprovalHandler Trait

```rust
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    async fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult;
}
```

## Built-in Handlers

### AutoApprovalHandler

Approves all requests automatically:

```rust
use liteforge::hitl::AutoApprovalHandler;

let handler = AutoApprovalHandler::new();
```

### DenyAllHandler

Denies all requests:

```rust
use liteforge::hitl::DenyAllHandler;

let handler = DenyAllHandler::new();
```

### QueueApprovalHandler

Queues requests for asynchronous review:

```rust
use liteforge::hitl::QueueApprovalHandler;

let handler = QueueApprovalHandler::new();
// Requests are queued and can be reviewed later
let pending = handler.pending_requests();
handler.approve(request_id, "Looks safe");
handler.deny(request_id, "Too risky");
```

### RiskBasedHandler

Automatically approves requests below a risk threshold:

```rust
use liteforge::hitl::RiskBasedHandler;

let handler = RiskBasedHandler::new(RiskLevel::Medium);
// Low/Medium risk: auto-approved
// High/Critical risk: denied (or delegated to fallback)
```

### TimeoutApprovalHandler

Auto-approves if no response within a timeout:

```rust
use liteforge::hitl::TimeoutApprovalHandler;
use std::time::Duration;

let handler = TimeoutApprovalHandler::new(Duration::from_secs(30));
```

## Integration with Agents

See the [Agents Guide](../guides/agents.md#human-in-the-loop) for using HITL with the agent framework.

## JavaScript / TypeScript

```javascript
import { createApprovalRequest, RiskLevel } from '@seanpoyner/liteforge';

const request = createApprovalRequest('delete_file', 'Delete user data', RiskLevel.High);
```
