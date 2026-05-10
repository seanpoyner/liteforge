# Skills

Composable, prompt-based skills for common LLM tasks.

## Skill Trait

```rust
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, ForgeError>;
}
```

## SkillConfig

```rust
pub struct SkillConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}
```

## PromptSkill

A skill backed by a system prompt template:

```rust
use liteforge::skills::{PromptSkill, SkillConfig};

let skill = PromptSkill::new(SkillConfig {
    name: "summarizer".to_string(),
    description: "Summarizes text".to_string(),
    system_prompt: "Summarize the following text concisely.".to_string(),
    temperature: Some(0.3),
    max_tokens: Some(500),
});
```

## Built-in Skills

| Function | Description |
|----------|-------------|
| `summarize_skill()` | Summarizes text |
| `translate_skill()` | Translates text to a target language |
| `extract_skill()` | Extracts structured information |
| `rewrite_skill()` | Rewrites text in a different style |
| `qa_skill()` | Answers questions based on context |

## SkillRegistry

Manage and retrieve skills by name:

```rust
use liteforge::skills::SkillRegistry;

let mut registry = SkillRegistry::new();
registry.register(Box::new(skill));

let skill = registry.get("summarizer").unwrap();
```

## SkillLoader

Load skills from external sources:

```rust
use liteforge::skills::{SkillLoader, SkillSource};

let loader = SkillLoader::new();
let skill = loader.load(SkillSource::Inline {
    name: "custom".to_string(),
    config: skill_config,
});
```

## SkillComposer

Chain skills together:

```rust
use liteforge::skills::{SkillComposer, CompositionStrategy};

let composed = SkillComposer::new()
    .add(summarize_skill())
    .add(translate_skill())
    .strategy(CompositionStrategy::Sequential)
    .build();
```

### CompositionStrategy

| Variant | Behavior |
|---------|----------|
| `Sequential` | Run skills in order, passing output to next input |
| `Parallel` | Run skills concurrently, merge outputs |

## JavaScript / TypeScript

```javascript
import {
  getSummarizeSkill, getTranslateSkill,
  getExtractSkill, getRewriteSkill, getQaSkill,
} from '@forge/sdk';

const summarize = getSummarizeSkill();
const translate = getTranslateSkill();
```
