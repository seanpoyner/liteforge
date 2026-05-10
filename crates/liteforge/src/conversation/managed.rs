//! Managed conversation with message tracking.

use crate::types::Message;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A message with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedMessage {
    /// The message content.
    pub message: Message,
    /// Timestamp when added.
    pub timestamp: u64,
    /// Estimated token count.
    pub token_estimate: usize,
    /// Message ID (sequential).
    pub id: usize,
}

impl TrackedMessage {
    /// Create a new tracked message.
    pub fn new(message: Message, id: usize) -> Self {
        let token_estimate = estimate_tokens(&message);
        Self {
            message,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            token_estimate,
            id,
        }
    }
}

/// Estimate token count for a message (rough approximation: ~4 chars per token).
fn estimate_tokens(message: &Message) -> usize {
    let content_len = message.content.as_ref().map(|c| c.len()).unwrap_or(0);
    let role_len = message.role.len();
    (content_len + role_len) / 4 + 1
}

/// A managed conversation that tracks messages and their metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedConversation {
    /// All tracked messages.
    messages: Vec<TrackedMessage>,
    /// Next message ID.
    next_id: usize,
    /// System message (if any).
    system_message: Option<TrackedMessage>,
    /// Conversation metadata.
    metadata: std::collections::HashMap<String, String>,
}

impl Default for ManagedConversation {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedConversation {
    /// Create a new managed conversation.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            next_id: 0,
            system_message: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a conversation with a system message.
    pub fn with_system(system: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.set_system(system);
        conv
    }

    /// Set or replace the system message.
    pub fn set_system(&mut self, content: impl Into<String>) {
        let message = Message::system(content);
        let id = self.next_id;
        self.next_id += 1;
        self.system_message = Some(TrackedMessage::new(message, id));
    }

    /// Get the system message.
    pub fn system_message(&self) -> Option<&Message> {
        self.system_message.as_ref().map(|t| &t.message)
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::user(content));
    }

