//! Skills for composable AI capabilities.
//!
//! Skills are reusable, composable units of AI functionality that can be
//! loaded from files, combined, and executed.

mod composer;
mod loader;

pub use composer::{ComposedSkill, CompositionStrategy, SkillComposer};
pub use loader::{SkillLoader, SkillSource};

use crate::client::AsyncForgeClient;
use crate::error::ForgeError;
use crate::types::{ChatCompletionRequest, Message};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Error type for skill operations.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// Skill not found.
    #[error("Skill not found: {0}")]
    NotFound(String),

    /// Invalid skill configuration.
    #[error("Invalid skill configuration: {0}")]
    InvalidConfig(String),

    /// Skill execution failed.
    #[error("Skill execution failed: {0}")]
    ExecutionFailed(String),

    /// Skill composition error.
    #[error("Skill composition error: {0}")]
    CompositionError(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] ForgeError),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// Result type for skill operations.
pub type SkillResult<T> = Result<T, SkillError>;

/// Configuration for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Skill name/identifier.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Version string.
    #[serde(default = "default_version")]
    pub version: String,
    /// Model to use.
    #[serde(default)]
    pub model: Option<String>,
    /// System prompt for the skill.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Temperature for LLM.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Maximum tokens.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Input parameters schema.
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    /// Output format/schema.
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            name: "unnamed-skill".to_string(),
            description: String::new(),
            version: default_version(),
            model: None,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            input_schema: None,
            output_schema: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl SkillConfig {
    /// Create a new skill config.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            ..Default::default()
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Input to a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    /// The main text input.
    pub text: String,
    /// Additional parameters.
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    /// Context from previous skill executions.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

impl SkillInput {
    /// Create a new skill input.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            params: HashMap::new(),
            context: None,
        }
    }

    /// Add a parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Set context.
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

impl From<&str> for SkillInput {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SkillInput {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Output from a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// The main text output.
    pub text: String,
    /// Structured data (if any).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Metadata about the execution.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SkillOutput {
    /// Create a new skill output.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            data: None,
            metadata: HashMap::new(),
        }
    }

    /// Set structured data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Trait for implementing skills.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Get the skill's name.
    fn name(&self) -> &str;

    /// Get the skill's configuration.
    fn config(&self) -> &SkillConfig;

    /// Execute the skill with the given input.
    async fn execute(&self, client: &AsyncForgeClient, input: SkillInput)
        -> SkillResult<SkillOutput>;

    /// Validate input before execution.
    fn validate_input(&self, _input: &SkillInput) -> SkillResult<()> {
        Ok(())
    }
}

/// A prompt-based skill that uses an LLM.
pub struct PromptSkill {
    config: SkillConfig,
}

impl PromptSkill {
    /// Create a new prompt skill.
    pub fn new(config: SkillConfig) -> Self {
        Self { config }
    }

    /// Create a simple prompt skill.
    pub fn simple(
        name: impl Into<String>,
        description: impl Into<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            config: SkillConfig::new(name, description).with_system_prompt(system_prompt),
        }
    }
}

#[async_trait]
impl Skill for PromptSkill {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &SkillConfig {
        &self.config
    }

    async fn execute(
        &self,
        client: &AsyncForgeClient,
        input: SkillInput,
    ) -> SkillResult<SkillOutput> {
        let mut messages = Vec::new();

        // Add system prompt
        if let Some(ref prompt) = self.config.system_prompt {
            // Substitute any parameters in the prompt
            let mut processed_prompt = prompt.clone();
            for (key, value) in &input.params {
                let placeholder = format!("{{{}}}", key);
                if let Some(val_str) = value.as_str() {
                    processed_prompt = processed_prompt.replace(&placeholder, val_str);
                }
            }
            messages.push(Message::system(&processed_prompt));
        }

        // Add user message
        messages.push(Message::user(&input.text));

        // Build request
        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| client.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);

        if let Some(temp) = self.config.temperature {
            request = request.temperature(temp);
        }

        if let Some(max_tokens) = self.config.max_tokens {
            request = request.max_tokens(max_tokens);
        }

        // Execute
        let response = client.chat_completions(request).await?;

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(SkillOutput::new(text))
    }
}

/// Registry for managing skills.
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// Create a new skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill.
    pub fn register<S: Skill + 'static>(&mut self, skill: S) {
        self.skills
            .insert(skill.name().to_string(), Arc::new(skill));
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(name).cloned()
    }

    /// Check if a skill is registered.
    pub fn has(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Get skill count.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Remove a skill.
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.remove(name)
    }

    /// Execute a skill by name.
    pub async fn execute(
        &self,
        name: &str,
        client: &AsyncForgeClient,
        input: impl Into<SkillInput>,
    ) -> SkillResult<SkillOutput> {
        let skill = self
            .get(name)
            .ok_or_else(|| SkillError::NotFound(name.to_string()))?;

        let input = input.into();
        skill.validate_input(&input)?;
        skill.execute(client, input).await
    }
}

