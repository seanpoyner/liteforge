//! In-memory queue trigger.

use super::{Trigger, TriggerError, TriggerEvent, TriggerStatus};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// An in-memory queue trigger that fires when messages are pushed.
#[derive(Debug, Clone)]
pub struct QueueTrigger {
    id: String,
    queue: Arc<Mutex<VecDeque<serde_json::Value>>>,
    status: Arc<Mutex<TriggerStatus>>,
    max_size: Option<usize>,
}

impl QueueTrigger {
    /// Create a new queue trigger.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            max_size: None,
        }
    }

    /// Set maximum queue size.
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Push a value onto the queue.
    pub fn push<T: Serialize>(&self, value: T) -> Result<(), TriggerError> {
        let json = serde_json::to_value(value)
            .map_err(|e| TriggerError::runtime(format!("Failed to serialize: {}", e)))?;

        let mut queue = self.queue.lock().unwrap();

        if let Some(max) = self.max_size {
            if queue.len() >= max {
                return Err(TriggerError::runtime("Queue is full"));
            }
        }

        queue.push_back(json);
        Ok(())
    }

    /// Push a raw JSON value onto the queue.
    pub fn push_json(&self, value: serde_json::Value) -> Result<(), TriggerError> {
        let mut queue = self.queue.lock().unwrap();

        if let Some(max) = self.max_size {
            if queue.len() >= max {
                return Err(TriggerError::runtime("Queue is full"));
            }
        }

        queue.push_back(value);
        Ok(())
    }

    /// Get the current queue length.
    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    /// Clear the queue.
    pub fn clear(&self) {
        self.queue.lock().unwrap().clear();
    }
}

impl Trigger for QueueTrigger {
    fn id(&self) -> &str {
        &self.id
    }

    fn trigger_type(&self) -> &str {
        "queue"
    }

    fn status(&self) -> TriggerStatus {
        *self.status.lock().unwrap()
    }

    fn start(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status == TriggerStatus::Running {
            return Err(TriggerError::already_running());
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        *status = TriggerStatus::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Running {
            return Err(TriggerError::not_running());
        }
        *status = TriggerStatus::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Paused {
            return Err(TriggerError::runtime("Trigger is not paused"));
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn poll(&self) -> Option<TriggerEvent> {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return None;
        }

        let mut queue = self.queue.lock().unwrap();
        queue
            .pop_front()
            .map(|payload| TriggerEvent::new(&self.id, "queue", payload))
    }

    fn has_pending(&self) -> bool {
        !self.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_trigger_new() {
        let trigger = QueueTrigger::new("test-queue");
        assert_eq!(trigger.id(), "test-queue");
        assert_eq!(trigger.trigger_type(), "queue");
        assert_eq!(trigger.status(), TriggerStatus::Stopped);
    }

    #[test]
    fn test_queue_trigger_push_poll() {
        let mut trigger = QueueTrigger::new("test");
        trigger.start().unwrap();

        trigger.push("message 1").unwrap();
        trigger.push("message 2").unwrap();

        assert_eq!(trigger.len(), 2);

        let event = trigger.poll().unwrap();
        assert_eq!(event.payload, serde_json::json!("message 1"));

        let event = trigger.poll().unwrap();
        assert_eq!(event.payload, serde_json::json!("message 2"));

        assert!(trigger.poll().is_none());
    }

    #[test]
    fn test_queue_trigger_max_size() {
        let trigger = QueueTrigger::new("test").with_max_size(2);

        trigger.push("1").unwrap();
        trigger.push("2").unwrap();

        let result = trigger.push("3");
        assert!(result.is_err());
    }

    #[test]
    fn test_queue_trigger_not_running() {
        let trigger = QueueTrigger::new("test");
        trigger.push("message").unwrap();

        // Should return None because trigger is not running
        assert!(trigger.poll().is_none());
    }

    #[test]
    fn test_queue_trigger_pause_resume() {
        let mut trigger = QueueTrigger::new("test");
        trigger.start().unwrap();
        trigger.push("message").unwrap();

        trigger.pause().unwrap();
        assert!(trigger.poll().is_none()); // Paused, no events

        trigger.resume().unwrap();
        assert!(trigger.poll().is_some()); // Resumed, events flow
    }

    #[test]
    fn test_queue_trigger_clear() {
        let trigger = QueueTrigger::new("test");
        trigger.push("1").unwrap();
        trigger.push("2").unwrap();

        assert_eq!(trigger.len(), 2);
        trigger.clear();
        assert!(trigger.is_empty());
    }
}