    /// Add an assistant message.
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::assistant(content));
    }

    /// Add any message.
    pub fn add_message(&mut self, message: Message) {
        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(TrackedMessage::new(message, id));
    }

    /// Add multiple messages.
    pub fn add_messages(&mut self, messages: impl IntoIterator<Item = Message>) {
        for message in messages {
            self.add_message(message);
        }
    }

    /// Get all messages as a vector (including system message).
    pub fn messages(&self) -> Vec<Message> {
        let mut result = Vec::with_capacity(self.messages.len() + 1);
        if let Some(sys) = &self.system_message {
            result.push(sys.message.clone());
        }
        for tracked in &self.messages {
            result.push(tracked.message.clone());
        }
        result
    }

    /// Get tracked messages (excluding system).
    pub fn tracked_messages(&self) -> &[TrackedMessage] {
        &self.messages
    }

    /// Get the last N messages (excluding system).
    pub fn last_n_messages(&self, n: usize) -> Vec<Message> {
        let start = self.messages.len().saturating_sub(n);
        self.messages[start..]
            .iter()
            .map(|t| t.message.clone())
            .collect()
    }

    /// Get the last message.
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last().map(|t| &t.message)
    }

    /// Get message count (excluding system).
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if empty (excluding system).
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Estimate total token count.
    pub fn estimated_tokens(&self) -> usize {
        let system_tokens = self
            .system_message
            .as_ref()
            .map(|t| t.token_estimate)
            .unwrap_or(0);
        let message_tokens: usize = self.messages.iter().map(|t| t.token_estimate).sum();
        system_tokens + message_tokens
    }

    /// Clear all messages (optionally keeping system message).
    pub fn clear(&mut self, keep_system: bool) {
        self.messages.clear();
        if !keep_system {
            self.system_message = None;
        }
    }

    /// Remove messages older than a given ID.
    pub fn remove_before(&mut self, id: usize) {
        self.messages.retain(|t| t.id >= id);
    }

    /// Remove the first N messages.
    pub fn remove_first(&mut self, n: usize) {
        if n >= self.messages.len() {
            self.messages.clear();
        } else {
            self.messages.drain(0..n);
        }
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Get all metadata.
    pub fn metadata(&self) -> &std::collections::HashMap<String, String> {
        &self.metadata
    }

    /// Find messages containing text.
    pub fn find_messages(&self, text: &str) -> Vec<&TrackedMessage> {
        let text_lower = text.to_lowercase();
        self.messages
            .iter()
            .filter(|t| {
                t.message
                    .content
                    .as_ref()
                    .map(|c| c.to_lowercase().contains(&text_lower))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get messages by role.
    pub fn messages_by_role(&self, role: &str) -> Vec<&TrackedMessage> {
        self.messages
            .iter()
            .filter(|t| t.message.role == role)
            .collect()
    }

    /// Get conversation as formatted text.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        if let Some(sys) = &self.system_message {
            if let Some(content) = &sys.message.content {
                text.push_str(&format!("System: {}\n\n", content));
            }
        }
        for tracked in &self.messages {
            let role = match tracked.message.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                other => other,
            };
            if let Some(content) = &tracked.message.content {
                text.push_str(&format!("{}: {}\n\n", role, content));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_managed_conversation_new() {
        let conv = ManagedConversation::new();
        assert!(conv.is_empty());
        assert_eq!(conv.len(), 0);
        assert!(conv.system_message().is_none());
    }

    #[test]
    fn test_managed_conversation_with_system() {
        let conv = ManagedConversation::with_system("You are a helpful assistant.");
        assert!(conv.system_message().is_some());
        assert_eq!(
            conv.system_message().unwrap().content.as_deref(),
            Some("You are a helpful assistant.")
        );
    }

    #[test]
    fn test_add_messages() {
        let mut conv = ManagedConversation::new();
        conv.add_user_message("Hello!");
        conv.add_assistant_message("Hi there!");

        assert_eq!(conv.len(), 2);
        assert_eq!(conv.messages().len(), 2);
    }

    #[test]
    fn test_messages_with_system() {
        let mut conv = ManagedConversation::with_system("System prompt");
        conv.add_user_message("Hello!");

        let messages = conv.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_last_n_messages() {
        let mut conv = ManagedConversation::new();
        for i in 1..=10 {
            conv.add_user_message(format!("Message {}", i));
        }

        let last_3 = conv.last_n_messages(3);
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].content.as_deref(), Some("Message 8"));
        assert_eq!(last_3[2].content.as_deref(), Some("Message 10"));
    }

    #[test]
    fn test_estimated_tokens() {
        let mut conv = ManagedConversation::new();
        conv.add_user_message("Hello, how are you?");
        conv.add_assistant_message("I'm doing well, thank you!");

        let tokens = conv.estimated_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_clear() {
        let mut conv = ManagedConversation::with_system("System");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi");

        conv.clear(true);
        assert!(conv.is_empty());
        assert!(conv.system_message().is_some());

        conv.clear(false);
        assert!(conv.system_message().is_none());
    }

    #[test]
    fn test_remove_first() {
        let mut conv = ManagedConversation::new();
        conv.add_user_message("1");
        conv.add_user_message("2");
        conv.add_user_message("3");
        conv.add_user_message("4");

        conv.remove_first(2);
        assert_eq!(conv.len(), 2);
        assert_eq!(conv.messages()[0].content.as_deref(), Some("3"));
    }

    #[test]
    fn test_find_messages() {
        let mut conv = ManagedConversation::new();
        conv.add_user_message("Hello there!");
        conv.add_assistant_message("Hello to you too!");
        conv.add_user_message("Goodbye");

        let found = conv.find_messages("hello");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_messages_by_role() {
        let mut conv = ManagedConversation::new();
        conv.add_user_message("U1");
        conv.add_assistant_message("A1");
        conv.add_user_message("U2");

        let user_messages = conv.messages_by_role("user");
        assert_eq!(user_messages.len(), 2);
    }

    #[test]
    fn test_metadata() {
        let mut conv = ManagedConversation::new();
        conv.set_metadata("session_id", "abc123");
        conv.set_metadata("user_id", "user456");

        assert_eq!(conv.get_metadata("session_id"), Some("abc123"));
        assert_eq!(conv.get_metadata("user_id"), Some("user456"));
        assert_eq!(conv.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_to_text() {
        let mut conv = ManagedConversation::with_system("You are helpful.");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi!");

        let text = conv.to_text();
        assert!(text.contains("System: You are helpful."));
        assert!(text.contains("User: Hello"));
        assert!(text.contains("Assistant: Hi!"));
    }
}
