//! Event bus for publishing and subscribing to events.

use super::types::{Event, EventType};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Default channel capacity for subscriptions.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// A subscription handle for receiving events.
pub struct Subscription {
    /// The receiver for events.
    pub receiver: broadcast::Receiver<Event>,

    /// The event type this subscription is for (None = all events).
    pub event_type: Option<EventType>,
}

impl Subscription {
    /// Try to receive the next event without blocking.
    pub fn try_recv(&mut self) -> Result<Event, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive the next event, blocking until available.
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}

/// An event bus for publishing and subscribing to events.
///
/// Events can be published to all subscribers or filtered by type.
/// Uses Tokio broadcast channels for async event delivery.
///
/// # Example
///
/// ```
/// use liteforge::events::{EventBus, Event, EventType};
///
/// #[tokio::main]
/// async fn main() {
///     let bus = EventBus::new();
///
///     // Subscribe to specific event type
///     let mut sub = bus.subscribe(EventType::ToolCall);
///
///     // Publish event
///     bus.publish(Event::tool_call("calc", "{}"));
///
///     // Receive
///     if let Ok(event) = sub.try_recv() {
///         println!("Got event: {}", event.event_type);
///     }
/// }
/// ```
#[derive(Clone)]
pub struct EventBus {
    /// Broadcast sender for all events.
    all_sender: broadcast::Sender<Event>,

    /// Per-type broadcast senders.
    type_senders: Arc<RwLock<HashMap<EventType, broadcast::Sender<Event>>>>,

    /// Channel capacity.
    capacity: usize,
}

impl EventBus {
    /// Create a new event bus with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Create a new event bus with custom capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (all_sender, _) = broadcast::channel(capacity);
        Self {
            all_sender,
            type_senders: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// Subscribe to all events.
    pub fn subscribe_all(&self) -> Subscription {
        Subscription {
            receiver: self.all_sender.subscribe(),
            event_type: None,
        }
    }

    /// Subscribe to events of a specific type.
    pub fn subscribe(&self, event_type: EventType) -> Subscription {
        let sender = self.get_or_create_sender(event_type);
        Subscription {
            receiver: sender.subscribe(),
            event_type: Some(event_type),
        }
    }

    /// Subscribe to multiple event types.
    ///
    /// Returns a subscription that receives events matching any of the types.
    /// Note: This creates a subscription to all events and filters locally,
    /// which may be less efficient than single-type subscriptions.
    pub fn subscribe_many(&self, event_types: &[EventType]) -> FilteredSubscription {
        FilteredSubscription {
            receiver: self.all_sender.subscribe(),
            event_types: event_types.to_vec(),
        }
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, event: Event) {
        // Send to all-events channel
        let _ = self.all_sender.send(event.clone());

        // Send to type-specific channel if it exists
        if let Ok(senders) = self.type_senders.read() {
            if let Some(sender) = senders.get(&event.event_type) {
                let _ = sender.send(event);
            }
        }
    }

    /// Publish multiple events.
    pub fn publish_batch(&self, events: Vec<Event>) {
        for event in events {
            self.publish(event);
        }
    }

    /// Get the number of subscribers for all events.
    pub fn subscriber_count(&self) -> usize {
        self.all_sender.receiver_count()
    }

    /// Get the number of subscribers for a specific event type.
    pub fn subscriber_count_for(&self, event_type: EventType) -> usize {
        if let Ok(senders) = self.type_senders.read() {
            senders
                .get(&event_type)
                .map(|s| s.receiver_count())
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Get or create a sender for a specific event type.
    fn get_or_create_sender(&self, event_type: EventType) -> broadcast::Sender<Event> {
        // Try to get existing sender
        if let Ok(senders) = self.type_senders.read() {
            if let Some(sender) = senders.get(&event_type) {
                return sender.clone();
            }
        }

        // Create new sender
        let mut senders = self.type_senders.write().unwrap();
        senders
            .entry(event_type)
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(self.capacity);
                sender
            })
            .clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// A subscription that filters for multiple event types.
pub struct FilteredSubscription {
    receiver: broadcast::Receiver<Event>,
    event_types: Vec<EventType>,
}

impl FilteredSubscription {
    /// Try to receive the next matching event without blocking.
    pub fn try_recv(&mut self) -> Result<Event, broadcast::error::TryRecvError> {
        loop {
            let event = self.receiver.try_recv()?;
            if self.event_types.contains(&event.event_type) {
                return Ok(event);
            }
        }
    }

    /// Receive the next matching event, blocking until available.
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            if self.event_types.contains(&event.event_type) {
                return Ok(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_creation() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_subscribe_all() {
        let bus = EventBus::new();
        let _sub = bus.subscribe_all();
        assert_eq!(bus.subscriber_count(), 1);
    }

    #[test]
    fn test_subscribe_type() {
        let bus = EventBus::new();
        let _sub = bus.subscribe(EventType::ToolCall);
        assert_eq!(bus.subscriber_count_for(EventType::ToolCall), 1);
        assert_eq!(bus.subscriber_count_for(EventType::AgentStart), 0);
    }

    #[test]
    fn test_publish_and_receive() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe(EventType::ToolCall);

        bus.publish(Event::tool_call("calc", "{}"));

        let event = sub.try_recv().unwrap();
        assert_eq!(event.event_type, EventType::ToolCall);
    }

    #[test]
    fn test_type_filtering() {
        let bus = EventBus::new();
        let mut tool_sub = bus.subscribe(EventType::ToolCall);
        let mut agent_sub = bus.subscribe(EventType::AgentStart);

        bus.publish(Event::tool_call("calc", "{}"));

        // Tool subscriber should receive
        assert!(tool_sub.try_recv().is_ok());

        // Agent subscriber should not receive (different type)
        assert!(agent_sub.try_recv().is_err());
    }

    #[test]
    fn test_all_events_subscription() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe_all();

        bus.publish(Event::tool_call("calc", "{}"));
        bus.publish(Event::agent_start("agent-1"));

        // Should receive both events
        assert!(sub.try_recv().is_ok());
        assert!(sub.try_recv().is_ok());
    }

    #[test]
    fn test_publish_batch() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe_all();

        bus.publish_batch(vec![
            Event::tool_call("a", "{}"),
            Event::tool_call("b", "{}"),
        ]);

        assert!(sub.try_recv().is_ok());
        assert!(sub.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_async_receive() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe(EventType::ToolCall);

        // Spawn publisher
        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            bus_clone.publish(Event::tool_call("calc", "{}"));
        });

        // Wait for event
        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::ToolCall);
    }
}
