//! Event types for the event system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Types of events that can be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Agent events
    AgentStart,
    AgentEnd,
    AgentStep,
    AgentError,

    // Tool events
    ToolCall,
    ToolResult,
    ToolError,

    // LLM events
    LlmRequest,
    LlmResponse,
    LlmStream,
    LlmError,

    // Knowledge events
    KnowledgeSearch,
    KnowledgeUpload,
    KnowledgeDelete,

    // Custom events
    Custom,
}

impl EventType {
    /// Check if this is an agent-related event.
    pub fn is_agent_event(&self) -> bool {
        matches!(
            self,
            EventType::AgentStart
                | EventType::AgentEnd
                | EventType::AgentStep
                | EventType::AgentError
        )
    }

    /// Check if this is a tool-related event.
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self,
            EventType::ToolCall | EventType::ToolResult | EventType::ToolError
        )
    }

    /// Check if this is an LLM-related event.
    pub fn is_llm_event(&self) -> bool {
        matches!(
            self,
            EventType::LlmRequest
                | EventType::LlmResponse
                | EventType::LlmStream
                | EventType::LlmError
        )
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EventType::AgentStart => "agent_start",
            EventType::AgentEnd => "agent_end",
            EventType::AgentStep => "agent_step",
            EventType::AgentError => "agent_error",
            EventType::ToolCall => "tool_call",
            EventType::ToolResult => "tool_result",
            EventType::ToolError => "tool_error",
            EventType::LlmRequest => "llm_request",
            EventType::LlmResponse => "llm_response",
            EventType::LlmStream => "llm_stream",
            EventType::LlmError => "llm_error",
            EventType::KnowledgeSearch => "knowledge_search",
            EventType::KnowledgeUpload => "knowledge_upload",
            EventType::KnowledgeDelete => "knowledge_delete",
            EventType::Custom => "custom",
        };
        write!(f, "{}", s)
    }
}

/// Data payload for an event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventData {
    /// No data.
    #[default]
    None,

    /// String data.
    String(String),

    /// JSON data.
    Json(serde_json::Value),

    /// Key-value data.
    Map(HashMap<String, serde_json::Value>),
}

impl From<String> for EventData {
    fn from(s: String) -> Self {
        EventData::String(s)
    }
}

impl From<&str> for EventData {
    fn from(s: &str) -> Self {
        EventData::String(s.to_string())
    }
}

impl From<serde_json::Value> for EventData {
    fn from(v: serde_json::Value) -> Self {
        EventData::Json(v)
    }
}

impl From<HashMap<String, serde_json::Value>> for EventData {
    fn from(m: HashMap<String, serde_json::Value>) -> Self {
        EventData::Map(m)
    }
}

