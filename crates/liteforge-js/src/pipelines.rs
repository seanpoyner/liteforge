use liteforge::pipelines::PipelineContext as RustPipelineContext;

#[napi]
pub struct PipelineContext {
    inner: RustPipelineContext,
}

#[napi]
impl PipelineContext {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustPipelineContext::new(),
        }
    }

    #[napi]
    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.inner.set(key, value);
    }

    #[napi]
    pub fn get(&self, key: String) -> Option<serde_json::Value> {
        self.inner.get(&key).cloned()
    }

    #[napi]
    pub fn get_string(&self, key: String) -> Option<String> {
        self.inner.get_string(&key).map(|s| s.to_string())
    }

    #[napi]
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.inner.set_metadata(key, value);
    }
}

#[napi(object)]
pub struct PipelineStepOutput {
    pub text: String,
    pub data: serde_json::Value,
    pub continue_pipeline: bool,
}

#[napi]
pub fn create_step_output(text: String) -> PipelineStepOutput {
    PipelineStepOutput {
        text,
        data: serde_json::Value::Null,
        continue_pipeline: true,
    }
}

#[napi]
pub fn create_step_output_with_data(text: String, data: serde_json::Value) -> PipelineStepOutput {
    PipelineStepOutput {
        text,
        data,
        continue_pipeline: true,
    }
}

#[napi]
pub fn create_stop_output(text: String) -> PipelineStepOutput {
    PipelineStepOutput {
        text,
        data: serde_json::Value::Null,
        continue_pipeline: false,
    }
}
