//! Trigger manager for handling multiple triggers.

use super::{Trigger, TriggerError, TriggerEvent, TriggerStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// Type alias for the trigger storage to reduce type complexity.
type TriggerMap = HashMap<String, Arc<Mutex<Box<dyn Trigger>>>>;

/// Handle to a registered trigger.
#[derive(Debug, Clone)]
pub struct TriggerHandle {
    id: String,
    trigger_type: String,
}

impl TriggerHandle {
    /// Get the trigger ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the trigger type.
    pub fn trigger_type(&self) -> &str {
        &self.trigger_type
    }
}

/// Manages multiple triggers and dispatches their events.
pub struct TriggerManager {
    triggers: RwLock<TriggerMap>,
    running: Arc<Mutex<bool>>,
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TriggerManager {
    /// Create a new trigger manager.
    pub fn new() -> Self {
        Self {
            triggers: RwLock::new(HashMap::new()),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a trigger.
    pub fn register<T: Trigger + 'static>(
        &self,
        trigger: T,
    ) -> Result<TriggerHandle, TriggerError> {
        let id = trigger.id().to_string();
        let trigger_type = trigger.trigger_type().to_string();

        let mut triggers = self.triggers.write().unwrap();
        if triggers.contains_key(&id) {
            return Err(TriggerError::config(format!(
                "Trigger with ID '{}' already registered",
                id
            )));
        }

        triggers.insert(id.clone(), Arc::new(Mutex::new(Box::new(trigger))));

        Ok(TriggerHandle { id, trigger_type })
    }

    /// Unregister a trigger by ID.
    pub fn unregister(&self, id: &str) -> Result<(), TriggerError> {
        let mut triggers = self.triggers.write().unwrap();
        if triggers.remove(id).is_none() {
            return Err(TriggerError::config(format!(
                "Trigger with ID '{}' not found",
                id
            )));
        }
        Ok(())
    }

    /// Get a trigger handle by ID.
    pub fn get_handle(&self, id: &str) -> Option<TriggerHandle> {
        let triggers = self.triggers.read().unwrap();
        triggers.get(id).map(|t| {
            let trigger = t.lock().unwrap();
            TriggerHandle {
                id: trigger.id().to_string(),
                trigger_type: trigger.trigger_type().to_string(),
            }
        })
    }

    /// Get the status of a trigger.
    pub fn status(&self, id: &str) -> Option<TriggerStatus> {
        let triggers = self.triggers.read().unwrap();
        triggers.get(id).map(|t| t.lock().unwrap().status())
    }

    /// Start a specific trigger.
    pub fn start_trigger(&self, id: &str) -> Result<(), TriggerError> {
        let trigger = {
            let triggers = self.triggers.read().unwrap();
            triggers
                .get(id)
                .ok_or_else(|| TriggerError::config(format!("Trigger '{}' not found", id)))?
                .clone()
        };
        let result = trigger.lock().unwrap().start();
        result
    }

    /// Stop a specific trigger.
    pub fn stop_trigger(&self, id: &str) -> Result<(), TriggerError> {
        let trigger = {
            let triggers = self.triggers.read().unwrap();
            triggers
                .get(id)
                .ok_or_else(|| TriggerError::config(format!("Trigger '{}' not found", id)))?
                .clone()
        };
        let result = trigger.lock().unwrap().stop();
        result
    }

    /// Pause a specific trigger.
    pub fn pause_trigger(&self, id: &str) -> Result<(), TriggerError> {
        let trigger = {
            let triggers = self.triggers.read().unwrap();
            triggers
                .get(id)
                .ok_or_else(|| TriggerError::config(format!("Trigger '{}' not found", id)))?
                .clone()
        };
        let result = trigger.lock().unwrap().pause();
        result
    }

    /// Resume a specific trigger.
    pub fn resume_trigger(&self, id: &str) -> Result<(), TriggerError> {
        let trigger = {
            let triggers = self.triggers.read().unwrap();
            triggers
                .get(id)
                .ok_or_else(|| TriggerError::config(format!("Trigger '{}' not found", id)))?
                .clone()
        };
        let result = trigger.lock().unwrap().resume();
        result
    }

    /// Start all registered triggers.
    pub fn start_all(&self) -> Vec<Result<(), TriggerError>> {
        *self.running.lock().unwrap() = true;

        let triggers = self.triggers.read().unwrap();
        triggers
            .values()
            .map(|t| t.lock().unwrap().start())
            .collect()
    }

    /// Stop all registered triggers.
    pub fn stop_all(&self) -> Vec<Result<(), TriggerError>> {
        *self.running.lock().unwrap() = false;

        let triggers = self.triggers.read().unwrap();
        triggers
            .values()
            .map(|t| t.lock().unwrap().stop())
            .collect()
    }

    /// Poll all triggers for events.
    ///
    /// Returns events from all triggers that have pending events.
    pub fn poll_all(&self) -> Vec<TriggerEvent> {
        let triggers = self.triggers.read().unwrap();
        let mut events = Vec::new();

        for trigger in triggers.values() {
            let trigger = trigger.lock().unwrap();
            while let Some(event) = trigger.poll() {
                events.push(event);
            }
        }

        events
    }

    /// Poll a specific trigger for events.
    pub fn poll(&self, id: &str) -> Vec<TriggerEvent> {
        let triggers = self.triggers.read().unwrap();
        let Some(trigger) = triggers.get(id) else {
            return Vec::new();
        };

        let trigger = trigger.lock().unwrap();
        let mut events = Vec::new();

        while let Some(event) = trigger.poll() {
            events.push(event);
        }

        events
    }

    /// Poll for a single event from any trigger.
    pub fn poll_one(&self) -> Option<TriggerEvent> {
        let triggers = self.triggers.read().unwrap();

        for trigger in triggers.values() {
            let trigger = trigger.lock().unwrap();
            if let Some(event) = trigger.poll() {
                return Some(event);
            }
        }

        None
    }

    /// Check if any trigger has pending events.
    pub fn has_pending(&self) -> bool {
        let triggers = self.triggers.read().unwrap();
        triggers.values().any(|t| t.lock().unwrap().has_pending())
    }

    /// Get the count of registered triggers.
    pub fn trigger_count(&self) -> usize {
        self.triggers.read().unwrap().len()
    }

    /// List all registered trigger IDs.
    pub fn list_triggers(&self) -> Vec<TriggerHandle> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .values()
            .map(|t| {
                let trigger = t.lock().unwrap();
                TriggerHandle {
                    id: trigger.id().to_string(),
                    trigger_type: trigger.trigger_type().to_string(),
                }
            })
            .collect()
    }

    /// Check if manager is running.
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Get trigger by ID and apply a function to it.
    pub fn with_trigger<F, R>(&self, id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&dyn Trigger) -> R,
    {
        let triggers = self.triggers.read().unwrap();
        triggers.get(id).map(|t| {
            let trigger = t.lock().unwrap();
            f(trigger.as_ref())
        })
    }

    /// Get trigger by ID and apply a mutable function to it.
    pub fn with_trigger_mut<F, R>(&self, id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut dyn Trigger) -> R,
    {
        let triggers = self.triggers.read().unwrap();
        triggers.get(id).map(|t| {
            let mut trigger = t.lock().unwrap();
            f(trigger.as_mut())
        })
    }
}

impl std::fmt::Debug for TriggerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let triggers = self.triggers.read().unwrap();
        f.debug_struct("TriggerManager")
            .field("trigger_count", &triggers.len())
            .field("running", &*self.running.lock().unwrap())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::{QueueTrigger, ScheduleTrigger};

    #[test]
    fn test_trigger_manager_new() {
        let manager = TriggerManager::new();
        assert_eq!(manager.trigger_count(), 0);
        assert!(!manager.is_running());
    }

    #[test]
    fn test_trigger_manager_register() {
        let manager = TriggerManager::new();
        let trigger = QueueTrigger::new("queue-1");

        let handle = manager.register(trigger).unwrap();
        assert_eq!(handle.id(), "queue-1");
        assert_eq!(handle.trigger_type(), "queue");
        assert_eq!(manager.trigger_count(), 1);
    }

    #[test]
    fn test_trigger_manager_register_duplicate() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("test")).unwrap();

        let result = manager.register(QueueTrigger::new("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_manager_unregister() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("test")).unwrap();

        manager.unregister("test").unwrap();
        assert_eq!(manager.trigger_count(), 0);
    }

    #[test]
    fn test_trigger_manager_start_stop() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("test")).unwrap();

        manager.start_trigger("test").unwrap();
        assert_eq!(manager.status("test"), Some(TriggerStatus::Running));

        manager.stop_trigger("test").unwrap();
        assert_eq!(manager.status("test"), Some(TriggerStatus::Stopped));
    }

    #[test]
    fn test_trigger_manager_start_all() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("queue-1")).unwrap();
        manager.register(QueueTrigger::new("queue-2")).unwrap();

        let results = manager.start_all();
        assert!(results.iter().all(|r| r.is_ok()));
        assert!(manager.is_running());

        assert_eq!(manager.status("queue-1"), Some(TriggerStatus::Running));
        assert_eq!(manager.status("queue-2"), Some(TriggerStatus::Running));
    }

    #[test]
    fn test_trigger_manager_poll() {
        let manager = TriggerManager::new();
        let trigger = QueueTrigger::new("test");
        trigger.push("message 1").unwrap();
        trigger.push("message 2").unwrap();

        manager.register(trigger).unwrap();
        manager.start_trigger("test").unwrap();

        let events = manager.poll("test");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_trigger_manager_poll_all() {
        let manager = TriggerManager::new();

        let trigger1 = QueueTrigger::new("queue-1");
        trigger1.push("msg 1").unwrap();

        let trigger2 = QueueTrigger::new("queue-2");
        trigger2.push("msg 2").unwrap();

        manager.register(trigger1).unwrap();
        manager.register(trigger2).unwrap();
        manager.start_all();

        let events = manager.poll_all();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_trigger_manager_poll_one() {
        let manager = TriggerManager::new();

        let trigger = QueueTrigger::new("test");
        trigger.push("message").unwrap();

        manager.register(trigger).unwrap();
        manager.start_trigger("test").unwrap();

        let event = manager.poll_one();
        assert!(event.is_some());

        let event = manager.poll_one();
        assert!(event.is_none());
    }

    #[test]
    fn test_trigger_manager_list_triggers() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("queue-1")).unwrap();
        manager
            .register(ScheduleTrigger::once("schedule-1"))
            .unwrap();

        let handles = manager.list_triggers();
        assert_eq!(handles.len(), 2);

        let ids: Vec<_> = handles.iter().map(|h| h.id()).collect();
        assert!(ids.contains(&"queue-1"));
        assert!(ids.contains(&"schedule-1"));
    }

    #[test]
    fn test_trigger_manager_with_trigger() {
        let manager = TriggerManager::new();
        manager.register(QueueTrigger::new("test")).unwrap();
        manager.start_trigger("test").unwrap();

        let status = manager.with_trigger("test", |t| t.status());
        assert_eq!(status, Some(TriggerStatus::Running));
    }

    #[test]
    fn test_trigger_manager_has_pending() {
        let manager = TriggerManager::new();

        let trigger = QueueTrigger::new("test");
        manager.register(trigger).unwrap();

        // Access the trigger through with_trigger pattern
        manager.with_trigger("test", |_t| {
            // QueueTrigger has push method, but we access it via dyn Trigger
            // For this test, we need to push before registering
        });

        // Better approach: push before registering
        let trigger = QueueTrigger::new("test2");
        trigger.push("message").unwrap();
        manager.register(trigger).unwrap();

        assert!(manager.has_pending());
    }
}
