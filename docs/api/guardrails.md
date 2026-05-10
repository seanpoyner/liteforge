# Guardrails API

PII detection/redaction and prompt injection detection.

## check_all

```rust
pub fn check_all(text: &str) -> GuardrailResult
```

Runs both PII and injection checks. Returns on first failure.

## GuardrailResult

```rust
pub struct GuardrailResult {
    pub passed: bool,
    pub value: Option<String>,
    pub message: String,
    pub guardrail_name: Option<String>,
}
```

| Method | Description |
|--------|-------------|
| `pass(msg)` | Create passing result |
| `fail(msg)` | Create failing result |
| `with_name(name)` | Attach guardrail name |

## PII Module

```rust
use liteforge::guardrails::pii::*;
```

| Function | Returns | Description |
|----------|---------|-------------|
| `detect_pii(text)` | `GuardrailResult` | Check for any PII |
| `find_pii(text)` | `Vec<(PiiType, String)>` | List all PII matches |
| `redact_pii(text)` | `String` | Replace PII with `[REDACTED]` |

### PiiType

`Ssn` | `Phone` | `Email` | `CreditCard` | `IpAddress`

Each has a `.name()` method returning a display string.

## Injection Module

```rust
use liteforge::guardrails::injection::*;
```

| Function | Returns | Description |
|----------|---------|-------------|
| `detect_injection(text)` | `GuardrailResult` | Check for injection patterns |

Detects 8 pattern categories: instruction override, role manipulation, system prompt extraction, jailbreak, roleplay, context manipulation, prompt leaking, encoding bypass.
