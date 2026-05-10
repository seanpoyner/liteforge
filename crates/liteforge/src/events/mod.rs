//! Event system for reactive programming patterns.
//!
//! Provides an event bus for publishing and subscribing to events
//! across different components of an application.
//!
//! # Example
//!
//! ```
//! use liteforge::events::{EventBus, Event, EventType};
//!
//! #[tokio::main]
//! async fn main() {
//!     let bus = EventBus::new();
//!
//!     // Subscribe to events
//!     let mut rx = bus.subscribe(EventType::ToolCall);
//!
//!     // Publish an event
//!     bus.publish(Event::tool_call("calculator", r#"{"op": "add"}"#));
//!
//!     // Receive event
//!     if let Ok(event) = rx.try_recv() {
//!         println!("Received: {:?}", event.event_type);
//!     }
//! }
//! ```

mod bus;
mod types;

pub use bus::{EventBus, Subscription};
pub use types::{Event, EventData, EventType};
