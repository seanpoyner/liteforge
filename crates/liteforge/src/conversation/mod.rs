//! Conversation management for context window handling.
//!
//! This module provides utilities for managing long-running conversations,
//! including automatic summarization and token management.
//!
//! # Overview
//!
//! - **ManagedConversation**: Tracks messages and provides utilities
//! - **CompactingConversation**: Auto-summarizes when approaching limits
//! - **ConversationConfig**: Configuration for behavior
//!
//! # Example
//!
//! ```rust
//! use liteforge::conversation::{ManagedConversation, ConversationConfig};
//! use liteforge::Message;
//!
//! let mut conv = ManagedConversation::new();
//!
//! conv.add_user_message("Hello!");
//! conv.add_assistant_message("Hi! How can I help?");
//!
//! let messages = conv.messages();
//! ```

mod compacting;
mod config;
mod managed;

pub use compacting::CompactingConversation;
pub use config::{ConversationConfig, SummarizationStrategy};
pub use managed::ManagedConversation;
