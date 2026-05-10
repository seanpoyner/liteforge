//! Pipelines for composable model transformations.
//!
//! Pipelines allow chaining multiple processing steps together,
//! with support for model transformations and adapters.

mod transform;

pub use transform::{ModelTransform, TransformChain};

use crate::client::AsyncForgeClient;
use crate::error::ForgeError;
use crate::types::{ChatCompletionRequest, Message};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for step transform functions to reduce complexity.
type StepTransformFn =
    Box<dyn Fn(&str, &mut PipelineContext) -> PipelineResult<String> + Send + Sync>;

/// Error type for pipeline operations.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// Step execution failed.
    #[error("Pipeline step '{0}' failed: {1}")]
    StepFailed(String, String),

    /// Invalid pipeline configuration.
    #[error("Invalid pipeline configuration: {0}")]
    InvalidConfig(String),

    /// Transform error.
    #[error("Transform error: {0}")]
    TransformError(String),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] ForgeError),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// Result type for pipeline operations.
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Context passed through pipeline steps.
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Key-value store for passing data between steps.
    pub data: HashMap<String, serde_json::Value>,
    /// Messages accumulated through the pipeline.
    pub messages: Vec<Message>,
    /// Metadata about the pipeline execution.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PipelineContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with initial messages.
    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self {
            messages,
            ..Default::default()
        }
    }

    /// Set a value in the context.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
    }

    /// Get a value from the context.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Get a string value from the context.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Add a message to the context.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
}

/// Output from a pipeline step.
#[derive(Debug, Clone)]
pub struct StepOutput {
    /// The text output from this step.
    pub text: String,
    /// Optional structured data.
    pub data: Option<serde_json::Value>,
    /// Whether to continue to the next step.
    pub continue_pipeline: bool,
}

impl StepOutput {
    /// Create a new step output.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            data: None,
            continue_pipeline: true,
        }
    }

    /// Set structured data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Mark this as a terminal step (stop pipeline).
    pub fn stop(mut self) -> Self {
        self.continue_pipeline = false;
        self
    }
}

/// Trait for pipeline steps.
#[async_trait]
pub trait PipelineStep: Send + Sync {
    /// Get the step name.
    fn name(&self) -> &str;

    /// Execute this step.
    async fn execute(
        &self,
        client: &AsyncForgeClient,
        context: &mut PipelineContext,
    ) -> PipelineResult<StepOutput>;
}

/// A step that makes an LLM call.
pub struct LlmStep {
    name: String,
    system_prompt: Option<String>,
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl LlmStep {
    /// Create a new LLM step.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }
}

#[async_trait]
impl PipelineStep for LlmStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        client: &AsyncForgeClient,
        context: &mut PipelineContext,
    ) -> PipelineResult<StepOutput> {
        let mut messages = Vec::new();

        // Add system prompt if specified
        if let Some(ref prompt) = self.system_prompt {
            messages.push(Message::system(prompt));
        }

        // Add accumulated messages from context
        messages.extend(context.messages.clone());

        // Build request
        let model = self
            .model
            .clone()
            .unwrap_or_else(|| client.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);

        if let Some(temp) = self.temperature {
            request = request.temperature(temp);
        }

        if let Some(max_tokens) = self.max_tokens {
            request = request.max_tokens(max_tokens);
        }

        // Execute
        let response = client.chat_completions(request).await?;

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        // Add assistant response to context
        context.add_message(Message::assistant(&text));

        Ok(StepOutput::new(text))
    }
}

/// A step that transforms text.
pub struct TransformStep {
    name: String,
    transform: StepTransformFn,
}

impl TransformStep {
    /// Create a new transform step.
    pub fn new<F>(name: impl Into<String>, transform: F) -> Self
    where
        F: Fn(&str, &mut PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            transform: Box::new(transform),
        }
    }
}

#[async_trait]
impl PipelineStep for TransformStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        _client: &AsyncForgeClient,
        context: &mut PipelineContext,
    ) -> PipelineResult<StepOutput> {
        // Get last message text and clone it to release the borrow
        let input = context
            .messages
            .last()
            .and_then(|m| m.content.as_ref())
            .cloned()
            .unwrap_or_default();

        let output = (self.transform)(&input, context)?;
        Ok(StepOutput::new(output))
    }
}

/// A step that branches based on a condition.
pub struct BranchStep {
    name: String,
    condition: Box<dyn Fn(&PipelineContext) -> bool + Send + Sync>,
    if_true: Arc<dyn PipelineStep>,
    if_false: Option<Arc<dyn PipelineStep>>,
}

impl BranchStep {
    /// Create a new branch step.
    pub fn new<F>(
        name: impl Into<String>,
        condition: F,
        if_true: impl PipelineStep + 'static,
    ) -> Self
    where
        F: Fn(&PipelineContext) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            condition: Box::new(condition),
            if_true: Arc::new(if_true),
            if_false: None,
        }
    }

    /// Set the step to execute if condition is false.
    pub fn with_else(mut self, step: impl PipelineStep + 'static) -> Self {
        self.if_false = Some(Arc::new(step));
        self
    }
}

