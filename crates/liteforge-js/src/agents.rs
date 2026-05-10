use crate::types::*;
use napi::bindgen_prelude::*;
use std::sync::Arc;
use liteforge::agents::{
    Agent, AgentConfig as RustAgentConfig, AgentMemory as RustAgentMemory,
    AgentState as RustAgentState, AgentStep as RustAgentStep, StepResult as RustStepResult,
    StepType as RustStepType, ToolCallingAgent as RustToolCallingAgent,
};
use liteforge::tools::ToolRegistry as RustToolRegistry;
use tokio::sync::Mutex;

#[napi(string_enum)]
pub enum AgentState {
    Idle,
    Thinking,
    ExecutingTool,
    WaitingForHuman,
    Completed,
    Error,
    Stopped,
}

fn rust_agent_state_to_js(s: &RustAgentState) -> AgentState {
    match s {
        RustAgentState::Idle => AgentState::Idle,
        RustAgentState::Thinking => AgentState::Thinking,
        RustAgentState::ExecutingTool => AgentState::ExecutingTool,
        RustAgentState::WaitingForHuman => AgentState::WaitingForHuman,
        RustAgentState::Completed => AgentState::Completed,
        RustAgentState::Error => AgentState::Error,
        RustAgentState::Stopped => AgentState::Stopped,
    }
}

#[napi]
pub struct JsAgentMemory {
    inner: RustAgentMemory,
}

#[napi]
impl JsAgentMemory {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustAgentMemory::default(),
        }
    }

    #[napi]
    pub fn add_message(&mut self, role: String, content: String) {
        use liteforge::Message as RustMessage;
        let msg = match role.as_str() {
            "user" => RustMessage::user(content),
            "assistant" => RustMessage::assistant(content),
            "system" => RustMessage::system(content),
            _ => RustMessage::user(content),
        };
        self.inner.add_message(msg);
    }

    #[napi]
    pub fn messages(&self) -> Vec<Message> {
        self.inner
            .messages()
            .iter()
            .map(rust_message_to_js)
            .collect()
    }

    #[napi]
    pub fn clear_short_term(&mut self) {
        self.inner.clear_short_term();
    }

    #[napi]
    pub fn remember(&mut self, key: String, value: serde_json::Value) {
        self.inner.remember(key, value);
    }

    #[napi]
    pub fn recall(&self, key: String) -> Option<serde_json::Value> {
        self.inner.recall(&key).cloned()
    }

    #[napi]
    pub fn forget(&mut self, key: String) -> bool {
        self.inner.forget(&key).is_some()
    }

    #[napi]
    pub fn set_working(&mut self, key: String, value: serde_json::Value) {
        self.inner.set_working(key, value);
    }

    #[napi]
    pub fn get_working(&self, key: String) -> Option<serde_json::Value> {
        self.inner.get_working(&key).cloned()
    }

    #[napi]
    pub fn clear_working(&mut self) {
        self.inner.clear_working();
    }

    #[napi]
    pub fn message_count(&self) -> u32 {
        self.inner.message_count() as u32
    }
}

#[napi]
pub struct JsAgentConfig {
    inner: RustAgentConfig,
}

#[napi]
impl JsAgentConfig {
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            inner: RustAgentConfig::new(name),
        }
    }

    #[napi]
    pub fn with_system_prompt(&mut self, prompt: String) -> &Self {
        self.inner = self.inner.clone().with_system_prompt(prompt);
        self
    }

    #[napi]
    pub fn with_model(&mut self, model: String) -> &Self {
        self.inner = self.inner.clone().with_model(model);
        self
    }

    #[napi]
    pub fn with_max_steps(&mut self, max_steps: u32) -> &Self {
        self.inner = self.inner.clone().with_max_steps(max_steps as usize);
        self
    }

    #[napi]
    pub fn with_temperature(&mut self, temperature: f64) -> &Self {
        self.inner = self.inner.clone().with_temperature(temperature as f32);
        self
    }

    #[napi]
    pub fn with_max_tokens(&mut self, max_tokens: u32) -> &Self {
        self.inner = self.inner.clone().with_max_tokens(max_tokens);
        self
    }

    #[napi]
    pub fn with_streaming(&mut self, streaming: bool) -> &Self {
        self.inner = self.inner.clone().with_streaming(streaming);
        self
    }

    #[napi]
    pub fn with_tool(&mut self, tool_name: String) -> &Self {
        self.inner = self.inner.clone().with_tool(tool_name);
        self
    }

    #[napi(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[napi(getter)]
    pub fn system_prompt(&self) -> Option<String> {
        self.inner.system_prompt.clone()
    }

    #[napi(getter)]
    pub fn model(&self) -> Option<String> {
        self.inner.model.clone()
    }

    #[napi(getter)]
    pub fn max_steps(&self) -> u32 {
        self.inner.max_steps as u32
    }
}