// Built-in skills

/// Summarization skill.
pub fn summarize_skill() -> PromptSkill {
    PromptSkill::simple(
        "summarize",
        "Summarize text into a concise form",
        "You are a summarization assistant. Summarize the following text concisely while preserving the key points and main ideas. Be clear and direct.",
    )
}

/// Translation skill.
pub fn translate_skill() -> PromptSkill {
    PromptSkill::new(
        SkillConfig::new("translate", "Translate text between languages")
            .with_system_prompt(
                "You are a translation assistant. Translate the following text to {target_language}. Preserve the meaning, tone, and style of the original text.",
            ),
    )
}

/// Extraction skill.
pub fn extract_skill() -> PromptSkill {
    PromptSkill::new(
        SkillConfig::new("extract", "Extract structured information from text")
            .with_system_prompt(
                "You are an information extraction assistant. Extract the requested information from the following text. Format the output as JSON.",
            ),
    )
}

/// Rewriting skill.
pub fn rewrite_skill() -> PromptSkill {
    PromptSkill::new(
        SkillConfig::new("rewrite", "Rewrite text in a different style or tone")
            .with_system_prompt(
                "You are a writing assistant. Rewrite the following text according to the specified style: {style}. Maintain the core meaning while adjusting the tone and presentation.",
            ),
    )
}

/// Question answering skill.
pub fn qa_skill() -> PromptSkill {
    PromptSkill::new(
        SkillConfig::new("qa", "Answer questions based on provided context")
            .with_system_prompt(
                "You are a question answering assistant. Based on the provided context, answer the user's question accurately and concisely. If the answer cannot be determined from the context, say so.",
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_config() {
        let config = SkillConfig::new("test", "A test skill")
            .with_model("gpt-4")
            .with_temperature(0.7)
            .with_tag("test");

        assert_eq!(config.name, "test");
        assert_eq!(config.description, "A test skill");
        assert_eq!(config.model, Some("gpt-4".to_string()));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.tags, vec!["test"]);
    }

    #[test]
    fn test_skill_input() {
        let input = SkillInput::new("Hello world")
            .with_param("lang", serde_json::json!("en"))
            .with_context(serde_json::json!({"previous": "data"}));

        assert_eq!(input.text, "Hello world");
        assert_eq!(input.params.get("lang"), Some(&serde_json::json!("en")));
        assert!(input.context.is_some());
    }

    #[test]
    fn test_skill_output() {
        let output = SkillOutput::new("Result")
            .with_data(serde_json::json!({"key": "value"}))
            .with_metadata("tokens", serde_json::json!(100));

        assert_eq!(output.text, "Result");
        assert!(output.data.is_some());
        assert_eq!(output.metadata.get("tokens"), Some(&serde_json::json!(100)));
    }

    #[test]
    fn test_skill_registry() {
        let mut registry = SkillRegistry::new();

        registry.register(summarize_skill());
        registry.register(translate_skill());

        assert_eq!(registry.len(), 2);
        assert!(registry.has("summarize"));
        assert!(registry.has("translate"));
        assert!(!registry.has("nonexistent"));

        let skills = registry.list();
        assert!(skills.contains(&"summarize"));
        assert!(skills.contains(&"translate"));
    }

    #[test]
    fn test_prompt_skill_creation() {
        let skill = PromptSkill::simple(
            "my-skill",
            "My custom skill",
            "You are a helpful assistant.",
        );

        assert_eq!(skill.name(), "my-skill");
        assert_eq!(skill.config().description, "My custom skill");
        assert_eq!(
            skill.config().system_prompt,
            Some("You are a helpful assistant.".to_string())
        );
    }

    #[test]
    fn test_builtin_skills() {
        let summarize = summarize_skill();
        assert_eq!(summarize.name(), "summarize");

        let translate = translate_skill();
        assert_eq!(translate.name(), "translate");

        let extract = extract_skill();
        assert_eq!(extract.name(), "extract");

        let rewrite = rewrite_skill();
        assert_eq!(rewrite.name(), "rewrite");

        let qa = qa_skill();
        assert_eq!(qa.name(), "qa");
    }

    #[test]
    fn test_skill_input_from_str() {
        let input: SkillInput = "Hello".into();
        assert_eq!(input.text, "Hello");

        let input: SkillInput = String::from("World").into();
        assert_eq!(input.text, "World");
    }
}
