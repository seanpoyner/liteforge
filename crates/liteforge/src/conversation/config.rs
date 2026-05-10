//! Conversation configuration.

use serde::{Deserialize, Serialize};

/// Strategy for summarizing conversations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SummarizationStrategy {
    /// Keep the most recent N messages, discard the rest.
    KeepRecent,
    /// Summarize older messages into a single system message.
    #[default]
    Summarize,
    /// Summarize in chunks, keeping multiple summary levels.
    ChunkedSummary,
    /// Keep system messages and summarize user/assistant turns.
    PreserveSystem,
}

/// Configuration for conversation management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    /// Maximum number of tokens before compaction.
    pub max_tokens: usize,
    /// Target number of tokens after compaction.
    pub target_tokens: usize,
    /// Number of recent messages to always preserve.
    pub preserve_recent: usize,
    /// Whether to preserve system messages during compaction.
    pub preserve_system: bool,
    /// Strategy for summarization.
    pub strategy: SummarizationStrategy,
    /// Model to use for summarization (if strategy requires it).
    pub summarization_model: Option<String>,
    /// System prompt for summarization.
    pub summarization_prompt: String,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8000,
            target_tokens: 4000,
            preserve_recent: 4,
            preserve_system: true,
            strategy: SummarizationStrategy::Summarize,
            summarization_model: None,
            summarization_prompt: "Summarize the following conversation, preserving key information, decisions, and context. Be concise but complete:".to_string(),
        }
    }
}

impl ConversationConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> ConversationConfigBuilder {
        ConversationConfigBuilder::default()
    }
}

/// Builder for ConversationConfig.
#[derive(Debug, Default)]
pub struct ConversationConfigBuilder {
    max_tokens: Option<usize>,
    target_tokens: Option<usize>,
    preserve_recent: Option<usize>,
    preserve_system: Option<bool>,
    strategy: Option<SummarizationStrategy>,
    summarization_model: Option<String>,
    summarization_prompt: Option<String>,
}

impl ConversationConfigBuilder {
    /// Set maximum tokens before compaction.
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set target tokens after compaction.
    pub fn target_tokens(mut self, tokens: usize) -> Self {
        self.target_tokens = Some(tokens);
        self
    }

    /// Set number of recent messages to preserve.
    pub fn preserve_recent(mut self, count: usize) -> Self {
        self.preserve_recent = Some(count);
        self
    }

    /// Set whether to preserve system messages.
    pub fn preserve_system(mut self, preserve: bool) -> Self {
        self.preserve_system = Some(preserve);
        self
    }

    /// Set summarization strategy.
    pub fn strategy(mut self, strategy: SummarizationStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Set model for summarization.
    pub fn summarization_model(mut self, model: impl Into<String>) -> Self {
        self.summarization_model = Some(model.into());
        self
    }

    /// Set prompt for summarization.
    pub fn summarization_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.summarization_prompt = Some(prompt.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> ConversationConfig {
        let default = ConversationConfig::default();
        ConversationConfig {
            max_tokens: self.max_tokens.unwrap_or(default.max_tokens),
            target_tokens: self.target_tokens.unwrap_or(default.target_tokens),
            preserve_recent: self.preserve_recent.unwrap_or(default.preserve_recent),
            preserve_system: self.preserve_system.unwrap_or(default.preserve_system),
            strategy: self.strategy.unwrap_or(default.strategy),
            summarization_model: self.summarization_model.or(default.summarization_model),
            summarization_prompt: self
                .summarization_prompt
                .unwrap_or(default.summarization_prompt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ConversationConfig::default();
        assert_eq!(config.max_tokens, 8000);
        assert_eq!(config.target_tokens, 4000);
        assert_eq!(config.preserve_recent, 4);
        assert!(config.preserve_system);
        assert_eq!(config.strategy, SummarizationStrategy::Summarize);
    }

    #[test]
    fn test_config_builder() {
        let config = ConversationConfig::builder()
            .max_tokens(16000)
            .target_tokens(8000)
            .preserve_recent(6)
            .strategy(SummarizationStrategy::KeepRecent)
            .build();

        assert_eq!(config.max_tokens, 16000);
        assert_eq!(config.target_tokens, 8000);
        assert_eq!(config.preserve_recent, 6);
        assert_eq!(config.strategy, SummarizationStrategy::KeepRecent);
    }
}