#[napi(object)]
pub struct JsStepType {
    pub kind: String,
    pub tool_name: Option<String>,
    pub call_id: Option<String>,
}

fn rust_step_type_to_js(st: &RustStepType) -> JsStepType {
    match st {
        RustStepType::Think => JsStepType {
            kind: "think".to_string(),
            tool_name: None,
            call_id: None,
        },
        RustStepType::ToolCall { tool_name, call_id } => JsStepType {
            kind: "tool_call".to_string(),
            tool_name: Some(tool_name.clone()),
            call_id: Some(call_id.clone()),
        },
        RustStepType::ToolResult { tool_name, call_id } => JsStepType {
            kind: "tool_result".to_string(),
            tool_name: Some(tool_name.clone()),
            call_id: Some(call_id.clone()),
        },
        RustStepType::Response => JsStepType {
            kind: "response".to_string(),
            tool_name: None,
            call_id: None,
        },
        RustStepType::Error => JsStepType {
            kind: "error".to_string(),
            tool_name: None,
            call_id: None,
        },
        RustStepType::WaitForHuman => JsStepType {
            kind: "wait_for_human".to_string(),
            tool_name: None,
            call_id: None,
        },
    }
}

#[napi(object)]
pub struct JsStepResult {
    pub kind: String,
    pub response: Option<String>,
    pub tool_calls: Option<Vec<JsToolCall>>,
    pub prompt: Option<String>,
    pub error: Option<String>,
}

fn rust_step_result_to_js(sr: &RustStepResult) -> JsStepResult {
    match sr {
        RustStepResult::Continue => JsStepResult {
            kind: "continue".to_string(),
            response: None,
            tool_calls: None,
            prompt: None,
            error: None,
        },
        RustStepResult::Done { response } => JsStepResult {
            kind: "done".to_string(),
            response: Some(response.clone()),
            tool_calls: None,
            prompt: None,
            error: None,
        },
        RustStepResult::ToolCalls { calls } => JsStepResult {
            kind: "tool_calls".to_string(),
            response: None,
            tool_calls: Some(
                calls
                    .iter()
                    .map(|tc| JsToolCall {
                        index: tc.index,
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: JsFunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect(),
            ),
            prompt: None,
            error: None,
        },
        RustStepResult::WaitForHuman { prompt } => JsStepResult {
            kind: "wait_for_human".to_string(),
            response: None,
            tool_calls: None,
            prompt: Some(prompt.clone()),
            error: None,
        },
        RustStepResult::Error { message } => JsStepResult {
            kind: "error".to_string(),
            response: None,
            tool_calls: None,
            prompt: None,
            error: Some(message.clone()),
        },
    }
}

#[napi(object)]
pub struct JsAgentStep {
    pub step_number: u32,
    pub step_type: JsStepType,
    pub result: JsStepResult,
    pub duration_ms: f64,
}

fn rust_agent_step_to_js(s: &RustAgentStep) -> JsAgentStep {
    JsAgentStep {
        step_number: s.step_number as u32,
        step_type: rust_step_type_to_js(&s.step_type),
        result: rust_step_result_to_js(&s.result),
        duration_ms: s.duration.as_millis() as f64,
    }
}

#[napi]
pub struct ToolCallingAgent {
    inner: Arc<Mutex<RustToolCallingAgent>>,
}

#[napi]
impl ToolCallingAgent {
    #[napi(constructor)]
    pub fn new(client: &crate::client::AsyncForgeClient) -> Self {
        let rust_client = client.inner.clone();
        let rust_registry = RustToolRegistry::new();
        Self {
            inner: Arc::new(Mutex::new(RustToolCallingAgent::new(
                rust_client,
                rust_registry,
            ))),
        }
    }

    #[napi]
    pub async fn run(&self, input: String) -> Result<String> {
        let mut agent = self.inner.lock().await;
        let result = agent
            .run(&input)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result)
    }

    #[napi]
    pub async fn name(&self) -> String {
        let agent = self.inner.lock().await;
        agent.name().to_string()
    }

    #[napi]
    pub async fn history(&self) -> Vec<JsAgentStep> {
        let agent = self.inner.lock().await;
        agent.history().iter().map(rust_agent_step_to_js).collect()
    }

    #[napi]
    pub async fn clear_history(&self) {
        let mut agent = self.inner.lock().await;
        agent.clear_history();
    }
}
