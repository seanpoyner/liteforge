# Evals API

Evaluation framework for testing LLM outputs.

## TestCase

```rust
use liteforge::evals::TestCase;

let case = TestCase::builder()
    .input("What is 2+2?")
    .expected("4")
    .build();
```

## Evaluator Trait

```rust
pub trait Evaluator: Send + Sync {
    fn evaluate(&self, output: &str, expected: &str) -> EvalResult;
}
```

## Built-in Evaluators

| Evaluator | Match Logic |
|-----------|-------------|
| `ExactMatchEvaluator` | Output == expected (exact string) |
| `ContainsEvaluator` | Output contains expected |
| `RegexEvaluator` | Output matches regex pattern |
| `JsonMatchEvaluator` | JSON structural equality |
| `SimilarityEvaluator` | String similarity score above threshold |

## EvalResult

```rust
pub struct EvalResult {
    pub passed: bool,
}
```

## EvalSuite

Run multiple test cases:

```rust
use liteforge::evals::{EvalSuite, ExactMatchEvaluator};

let mut suite = EvalSuite::new(Box::new(ExactMatchEvaluator));

suite.add_case(TestCase::builder()
    .input("capital of France")
    .expected("Paris")
    .build());

let result: SuiteResult = suite.run(&outputs);
println!("{}/{} passed", result.stats.passed, result.stats.total);
```

### SuiteStats

| Field | Type |
|-------|------|
| `total` | `usize` |
| `passed` | `usize` |
| `failed` | `usize` |
