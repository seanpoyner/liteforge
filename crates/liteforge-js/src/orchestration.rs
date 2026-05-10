use napi::bindgen_prelude::*;
use std::collections::HashMap;
use liteforge::orchestration::{
    CommonIntents as RustCommonIntents, Intent as RustIntent, IntentRoute as RustIntentRoute,
    IntentRouter as RustIntentRouter, Session as RustSession, SessionStore as RustSessionStore,
};

#[napi(object)]
pub struct JsIntent {
    pub name: String,
    pub confidence: f64,
    pub entities: HashMap<String, String>,
    pub input: String,
}

fn rust_intent_to_js(i: &RustIntent) -> JsIntent {
    JsIntent {
        name: i.name.clone(),
        confidence: i.confidence as f64,
        entities: i.entities.clone(),
        input: i.input.clone(),
    }
}

#[napi(object)]
pub struct JsRoutingDecision {
    pub agent: String,
    pub intent: JsIntent,
    pub confidence: f64,
}

#[napi]
pub struct IntentRoute {
    inner: RustIntentRoute,
}

#[napi]
impl IntentRoute {
    #[napi(constructor)]
    pub fn new(agent: String) -> Self {
        Self {
            inner: RustIntentRoute::new(agent),
        }
    }

    #[napi]
    pub fn pattern(&mut self, pattern: String) -> &Self {
        self.inner = self.inner.clone().pattern(pattern);
        self
    }

    #[napi]
    pub fn priority(&mut self, priority: i32) -> &Self {
        self.inner = self.inner.clone().priority(priority);
        self
    }

    #[napi]
    pub fn keyword(&mut self, keyword: String) -> &Self {
        self.inner = self.inner.clone().keyword(keyword);
        self
    }
}

#[napi]
pub struct CommonIntents {}

#[napi]
impl CommonIntents {
    #[napi(factory)]
    pub fn greeting(agent: String) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::greeting(agent),
        }
    }

    #[napi(factory)]
    pub fn question(agent: String) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::question(agent),
        }
    }

    #[napi(factory)]
    pub fn code(agent: String) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::code(agent),
        }
    }

    #[napi(factory)]
    pub fn search(agent: String) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::search(agent),
        }
    }

    #[napi(factory)]
    pub fn task(agent: String) -> IntentRoute {
        IntentRoute {
            inner: RustCommonIntents::task(agent),
        }
    }
}

#[napi]
pub struct IntentRouter {
    inner: RustIntentRouter,
}

#[napi]
impl IntentRouter {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustIntentRouter::new(),
        }
    }

    #[napi]
    pub fn route(&mut self, intent_route: &IntentRoute) -> &Self {
        self.inner = self.inner.clone().route(intent_route.inner.clone());
        self
    }

    #[napi]
    pub fn default_agent(&mut self, agent: String) -> &Self {
        self.inner = self.inner.clone().default_agent(agent);
        self
    }

    #[napi]
    pub fn min_confidence(&mut self, min: f64) -> &Self {
        self.inner = self.inner.clone().min_confidence(min as f32);
        self
    }

    #[napi]
    pub fn classify_and_route(&self, input: String) -> Option<JsRoutingDecision> {
        self.inner
            .classify_and_route(&input)
            .map(|decision| JsRoutingDecision {
                agent: decision.agent.clone(),
                intent: rust_intent_to_js(&decision.intent),
                confidence: decision.confidence as f64,
            })
    }
}

#[napi(object)]
pub struct JsSessionMessage {
    pub role: String,
    pub content: String,
    pub agent: Option<String>,
    pub timestamp: i64,
}

#[napi(object)]
pub struct JsSession {
    pub id: String,
    pub history: Vec<JsSessionMessage>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: HashMap<String, serde_json::Value>,
}

fn rust_session_to_js(s: &RustSession) -> JsSession {
    JsSession {
        id: s.id.clone(),
        history: s
            .history
            .iter()
            .map(|m| JsSessionMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                agent: m.agent.clone(),
                timestamp: m.timestamp,
            })
            .collect(),
        created_at: s.created_at,
        updated_at: s.updated_at,
        metadata: s.metadata.clone(),
    }
}

#[napi]
pub struct SessionStore {
    inner: RustSessionStore,
    runtime: tokio::runtime::Runtime,
}

#[napi]
impl SessionStore {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: RustSessionStore::new(),
            runtime,
        })
    }

    #[napi]
    pub fn create(&self, id: String) -> JsSession {
        let session = self.runtime.block_on(self.inner.create(&id));
        rust_session_to_js(&session)
    }

    #[napi]
    pub fn get(&self, id: String) -> Option<JsSession> {
        let session = self.runtime.block_on(self.inner.get(&id));
        session.as_ref().map(rust_session_to_js)
    }

    #[napi]
    pub fn remove(&self, id: String) -> bool {
        self.runtime.block_on(self.inner.remove(&id)).is_some()
    }

    #[napi]
    pub fn exists(&self, id: String) -> bool {
        self.runtime.block_on(self.inner.exists(&id))
    }

    #[napi]
    pub fn list_ids(&self) -> Vec<String> {
        self.runtime.block_on(self.inner.list_ids())
    }

    #[napi]
    pub fn count(&self) -> u32 {
        self.runtime.block_on(self.inner.count()) as u32
    }

    #[napi]
    pub fn clear(&self) {
        self.runtime.block_on(self.inner.clear());
    }
}

#[napi(object)]
pub struct JsWorkflowStep {
    pub id: String,
    pub name: String,
    pub agent: String,
    pub input: serde_json::Value,
    pub depends_on: Vec<String>,
    pub condition: Option<String>,
    pub timeout_secs: Option<u32>,
    pub retries: Option<u32>,
}

#[napi(object)]
pub struct JsWorkflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<JsWorkflowStep>,
}

#[napi(string_enum)]
pub enum OrchestrationStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[napi(object)]
pub struct JsOrchestrationResult {
    pub agent: String,
    pub response: String,
    pub session_id: String,
}
