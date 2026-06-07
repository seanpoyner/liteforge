# Conversation

Multi-turn conversation management with automatic compaction.

## ManagedConversation

Track conversation history with token-aware management:

```rust
use liteforge::conversation::ManagedConversation;
use liteforge::Message;

let mut conv = ManagedConversation::new();

conv.add_message(Message::user("Hello!"));
conv.add_message(Message::assistant("Hi! How can I help?"));
conv.add_message(Message::user("Tell me about Rust."));

let messages = conv.messages();
let count = conv.len();
```

### Methods

| Method | Description |
|--------|-------------|
| `new()` | Create empty conversation |
| `add_message(msg)` | Add a message |
| `messages()` | Get all messages |
| `len()` | Message count |
| `clear()` | Clear all messages |

## CompactingConversation

Automatically compacts conversation history when it exceeds token limits by summarizing older messages:

```rust
use liteforge::conversation::{CompactingConversation, ConversationConfig, SummarizationStrategy};

let config = ConversationConfig {
    max_tokens: Some(4096),
    target_tokens: Some(2048),
    preserve_recent: Some(5),
    strategy: SummarizationStrategy::Summarize,
};

let mut conv = CompactingConversation::new(config);
conv.add_message(Message::user("Hello!"));
// When max_tokens is exceeded, older messages are summarized
```

## ConversationConfig

```rust
pub struct ConversationConfig {
    pub max_tokens: Option<u32>,
    pub target_tokens: Option<u32>,
    pub preserve_recent: Option<usize>,
    pub strategy: SummarizationStrategy,
}
```

| Field | Description | Default |
|-------|-------------|---------|
| `max_tokens` | Maximum token count before compaction triggers | `None` (no limit) |
| `target_tokens` | Target token count after compaction | `None` |
| `preserve_recent` | Number of recent messages to always keep | `None` |
| `strategy` | How to compact older messages | `Summarize` |

## SummarizationStrategy

| Variant | Description |
|---------|-------------|
| `Summarize` | Summarize older messages into a single system message |
| `Truncate` | Drop oldest messages |
| `SlidingWindow` | Keep a sliding window of recent messages |

## JavaScript / TypeScript

```javascript
import { ManagedConversation, CompactingConversation, ConversationConfig } from '@seanpoyner/liteforge';

// Simple conversation tracking
const conv = new ManagedConversation();
conv.addMessage('user', 'Hello!');
conv.addMessage('assistant', 'Hi there!');
console.log(conv.messages());

// Auto-compacting conversation
const config = new ConversationConfig(4096, 2048, 5, 'summarize');
const compacting = new CompactingConversation(config);
```
