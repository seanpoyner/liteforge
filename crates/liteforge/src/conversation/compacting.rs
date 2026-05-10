//! Compacting conversation that auto-summarizes when approaching limits.

use super::config::{ConversationConfig, SummarizationStrategy};
use super::managed::{ManagedConversation, TrackedMessage};
use crate::types::Message;
use serde::{Deserialize, Serialize};

/// Result of a compaction operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    /// Number of messages removed.
    pub messages_removed: usize,
    /// Tokens before compaction.
    pub tokens_before: usize,
    /// Tokens after compaction.
    pub tokens_after: usize,
    /// Summary generated (if any).
    pub summary: Option<String>,
}

/// A conversation that automatically compacts when approaching token limits.
///
/// This wraps a ManagedConversation and adds automatic summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactingConversation {
    /// The underlying conversation.
    conversation: ManagedConversation,
    /// Configuration.
    config: ConversationConfig,
    /// Summaries of compacted messages.
    summaries: Vec<String>,
    /// Total messages ever added (before compaction).
    total_messages_added: usize,
    /// Number of compactions performed.
    compaction_count: usize,
}

impl Default for CompactingConversation {
    fn default() -> Self {
        Self::new(ConversationConfig::default())
    }
}

impl CompactingConversation {
    /// Create a new compacting conversation with config.
    pub fn new(config: ConversationConfig) -> Self {
        Self {
            conversation: ManagedConversation::new(),
            config,
            summaries: Vec::new(),
            total_messages_added: 0,
            compaction_count: 0,
        }
    }

    /// Create with default config.
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Create with a system message.
    pub fn with_system(system: impl Into<String>, config: ConversationConfig) -> Self {
        let mut conv = Self::new(config);
        conv.conversation.set_system(system);
        conv
    }

    /// Set or replace the system message.
    pub fn set_system(&mut self, content: impl Into<String>) {
        self.conversation.set_system(content);
    }

    /// Get the system message.
    pub fn system_message(&self) -> Option<&Message> {
        self.conversation.system_message()
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.conversation.add_user_message(content);
        self.total_messages_added += 1;
    }

