use std::sync::Arc;
use liteforge::events::{
    Event as RustEvent, EventBus as RustEventBus, EventData as RustEventData,
    EventType as RustEventType,
};

#[napi]
pub struct EventType {
    pub(crate) inner: RustEventType,
}

#[napi]
impl EventType {
    #[napi(factory)]
    pub fn agent_start() -> Self {
        Self {
            inner: RustEventType::AgentStart,
        }
    }
    #[napi(factory)]
    pub fn agent_end() -> Self {
        Self {
            inner: RustEventType::AgentEnd,
        }
    }
    #[napi(factory)]
    pub fn agent_step() -> Self {
        Self {
            inner: RustEventType::AgentStep,
        }
    }
    #[napi(factory)]
    pub fn agent_error() -> Self {
        Self {
            inner: RustEventType::AgentError,
        }
    }
    #[napi(factory)]
    pub fn tool_call() -> Self {
        Self {
            inner: RustEventType::ToolCall,
        }
    }
    #[napi(factory)]
    pub fn tool_result() -> Self {
        Self {
            inner: RustEventType::ToolResult,
        }
    }
    #[napi(factory)]
    pub fn tool_error() -> Self {
        Self {
            inner: RustEventType::ToolError,
        }
    }
    #[napi(factory)]
    pub fn llm_request() -> Self {
        Self {
            inner: RustEventType::LlmRequest,
        }
    }
    #[napi(factory)]
    pub fn llm_response() -> Self {
        Self {
            inner: RustEventType::LlmResponse,
        }
    }
    #[napi(factory)]
    pub fn llm_stream() -> Self {
        Self {
            inner: RustEventType::LlmStream,
        }
    }
    #[napi(factory)]
    pub fn llm_error() -> Self {
        Self {
            inner: RustEventType::LlmError,
        }
    }
    #[napi(factory)]
    pub fn knowledge_search() -> Self {
        Self {
            inner: RustEventType::KnowledgeSearch,
        }
    }
    #[napi(factory)]
    pub fn knowledge_upload() -> Self {
        Self {
            inner: RustEventType::KnowledgeUpload,
        }
    }
    #[napi(factory)]
    pub fn knowledge_delete() -> Self {
        Self {
            inner: RustEventType::KnowledgeDelete,
        }
    }
    #[napi(factory)]
    pub fn custom() -> Self {
        Self {
            inner: RustEventType::Custom,
        }
    }

    #[napi]
    pub fn is_agent_event(&self) -> bool {
        self.inner.is_agent_event()
    }

    #[napi]
    pub fn is_tool_event(&self) -> bool {
        self.inner.is_tool_event()
    }

    #[napi]
    pub fn is_llm_event(&self) -> bool {
        self.inner.is_llm_event()
    }

    #[napi]
    pub fn as_string(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[napi(object)]
pub struct JsEvent {
    pub id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub source: Option<String>,
    pub correlation_id: Option<String>,
}

fn rust_event_to_js(e: &RustEvent) -> JsEvent {
    let data = match &e.data {
        RustEventData::None => serde_json::Value::Null,
        RustEventData::String(s) => serde_json::Value::String(s.clone()),
        RustEventData::Json(v) => v.clone(),
        RustEventData::Map(m) => serde_json::to_value(m).unwrap_or(serde_json::Value::Null),
    };
    JsEvent {
        id: e.id.clone(),
        event_type: format!("{:?}", e.event_type),
        data,
        source: e.source.clone(),
        correlation_id: e.correlation_id.clone(),
    }
}

#[napi]
pub struct EventBus {
    inner: Arc<RustEventBus>,
}

#[napi]
impl EventBus {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RustEventBus::new()),
        }
    }

    #[napi(factory)]
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            inner: Arc::new(RustEventBus::with_capacity(capacity as usize)),
        }
    }

    #[napi]
    pub fn publish(&self, event_type: &EventType, data: Option<serde_json::Value>) -> String {
        let rust_data = match data {
            Some(v) => RustEventData::Json(v),
            None => RustEventData::None,
        };
        let event = RustEvent::with_data(event_type.inner, rust_data);
        let id = event.id.clone();
        self.inner.publish(event);
        id
    }

    #[napi]
    pub fn subscriber_count(&self) -> u32 {
        self.inner.subscriber_count() as u32
    }
}