/// An event that can be published and subscribed to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID.
    pub id: String,

    /// Type of event.
    pub event_type: EventType,

    /// Event data/payload.
    pub data: EventData,

    /// Unix timestamp in milliseconds.
    pub timestamp: u64,

    /// Optional source identifier.
    pub source: Option<String>,

    /// Optional correlation ID for tracing.
    pub correlation_id: Option<String>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Event {
    /// Create a new event with the given type.
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: generate_id(),
            event_type,
            data: EventData::None,
            timestamp: now_millis(),
            source: None,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new event with data.
    pub fn with_data(event_type: EventType, data: impl Into<EventData>) -> Self {
        Self {
            id: generate_id(),
            event_type,
            data: data.into(),
            timestamp: now_millis(),
            source: None,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the correlation ID.
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    // Convenience constructors for common event types

    /// Create an agent start event.
    pub fn agent_start(agent_id: &str) -> Self {
        Self::with_data(
            EventType::AgentStart,
            serde_json::json!({"agent_id": agent_id}),
        )
        .source(agent_id)
    }

    /// Create an agent end event.
    pub fn agent_end(agent_id: &str, result: serde_json::Value) -> Self {
        Self::with_data(
            EventType::AgentEnd,
            serde_json::json!({
                "agent_id": agent_id,
                "result": result
            }),
        )
        .source(agent_id)
    }

    /// Create an agent step event.
    pub fn agent_step(agent_id: &str, step: usize, action: &str) -> Self {
        Self::with_data(
            EventType::AgentStep,
            serde_json::json!({
                "agent_id": agent_id,
                "step": step,
                "action": action
            }),
        )
        .source(agent_id)
    }

    /// Create a tool call event.
    pub fn tool_call(tool_name: &str, arguments: &str) -> Self {
        Self::with_data(
            EventType::ToolCall,
            serde_json::json!({
                "tool": tool_name,
                "arguments": arguments
            }),
        )
        .source(tool_name)
    }

    /// Create a tool result event.
    pub fn tool_result(tool_name: &str, result: serde_json::Value) -> Self {
        Self::with_data(
            EventType::ToolResult,
            serde_json::json!({
                "tool": tool_name,
                "result": result
            }),
        )
        .source(tool_name)
    }

    /// Create a tool error event.
    pub fn tool_error(tool_name: &str, error: &str) -> Self {
        Self::with_data(
            EventType::ToolError,
            serde_json::json!({
                "tool": tool_name,
                "error": error
            }),
        )
        .source(tool_name)
    }

    /// Create an LLM request event.
    pub fn llm_request(model: &str, message_count: usize) -> Self {
        Self::with_data(
            EventType::LlmRequest,
            serde_json::json!({
                "model": model,
                "message_count": message_count
            }),
        )
        .source(model)
    }

    /// Create an LLM response event.
    pub fn llm_response(model: &str, tokens: u32) -> Self {
        Self::with_data(
            EventType::LlmResponse,
            serde_json::json!({
                "model": model,
                "tokens": tokens
            }),
        )
        .source(model)
    }

    /// Create a custom event.
    pub fn custom(name: &str, data: serde_json::Value) -> Self {
        Self::with_data(EventType::Custom, data).metadata("event_name", serde_json::json!(name))
    }
}

/// Generate a unique ID.
fn generate_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let ts = now_millis();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("evt_{:x}_{:04x}", ts, count & 0xFFFF)
}

/// Get current time in milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::AgentStart.to_string(), "agent_start");
        assert_eq!(EventType::ToolCall.to_string(), "tool_call");
    }

    #[test]
    fn test_event_type_categories() {
        assert!(EventType::AgentStart.is_agent_event());
        assert!(!EventType::AgentStart.is_tool_event());

        assert!(EventType::ToolCall.is_tool_event());
        assert!(!EventType::ToolCall.is_agent_event());

        assert!(EventType::LlmRequest.is_llm_event());
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventType::AgentStart);
        assert_eq!(event.event_type, EventType::AgentStart);
        assert!(!event.id.is_empty());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_event_with_data() {
        let event = Event::with_data(EventType::ToolCall, "test data");
        match event.data {
            EventData::String(s) => assert_eq!(s, "test data"),
            _ => panic!("Expected string data"),
        }
    }

    #[test]
    fn test_event_builder() {
        let event = Event::new(EventType::Custom)
            .source("my-source")
            .correlation_id("corr-123")
            .metadata("key", serde_json::json!("value"));

        assert_eq!(event.source, Some("my-source".to_string()));
        assert_eq!(event.correlation_id, Some("corr-123".to_string()));
        assert_eq!(event.metadata.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_convenience_constructors() {
        let event = Event::agent_start("agent-1");
        assert_eq!(event.event_type, EventType::AgentStart);
        assert_eq!(event.source, Some("agent-1".to_string()));

        let event = Event::tool_call("calculator", r#"{"x": 1}"#);
        assert_eq!(event.event_type, EventType::ToolCall);
    }

    #[test]
    fn test_event_data_from() {
        let _data: EventData = "hello".into();
        let _data: EventData = String::from("hello").into();
        let _data: EventData = serde_json::json!({"key": "value"}).into();
    }
}
