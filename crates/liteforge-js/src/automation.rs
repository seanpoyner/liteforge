use napi::bindgen_prelude::*;
use liteforge::automation::AutomationBuilder as RustAutomationBuilder;

#[napi(string_enum)]
pub enum AutomationTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[napi(string_enum)]
pub enum AutomationScheduleConfig {
    Once,
    Interval,
    Cron,
}

#[napi(object)]
pub struct JsAutomationConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub max_retries: u32,
    pub timeout_secs: u32,
}

#[napi(object)]
pub struct JsTaskContext {
    pub input: Option<serde_json::Value>,
    pub state: serde_json::Value,
}

#[napi(object)]
pub struct JsTaskOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub duration_ms: u32,
}

#[napi(object)]
pub struct JsExecutionRecord {
    pub execution_id: String,
    pub task_id: String,
    pub status: String,
    pub started_at: u32,
    pub ended_at: Option<u32>,
}

#[napi]
pub struct AutomationBuilder {
    inner: Option<RustAutomationBuilder>,
}

#[napi]
impl AutomationBuilder {
    #[napi(constructor)]
    pub fn new(id: String) -> Self {
        Self {
            inner: Some(RustAutomationBuilder::new(id)),
        }
    }

    #[napi]
    pub fn name(&mut self, name: String) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.name(name));
        }
        self
    }

    #[napi]
    pub fn description(&mut self, desc: String) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.description(desc));
        }
        self
    }

    #[napi]
    pub fn every_seconds(&mut self, seconds: u32) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.every_seconds(seconds as u64));
        }
        self
    }

    #[napi]
    pub fn every_minutes(&mut self, minutes: u32) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.every_minutes(minutes as u64));
        }
        self
    }

    #[napi]
    pub fn every_hours(&mut self, hours: u32) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.every_hours(hours as u64));
        }
        self
    }

    #[napi]
    pub fn cron(&mut self, expression: String) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.cron(expression));
        }
        self
    }

    #[napi]
    pub fn retries(&mut self, max_retries: u32) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.retries(max_retries));
        }
        self
    }

    #[napi]
    pub fn timeout(&mut self, timeout_secs: u32) -> &Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.timeout(timeout_secs as u64));
        }
        self
    }

    #[napi]
    pub fn build(&mut self) -> Result<JsAutomationConfig> {
        let builder = self
            .inner
            .take()
            .ok_or_else(|| Error::from_reason("Builder already consumed"))?;
        let config = builder.build();
        Ok(JsAutomationConfig {
            id: config.id.clone(),
            name: config.name.clone(),
            description: config.description.clone(),
            enabled: config.enabled,
            max_retries: config.max_retries,
            timeout_secs: config.timeout_secs as u32,
        })
    }
}
