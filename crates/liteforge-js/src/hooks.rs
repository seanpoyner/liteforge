use serde_json::Value as JsonValue;
use liteforge::hooks::{
    Hook as RustHook, HookContext as RustHookContext, HookEvent as RustHookEvent,
    HookManager as RustHookManager, HookResult as RustHookResult,
};

#[napi]
pub struct HookEvent {
    inner: RustHookEvent,
}

#[napi]
impl HookEvent {
    #[napi(factory)]
    pub fn before_agent_start() -> Self {
        Self {
            inner: RustHookEvent::BeforeAgentStart,
        }
    }
    #[napi(factory)]
    pub fn after_agent_end() -> Self {
        Self {
            inner: RustHookEvent::AfterAgentEnd,
        }
    }
    #[napi(factory)]
    pub fn before_agent_step() -> Self {
        Self {
            inner: RustHookEvent::BeforeAgentStep,
        }
    }
    #[napi(factory)]
    pub fn after_agent_step() -> Self {
        Self {
            inner: RustHookEvent::AfterAgentStep,
        }
    }
    #[napi(factory)]
    pub fn before_tool_call() -> Self {
        Self {
            inner: RustHookEvent::BeforeToolCall,
        }
    }
    #[napi(factory)]
    pub fn after_tool_call() -> Self {
        Self {
            inner: RustHookEvent::AfterToolCall,
        }
    }
    #[napi(factory)]
    pub fn before_llm_request() -> Self {
        Self {
            inner: RustHookEvent::BeforeLlmRequest,
        }
    }
    #[napi(factory)]
    pub fn after_llm_response() -> Self {
        Self {
            inner: RustHookEvent::AfterLlmResponse,
        }
    }
    #[napi(factory)]
    pub fn before_knowledge_search() -> Self {
        Self {
            inner: RustHookEvent::BeforeKnowledgeSearch,
        }
    }
    #[napi(factory)]
    pub fn after_knowledge_search() -> Self {
        Self {
            inner: RustHookEvent::AfterKnowledgeSearch,
        }
    }

    #[napi]
    pub fn as_string(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[napi(object)]
pub struct JsHookContext {
    pub event: String,
    pub data: serde_json::Value,
    pub correlation_id: Option<String>,
}

#[napi(string_enum)]
pub enum JsHookResult {
    Continue,
    Skip,
    Abort,
}

struct JsHookImpl {
    name: String,
    priority: i32,
    result: RustHookResult,
}

impl RustHook for JsHookImpl {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn on_event(&self, _ctx: &RustHookContext) -> RustHookResult {
        self.result.clone()
    }
}

#[napi]
pub struct HookManager {
    inner: RustHookManager,
}

#[napi]
impl HookManager {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustHookManager::new(),
        }
    }

    #[napi]
    pub fn register_continue(&self, name: String, priority: Option<i32>) {
        let hook = JsHookImpl {
            name,
            priority: priority.unwrap_or(100),
            result: RustHookResult::Continue,
        };
        self.inner.register(Box::new(hook));
    }

    #[napi]
    pub fn register_skip(&self, name: String, priority: Option<i32>) {
        let hook = JsHookImpl {
            name,
            priority: priority.unwrap_or(100),
            result: RustHookResult::Skip,
        };
        self.inner.register(Box::new(hook));
    }

    #[napi]
    pub fn register_abort(&self, name: String, message: String, priority: Option<i32>) {
        let hook = JsHookImpl {
            name,
            priority: priority.unwrap_or(100),
            result: RustHookResult::Abort(message),
        };
        self.inner.register(Box::new(hook));
    }

    #[napi]
    pub fn unregister(&self, name: String) -> bool {
        self.inner.unregister(&name)
    }

    #[napi]
    pub fn run(&self, event: &HookEvent, data: Option<serde_json::Value>) -> String {
        let ctx = RustHookContext::new(event.inner).data(data.unwrap_or(JsonValue::Null));
        let result = self.inner.run(&ctx);
        match result {
            RustHookResult::Continue => "continue".to_string(),
            RustHookResult::ContinueWith(v) => format!(
                "continue_with:{}",
                serde_json::to_string(&v).unwrap_or_default()
            ),
            RustHookResult::Skip => "skip".to_string(),
            RustHookResult::SkipWith(v) => format!(
                "skip_with:{}",
                serde_json::to_string(&v).unwrap_or_default()
            ),
            RustHookResult::Abort(msg) => format!("abort:{}", msg),
        }
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn hook_names(&self) -> Vec<String> {
        self.inner.hook_names()
    }

    #[napi]
    pub fn clear(&self) {
        self.inner.clear();
    }
}