#[async_trait]
impl PipelineStep for BranchStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        client: &AsyncForgeClient,
        context: &mut PipelineContext,
    ) -> PipelineResult<StepOutput> {
        if (self.condition)(context) {
            self.if_true.execute(client, context).await
        } else if let Some(ref step) = self.if_false {
            step.execute(client, context).await
        } else {
            // No else branch, just pass through
            let text = context
                .messages
                .last()
                .and_then(|m| m.content.as_ref())
                .cloned()
                .unwrap_or_default();
            Ok(StepOutput::new(text))
        }
    }
}

/// A pipeline that chains multiple steps.
pub struct Pipeline {
    name: String,
    description: String,
    steps: Vec<Arc<dyn PipelineStep>>,
}

impl Pipeline {
    /// Create a new pipeline.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            steps: Vec::new(),
        }
    }

    /// Add a step to the pipeline.
    pub fn add_step(mut self, step: impl PipelineStep + 'static) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    /// Get the pipeline name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the pipeline description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Execute the pipeline.
    pub async fn execute(
        &self,
        client: &AsyncForgeClient,
        input: impl Into<String>,
    ) -> PipelineResult<PipelineOutput> {
        let mut context = PipelineContext::new();
        context.add_message(Message::user(input.into()));

        let mut step_outputs = Vec::new();

        for step in &self.steps {
            let output = step
                .execute(client, &mut context)
                .await
                .map_err(|e| PipelineError::StepFailed(step.name().to_string(), e.to_string()))?;

            step_outputs.push((step.name().to_string(), output.clone()));

            if !output.continue_pipeline {
                break;
            }

            // If step produced text, add it as context for next step
            if !output.text.is_empty() {
                context.set("last_output", serde_json::json!(output.text));
            }
        }

        let final_text = step_outputs
            .last()
            .map(|(_, o)| o.text.clone())
            .unwrap_or_default();

        Ok(PipelineOutput {
            text: final_text,
            steps: step_outputs,
            context,
        })
    }
}

/// Output from a pipeline execution.
#[derive(Debug)]
pub struct PipelineOutput {
    /// The final text output.
    pub text: String,
    /// Outputs from each step.
    pub steps: Vec<(String, StepOutput)>,
    /// The final context.
    pub context: PipelineContext,
}

/// Builder for pipelines.
pub struct PipelineBuilder {
    name: String,
    description: String,
    steps: Vec<Arc<dyn PipelineStep>>,
}

impl PipelineBuilder {
    /// Create a new pipeline builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            steps: Vec::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add an LLM step.
    pub fn llm(mut self, name: impl Into<String>) -> Self {
        self.steps.push(Arc::new(LlmStep::new(name)));
        self
    }

    /// Add an LLM step with configuration.
    pub fn llm_with(mut self, step: LlmStep) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    /// Add a transform step.
    pub fn transform<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&str, &mut PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
    {
        self.steps.push(Arc::new(TransformStep::new(name, f)));
        self
    }

    /// Add a custom step.
    pub fn step(mut self, step: impl PipelineStep + 'static) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    /// Build the pipeline.
    pub fn build(self) -> Pipeline {
        Pipeline {
            name: self.name,
            description: self.description,
            steps: self.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_context() {
        let mut ctx = PipelineContext::new();

        ctx.set("key", serde_json::json!("value"));
        assert_eq!(ctx.get_string("key"), Some("value"));

        ctx.add_message(Message::user("Hello"));
        assert_eq!(ctx.messages.len(), 1);
    }

    #[test]
    fn test_step_output() {
        let output = StepOutput::new("Hello").with_data(serde_json::json!({"count": 1}));

        assert_eq!(output.text, "Hello");
        assert!(output.data.is_some());
        assert!(output.continue_pipeline);

        let stopped = StepOutput::new("Done").stop();
        assert!(!stopped.continue_pipeline);
    }

    #[test]
    fn test_llm_step_builder() {
        let step = LlmStep::new("test")
            .with_system_prompt("Be helpful")
            .with_model("gpt-4")
            .with_temperature(0.7)
            .with_max_tokens(100);

        assert_eq!(step.name, "test");
        assert_eq!(step.system_prompt, Some("Be helpful".to_string()));
        assert_eq!(step.model, Some("gpt-4".to_string()));
        assert_eq!(step.temperature, Some(0.7));
        assert_eq!(step.max_tokens, Some(100));
    }

    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new("test-pipeline")
            .description("A test pipeline")
            .llm("step1")
            .transform("uppercase", |s, _| Ok(s.to_uppercase()))
            .build();

        assert_eq!(pipeline.name(), "test-pipeline");
        assert_eq!(pipeline.description(), "A test pipeline");
        assert_eq!(pipeline.step_count(), 2);
    }

    #[test]
    fn test_pipeline_new() {
        let pipeline =
            Pipeline::new("my-pipeline", "Does something").add_step(LlmStep::new("process"));

        assert_eq!(pipeline.name(), "my-pipeline");
        assert_eq!(pipeline.step_count(), 1);
    }

    #[test]
    fn test_transform_step() {
        let step = TransformStep::new("reverse", |s, _| Ok(s.chars().rev().collect()));

        assert_eq!(step.name(), "reverse");
    }
}
