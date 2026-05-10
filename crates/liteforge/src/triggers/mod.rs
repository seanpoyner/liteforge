//! Triggers for event-driven agent execution.
//!
//! This module provides various trigger types that can start agent execution
//! based on external events like webhooks, file changes, queues, or schedules.
//!
//! # Overview
//!
//! - **WebhookTrigger**: HTTP endpoint that triggers on incoming requests
//! - **FileWatchTrigger**: Monitors filesystem for changes
//! - **QueueTrigger**: Processes messages from an in-memory queue
//! - **ScheduleTrigger**: Fires based on cron/interval schedules
//!
//! # Example
//!
//! ```rust,no_run
//! use liteforge::triggers::{Trigger, TriggerEvent, QueueTrigger};
//!
//! // Create a queue trigger
//! let trigger = QueueTrigger::new("my-queue");
//!
//! // Push an event
//! trigger.push("Hello, world!");
//!
//! // Poll for events
//! if let Some(event) = trigger.poll() {
//!     println!("Received: {:?}", event);
//! }
//! ```

mod filewatch;
mod manager;
mod queue;
mod schedule;
mod webhook;

pub use filewatch::{FileEvent, FileEventKind, FileWatchTrigger};
pub use manager::{TriggerHandle, TriggerManager};
pub use queue::QueueTrigger;
pub use schedule::ScheduleTrigger;
pub use webhook::{WebhookConfig, WebhookEvent, WebhookTrigger};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Unique identifier for a trigger.
pub type TriggerId = String;

/// Event emitted by a trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// Trigger ID that emitted this event.
    pub trigger_id: TriggerId,
    /// Event type (e.g., "webhook", "file", "queue", "schedule").
    pub event_type: String,
    /// Event payload as JSON value.
    pub payload: serde_json::Value,
    /// Timestamp when the event occurred.
    pub timestamp: u64,
    /// Event metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl TriggerEvent {
    /// Create a new trigger event.
    pub fn new(
        trigger_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            trigger_id: trigger_id.into(),
            event_type: event_type.into(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the payload as a specific type.
    pub fn payload_as<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.payload.clone())
    }
}

/// Status of a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TriggerStatus {
    /// Trigger is stopped.
    #[default]
    Stopped,
    /// Trigger is running and listening for events.
    Running,
    /// Trigger is paused.
    Paused,
    /// Trigger encountered an error.
    Error,
}

/// Trait for trigger implementations.
pub trait Trigger: Send + Sync {
    /// Get the trigger ID.
    fn id(&self) -> &str;

    /// Get the trigger type name.
    fn trigger_type(&self) -> &str;

    /// Get the current status.
    fn status(&self) -> TriggerStatus;

    /// Start the trigger.
    fn start(&mut self) -> Result<(), TriggerError>;

    /// Stop the trigger.
    fn stop(&mut self) -> Result<(), TriggerError>;

    /// Pause the trigger.
    fn pause(&mut self) -> Result<(), TriggerError>;

    /// Resume the trigger.
    fn resume(&mut self) -> Result<(), TriggerError>;

    /// Poll for the next event (non-blocking).
    fn poll(&self) -> Option<TriggerEvent>;

    /// Check if there are pending events.
    fn has_pending(&self) -> bool {
        false
    }
}

/// Error type for trigger operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerError {
    /// Error message.
    pub message: String,
    /// Error kind.
    pub kind: TriggerErrorKind,
}

impl TriggerError {
    /// Create a new trigger error.
    pub fn new(kind: TriggerErrorKind, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(TriggerErrorKind::Configuration, message)
    }

    /// Create a runtime error.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(TriggerErrorKind::Runtime, message)
    }

    /// Create an already running error.
    pub fn already_running() -> Self {
        Self::new(
            TriggerErrorKind::AlreadyRunning,
            "Trigger is already running",
        )
    }

    /// Create a not running error.
    pub fn not_running() -> Self {
        Self::new(TriggerErrorKind::NotRunning, "Trigger is not running")
    }
}

impl std::fmt::Display for TriggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for TriggerError {}

/// Kind of trigger error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerErrorKind {
    /// Configuration error.
    Configuration,
    /// Runtime error.
    Runtime,
    /// Trigger is already running.
    AlreadyRunning,
    /// Trigger is not running.
    NotRunning,
    /// IO error.
    Io,
    /// Network error.
    Network,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_event_new() {
        let event = TriggerEvent::new("test-trigger", "test", serde_json::json!({"key": "value"}));

        assert_eq!(event.trigger_id, "test-trigger");
        assert_eq!(event.event_type, "test");
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_trigger_event_with_metadata() {
        let event = TriggerEvent::new("test", "test", serde_json::json!(null))
            .with_metadata("source", "unit-test");

        assert_eq!(event.metadata.get("source"), Some(&"unit-test".to_string()));
    }

    #[test]
    fn test_trigger_event_payload_as() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct TestPayload {
            message: String,
        }

        let event = TriggerEvent::new("test", "test", serde_json::json!({"message": "hello"}));
        let payload: TestPayload = event.payload_as().unwrap();

        assert_eq!(payload.message, "hello");
    }

    #[test]
    fn test_trigger_error() {
        let err = TriggerError::config("Invalid configuration");
        assert_eq!(err.kind, TriggerErrorKind::Configuration);
        assert!(err.to_string().contains("Invalid configuration"));
    }
}
