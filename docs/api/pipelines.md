# Pipelines

Multi-step processing pipelines for composing LLM operations.

## Pipeline

```rust
use liteforge::pipelines::{Pipeline, PipelineBuilder, PipelineContext};

let pipeline = PipelineBuilder::new()
    .add_step(Box::new(my_step))
    .add_step(Box::new(another_step))
    .build();

let mut context = PipelineContext::new();
let output = pipeline.execute(&mut context).await?;
```

## PipelineStep Trait

```rust
pub trait PipelineStep: Send + Sync {
    fn name(&self) -> &str;
    fn execute(
        &self,
        context: &mut PipelineContext,
    ) -> Result<StepOutput, PipelineError>;
}
```

## Built-in Steps

### LlmStep

Calls an LLM for a completion:

```rust
use liteforge::pipelines::LlmStep;

let step = LlmStep::new(client, "Summarize the input");
```

### TransformStep

Applies a transformation function:

```rust
use liteforge::pipelines::TransformStep;

let step = TransformStep::new("uppercase", |input| input.to_uppercase());
```

### BranchStep

Conditional routing:

```rust
use liteforge::pipelines::BranchStep;

let step = BranchStep::new(
    |ctx| ctx.get("sentiment") == Some("negative"),
    Box::new(escalation_step),
    Box::new(standard_step),
);
```

## PipelineContext

Shared context passed through all pipeline steps:

```rust
pub struct PipelineContext {
    // Key-value store for passing data between steps
}
```

| Method | Description |
|--------|-------------|
| `new()` | Create empty context |
| `set(key, value)` | Store a value |
| `get(key)` | Retrieve a value |
| `input()` | Get the pipeline input |

## PipelineOutput / StepOutput

```rust
pub struct PipelineOutput {
    pub steps: Vec<StepOutput>,
    pub final_output: String,
}

pub struct StepOutput {
    pub step_name: String,
    pub output: String,
}
```

## ModelTransform & TransformChain

Chain transformations:

```rust
use liteforge::pipelines::{ModelTransform, TransformChain};

let chain = TransformChain::new()
    .add(ModelTransform::new("cleanup", cleanup_fn))
    .add(ModelTransform::new("format", format_fn));
```

## PipelineError

| Variant | Description |
|---------|-------------|
| `StepFailed(String)` | A pipeline step failed |
| `ContextMissing(String)` | Required context key not found |

## JavaScript / TypeScript

```javascript
import { PipelineContext } from '@forge/sdk';

const context = new PipelineContext();
```