    /// Add an assistant message.
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.conversation.add_assistant_message(content);
        self.total_messages_added += 1;
    }

    /// Add any message.
    pub fn add_message(&mut self, message: Message) {
        self.conversation.add_message(message);
        self.total_messages_added += 1;
    }

    /// Check if compaction is needed.
    pub fn needs_compaction(&self) -> bool {
        self.conversation.estimated_tokens() > self.config.max_tokens
    }

    /// Compact the conversation using the configured strategy.
    /// Returns None if no compaction was needed.
    pub fn compact(&mut self) -> Option<CompactionResult> {
        if !self.needs_compaction() {
            return None;
        }

        let tokens_before = self.conversation.estimated_tokens();
        let messages_before = self.conversation.len();

        let summary = match self.config.strategy {
            SummarizationStrategy::KeepRecent => self.compact_keep_recent(),
            SummarizationStrategy::Summarize => self.compact_with_summary(),
            SummarizationStrategy::ChunkedSummary => self.compact_chunked(),
            SummarizationStrategy::PreserveSystem => self.compact_preserve_system(),
        };

        let tokens_after = self.conversation.estimated_tokens();
        let messages_after = self.conversation.len();

        self.compaction_count += 1;

        Some(CompactionResult {
            messages_removed: messages_before - messages_after,
            tokens_before,
            tokens_after,
            summary,
        })
    }

    /// Compact by keeping only recent messages.
    fn compact_keep_recent(&mut self) -> Option<String> {
        let preserve = self.config.preserve_recent;
        if self.conversation.len() > preserve {
            let to_remove = self.conversation.len() - preserve;
            self.conversation.remove_first(to_remove);
        }
        None
    }

    /// Compact by creating a summary of older messages.
    fn compact_with_summary(&mut self) -> Option<String> {
        let preserve = self.config.preserve_recent;
        if self.conversation.len() <= preserve {
            return None;
        }

        // Get messages to summarize
        let to_remove = self.conversation.len() - preserve;
        let tracked = self.conversation.tracked_messages();
        let messages_to_summarize: Vec<&TrackedMessage> = tracked.iter().take(to_remove).collect();

        // Create a simple text summary (in production, this would call an LLM)
        let summary = self.create_summary_text(&messages_to_summarize);
        self.summaries.push(summary.clone());

        // Remove old messages
        self.conversation.remove_first(to_remove);

        Some(summary)
    }

    /// Compact in chunks, creating multiple summary levels.
    fn compact_chunked(&mut self) -> Option<String> {
        // For now, same as regular summarize but could be extended
        self.compact_with_summary()
    }

    /// Compact while preserving system messages.
    fn compact_preserve_system(&mut self) -> Option<String> {
        // Same as summarize but system is already preserved by default
        self.compact_with_summary()
    }

    /// Create a text representation of messages for summarization.
    fn create_summary_text(&self, messages: &[&TrackedMessage]) -> String {
        let mut text = String::from("[Previous conversation summary]\n");
        for tracked in messages {
            let role = match tracked.message.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                other => other,
            };
            if let Some(content) = &tracked.message.content {
                // Truncate very long messages
                let content = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.clone()
                };
                text.push_str(&format!("- {}: {}\n", role, content));
            }
        }
        text
    }

    /// Get all messages (including system).
    pub fn messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add summary as a system message if there are summaries
        if !self.summaries.is_empty() {
            let combined_summary = self.summaries.join("\n\n");
            messages.push(Message::system(format!(
                "Previous conversation context:\n{}",
                combined_summary
            )));
        }

        // Add current messages
        messages.extend(self.conversation.messages());
        messages
    }

    /// Get messages for API call (optimized format).
    pub fn messages_for_api(&self) -> Vec<Message> {
        self.messages()
    }

    /// Get current message count (excluding compacted).
    pub fn len(&self) -> usize {
        self.conversation.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.conversation.is_empty()
    }

    /// Get estimated tokens.
    pub fn estimated_tokens(&self) -> usize {
        self.conversation.estimated_tokens()
    }

    /// Get configuration.
    pub fn config(&self) -> &ConversationConfig {
        &self.config
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: ConversationConfig) {
        self.config = config;
    }

    /// Get total messages ever added.
    pub fn total_messages_added(&self) -> usize {
        self.total_messages_added
    }

    /// Get number of compactions performed.
    pub fn compaction_count(&self) -> usize {
        self.compaction_count
    }

    /// Get all summaries.
    pub fn summaries(&self) -> &[String] {
        &self.summaries
    }

    /// Clear the conversation.
    pub fn clear(&mut self, keep_system: bool) {
        self.conversation.clear(keep_system);
        self.summaries.clear();
    }

    /// Get the underlying managed conversation.
    pub fn inner(&self) -> &ManagedConversation {
        &self.conversation
    }

    /// Get mutable access to underlying conversation.
    pub fn inner_mut(&mut self) -> &mut ManagedConversation {
        &mut self.conversation
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.conversation.set_metadata(key, value);
    }

    /// Get metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.conversation.get_metadata(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> ConversationConfig {
        ConversationConfig {
            max_tokens: 100,
            target_tokens: 50,
            preserve_recent: 2,
            preserve_system: true,
            strategy: SummarizationStrategy::KeepRecent,
            summarization_model: None,
            summarization_prompt: String::new(),
        }
    }

    #[test]
    fn test_compacting_conversation_new() {
        let conv = CompactingConversation::with_defaults();
        assert!(conv.is_empty());
        assert_eq!(conv.compaction_count(), 0);
    }

    #[test]
    fn test_compacting_add_messages() {
        let mut conv = CompactingConversation::with_defaults();
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi!");

        assert_eq!(conv.len(), 2);
        assert_eq!(conv.total_messages_added(), 2);
    }

    #[test]
    fn test_compacting_needs_compaction() {
        let mut conv = CompactingConversation::new(small_config());
        conv.add_user_message("Short");
        assert!(!conv.needs_compaction());

        // Add many messages to exceed limit
        for i in 0..20 {
            conv.add_user_message(format!("Message number {} with some content", i));
        }
        assert!(conv.needs_compaction());
    }

    #[test]
    fn test_compact_keep_recent() {
        let mut conv = CompactingConversation::new(small_config());

        // Add enough messages to trigger compaction
        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        if conv.needs_compaction() {
            let result = conv.compact();
            assert!(result.is_some());

            let result = result.unwrap();
            assert!(result.messages_removed > 0);
            assert_eq!(conv.len(), 2); // preserve_recent = 2
        }
    }

    #[test]
    fn test_compact_with_summary() {
        let config = ConversationConfig {
            max_tokens: 100,
            target_tokens: 50,
            preserve_recent: 2,
            preserve_system: true,
            strategy: SummarizationStrategy::Summarize,
            summarization_model: None,
            summarization_prompt: String::new(),
        };
        let mut conv = CompactingConversation::new(config);

        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        if conv.needs_compaction() {
            let result = conv.compact();
            assert!(result.is_some());

            let result = result.unwrap();
            assert!(result.summary.is_some());
            assert_eq!(conv.summaries().len(), 1);
        }
    }

    #[test]
    fn test_messages_include_summary() {
        let config = ConversationConfig {
            max_tokens: 100,
            target_tokens: 50,
            preserve_recent: 2,
            preserve_system: true,
            strategy: SummarizationStrategy::Summarize,
            summarization_model: None,
            summarization_prompt: String::new(),
        };
        let mut conv = CompactingConversation::new(config);

        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        // Force compaction
        while conv.needs_compaction() {
            conv.compact();
        }

        // Messages should include summary context
        let messages = conv.messages();
        if !conv.summaries().is_empty() {
            assert!(messages.iter().any(|m| m.role == "system"));
        }
    }

    #[test]
    fn test_with_system_message() {
        let config = small_config();
        let mut conv = CompactingConversation::with_system("You are helpful.", config);

        assert!(conv.system_message().is_some());
        conv.add_user_message("Hello");

        let messages = conv.messages();
        assert!(messages.iter().any(|m| m.role == "system"));
    }

    #[test]
    fn test_compaction_count() {
        let mut conv = CompactingConversation::new(small_config());

        for i in 0..10 {
            conv.add_user_message(format!("Message {}", i));
        }

        let mut count = 0;
        while conv.needs_compaction() {
            conv.compact();
            count += 1;
        }

        assert_eq!(conv.compaction_count(), count);
    }

    #[test]
    fn test_clear() {
        let mut conv = CompactingConversation::with_system("System", small_config());
        conv.add_user_message("Hello");

        // Force some summaries
        for i in 0..10 {
            conv.add_user_message(format!("Msg {}", i));
        }
        while conv.needs_compaction() {
            conv.compact();
        }

        conv.clear(true);
        assert!(conv.is_empty());
        assert!(conv.summaries().is_empty());
        assert!(conv.system_message().is_some());
    }

    #[test]
    fn test_metadata() {
        let mut conv = CompactingConversation::with_defaults();
        conv.set_metadata("key", "value");
        assert_eq!(conv.get_metadata("key"), Some("value"));
    }
}
