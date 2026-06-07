# Prompts

Template-based prompt construction with variable substitution.

## PromptTemplate

```rust
use liteforge::prompts::PromptTemplate;

let template = PromptTemplate::new("Summarize the following {{language}} text: {{text}}");
let rendered = template.render(&[
    ("language", "English"),
    ("text", "Your document here..."),
])?;
```

### Methods

| Method | Description |
|--------|-------------|
| `new(template)` | Create from template string with `{{var}}` placeholders |
| `render(vars)` | Render with variable substitutions |
| `variables()` | List required variable names |

### TemplateError

| Variant | Description |
|---------|-------------|
| `MissingVariable(String)` | Required variable not provided |
| `InvalidTemplate(String)` | Template syntax error |

## PromptLibrary

Store and retrieve named templates:

```rust
use liteforge::prompts::PromptLibrary;

let mut library = PromptLibrary::new();
library.add("summarize", PromptTemplate::new("Summarize: {{text}}"));
library.add("translate", PromptTemplate::new("Translate to {{lang}}: {{text}}"));

let template = library.get("summarize").unwrap();
```

## PromptConfig

```rust
pub struct PromptConfig {
    pub name: String,
    pub template: String,
    pub description: Option<String>,
}
```

## PromptBuilder

Build prompts fluently:

```rust
use liteforge::prompts::PromptBuilder;

let prompt = PromptBuilder::new()
    .system("You are a helpful assistant.")
    .user("{{question}}")
    .build();
```

## CommonPrompts

Pre-built prompt templates:

| Method | Description |
|--------|-------------|
| `summarize()` | Text summarization |
| `translate()` | Language translation |
| `qa()` | Question answering |
| `code_review()` | Code review feedback |
| `explain()` | Explanation generation |

```rust
use liteforge::prompts::CommonPrompts;

let qa_template = CommonPrompts::qa();
```

## JavaScript / TypeScript

```javascript
import { PromptTemplate, PromptLibrary, CommonPrompts } from '@seanpoyner/liteforge';

const template = new PromptTemplate('Summarize: {{text}}');
const rendered = template.render({ text: 'Your document...' });

const library = new PromptLibrary();
library.add('summarize', template);

const qa = CommonPrompts.qa();
const review = CommonPrompts.codeReview();
```
