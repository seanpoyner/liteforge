use crate::types::*;
use liteforge::conversation::{
    CompactingConversation as RustCompactingConversation,
    ConversationConfig as RustConversationConfig, ManagedConversation as RustManagedConversation,
    SummarizationStrategy as RustSummarizationStrategy,
};

#[napi(string_enum)]
pub enum SummarizationStrategy {
    KeepRecent,
    Summarize,
    ChunkedSummary,
    PreserveSystem,
}

fn js_strategy_to_rust(s: &SummarizationStrategy) -> RustSummarizationStrategy {
    match s {
        SummarizationStrategy::KeepRecent => RustSummarizationStrategy::KeepRecent,
        SummarizationStrategy::Summarize => RustSummarizationStrategy::Summarize,
        SummarizationStrategy::ChunkedSummary => RustSummarizationStrategy::ChunkedSummary,
        SummarizationStrategy::PreserveSystem => RustSummarizationStrategy::PreserveSystem,
    }
}

#[napi]
pub struct ConversationConfig {
    inner: RustConversationConfig,
}

#[napi]
impl ConversationConfig {
    #[napi(constructor)]
    pub fn new(
        max_tokens: Option<u32>,
        target_tokens: Option<u32>,
        preserve_recent: Option<u32>,
        strategy: Option<SummarizationStrategy>,
    ) -> Self {
        let mut builder = RustConversationConfig::builder();
        if let Some(mt) = max_tokens {
            builder = builder.max_tokens(mt as usize);
        }
        if let Some(tt) = target_tokens {
            builder = builder.target_tokens(tt as usize);
        }
        if let Some(pr) = preserve_recent {
            builder = builder.preserve_recent(pr as usize);
        }
        if let Some(s) = strategy {
            builder = builder.strategy(js_strategy_to_rust(&s));
        }
        Self {
            inner: builder.build(),
        }
    }
}

#[napi]
pub struct ManagedConversation {
    inner: RustManagedConversation,
}

#[napi]
impl ManagedConversation {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustManagedConversation::new(),
        }
    }

    #[napi]
    pub fn set_system(&mut self, content: String) {
        self.inner.set_system(content);
    }

    #[napi]
    pub fn add_user_message(&mut self, content: String) {
        self.inner.add_user_message(content);
    }

    #[napi]
    pub fn add_assistant_message(&mut self, content: String) {
        self.inner.add_assistant_message(content);
    }

    #[napi]
    pub fn messages(&self) -> Vec<Message> {
        self.inner
            .messages()
            .iter()
            .map(rust_message_to_js)
            .collect()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn estimated_tokens(&self) -> u32 {
        self.inner.estimated_tokens() as u32
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear(false);
    }
}

#[napi]
pub struct CompactingConversation {
    inner: RustCompactingConversation,
}

#[napi]
impl CompactingConversation {
    #[napi(constructor)]
    pub fn new(config: &ConversationConfig) -> Self {
        Self {
            inner: RustCompactingConversation::new(config.inner.clone()),
        }
    }

    #[napi]
    pub fn set_system(&mut self, content: String) {
        self.inner.set_system(content);
    }

    #[napi]
    pub fn add_user_message(&mut self, content: String) {
        self.inner.add_user_message(content);
    }

    #[napi]
    pub fn add_assistant_message(&mut self, content: String) {
        self.inner.add_assistant_message(content);
    }

    #[napi]
    pub fn messages(&self) -> Vec<Message> {
        self.inner
            .messages()
            .iter()
            .map(rust_message_to_js)
            .collect()
    }

    #[napi]
    pub fn needs_compaction(&self) -> bool {
        self.inner.needs_compaction()
    }

    #[napi]
    pub fn compact(&mut self) -> serde_json::Value {
        let result = self.inner.compact();
        serde_json::to_value(result).unwrap_or(serde_json::Value::Null)
    }

    #[napi]
    pub fn estimated_tokens(&self) -> u32 {
        self.inner.estimated_tokens() as u32
    }
}
