//! Prompt templates and management.
//!
//! This module provides template-based prompt construction with
//! variable substitution and a library for managing reusable prompts.

mod template;

pub use template::{PromptTemplate, TemplateError, TemplateResult};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A library for storing and retrieving prompt templates.
#[derive(Debug, Default)]
pub struct PromptLibrary {
    templates: HashMap<String, PromptTemplate>,
    categories: HashMap<String, Vec<String>>,
}

impl PromptLibrary {
    /// Create a new empty prompt library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a template to the library.
    pub fn add(&mut self, name: impl Into<String>, template: PromptTemplate) {
        let name = name.into();
        self.templates.insert(name, template);
    }

    /// Add a template with a category.
    pub fn add_with_category(
        &mut self,
        name: impl Into<String>,
        template: PromptTemplate,
        category: impl Into<String>,
    ) {
        let name = name.into();
        let category = category.into();

        self.templates.insert(name.clone(), template);
        self.categories.entry(category).or_default().push(name);
    }

    /// Get a template by name.
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Remove a template.
    pub fn remove(&mut self, name: &str) -> Option<PromptTemplate> {
        // Remove from categories too
        for templates in self.categories.values_mut() {
            templates.retain(|n| n != name);
        }
        self.templates.remove(name)
    }

    /// List all template names.
    pub fn list(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// List templates in a category.
    pub fn list_by_category(&self, category: &str) -> Vec<&str> {
        self.categories
            .get(category)
            .map(|names| names.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// List all categories.
    pub fn categories(&self) -> Vec<&str> {
        self.categories.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a template exists.
    pub fn has(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get the number of templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Render a template by name with variables.
    pub fn render(
        &self,
        name: &str,
        variables: &HashMap<String, String>,
    ) -> TemplateResult<String> {
        let template = self
            .get(name)
            .ok_or_else(|| TemplateError::NotFound(name.to_string()))?;
        template.render(variables)
    }
}

/// Configuration for a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Template name.
    pub name: String,
    /// Template description.
    #[serde(default)]
    pub description: String,
    /// The template string.
    pub template: String,
    /// Default values for variables.
    #[serde(default)]
    pub defaults: HashMap<String, String>,
    /// Category for organization.
    #[serde(default)]
    pub category: Option<String>,
    /// Tags for searchability.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl PromptConfig {
    /// Create a new prompt config.
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            template: template.into(),
            defaults: HashMap::new(),
            category: None,
            tags: Vec::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a default value.
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Convert to a PromptTemplate.
    pub fn into_template(self) -> PromptTemplate {
        let mut template = PromptTemplate::new(&self.template);
        for (key, value) in self.defaults {
            template = template.with_default(key, value);
        }
        template
    }
}

/// Builder for creating prompts with sections.
#[derive(Debug, Default)]
pub struct PromptBuilder {
    sections: Vec<(String, String)>,
    variables: HashMap<String, String>,
}

impl PromptBuilder {
    /// Create a new prompt builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a section to the prompt.
    pub fn section(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.sections.push((name.into(), content.into()));
        self
    }

    /// Add a variable value.
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Add multiple variable values.
    pub fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (k, v) in vars {
            self.variables.insert(k.into(), v.into());
        }
        self
    }

    /// Build the final prompt string.
    pub fn build(self) -> String {
        let mut result = String::new();

        for (name, content) in self.sections {
            if !result.is_empty() {
                result.push_str("\n\n");
            }

            // Apply variable substitution
            let mut processed = content;
            for (key, value) in &self.variables {
                let placeholder = format!("{{{{{}}}}}", key);
                processed = processed.replace(&placeholder, value);
                // Also support simple {key} format
                let simple_placeholder = format!("{{{}}}", key);
                processed = processed.replace(&simple_placeholder, value);
            }

            // Add section header if name is not empty
            if !name.is_empty() {
                result.push_str(&format!("## {}\n\n", name));
            }
            result.push_str(&processed);
        }

        result
    }
}

/// Common prompt templates for typical use cases.
pub struct CommonPrompts;

impl CommonPrompts {
    /// Get a summarization prompt template.
    pub fn summarize() -> PromptTemplate {
        PromptTemplate::new(
            "Summarize the following text in {{style}} style:\n\n{{text}}\n\nSummary:",
        )
        .with_default("style", "concise")
    }

    /// Get a translation prompt template.
    pub fn translate() -> PromptTemplate {
        PromptTemplate::new(
            "Translate the following text to {{target_language}}:\n\n{{text}}\n\nTranslation:",
        )
    }

    /// Get a question answering prompt template.
    pub fn qa() -> PromptTemplate {
        PromptTemplate::new(
            "Context:\n{{context}}\n\nQuestion: {{question}}\n\nAnswer based only on the context provided:"
        )
    }

    /// Get a code review prompt template.
    pub fn code_review() -> PromptTemplate {
        PromptTemplate::new(
            "Review the following {{language}} code for issues, improvements, and best practices:\n\n```{{language}}\n{{code}}\n```\n\nProvide a detailed review:"
        )
        .with_default("language", "")
    }

    /// Get a text classification prompt template.
    pub fn classify() -> PromptTemplate {
        PromptTemplate::new(
            "Classify the following text into one of these categories: {{categories}}\n\nText: {{text}}\n\nCategory:"
        )
    }

    /// Get an entity extraction prompt template.
    pub fn extract_entities() -> PromptTemplate {
        PromptTemplate::new(
            "Extract all {{entity_type}} entities from the following text:\n\n{{text}}\n\nEntities (as JSON array):"
        )
        .with_default("entity_type", "named")
    }

    /// Get a rewriting prompt template.
    pub fn rewrite() -> PromptTemplate {
        PromptTemplate::new(
            "Rewrite the following text in {{tone}} tone:\n\n{{text}}\n\nRewritten:",
        )
        .with_default("tone", "professional")
    }

    /// Get a chain-of-thought reasoning prompt template.
    pub fn chain_of_thought() -> PromptTemplate {
        PromptTemplate::new("{{question}}\n\nLet's think step by step:")
    }

    /// Create a library with all common prompts.
    pub fn library() -> PromptLibrary {
        let mut lib = PromptLibrary::new();

        lib.add_with_category("summarize", Self::summarize(), "text");
        lib.add_with_category("translate", Self::translate(), "text");
        lib.add_with_category("qa", Self::qa(), "qa");
        lib.add_with_category("code_review", Self::code_review(), "code");
        lib.add_with_category("classify", Self::classify(), "classification");
        lib.add_with_category("extract_entities", Self::extract_entities(), "extraction");
        lib.add_with_category("rewrite", Self::rewrite(), "text");
        lib.add_with_category("chain_of_thought", Self::chain_of_thought(), "reasoning");

        lib
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_library() {
        let mut lib = PromptLibrary::new();

        lib.add("test", PromptTemplate::new("Hello {{name}}"));

        assert!(lib.has("test"));
        assert!(!lib.has("nonexistent"));
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn test_prompt_library_categories() {
        let mut lib = PromptLibrary::new();

        lib.add_with_category("t1", PromptTemplate::new("Template 1"), "cat1");
        lib.add_with_category("t2", PromptTemplate::new("Template 2"), "cat1");
        lib.add_with_category("t3", PromptTemplate::new("Template 3"), "cat2");

        assert_eq!(lib.list_by_category("cat1").len(), 2);
        assert_eq!(lib.list_by_category("cat2").len(), 1);
        assert_eq!(lib.categories().len(), 2);
    }

    #[test]
    fn test_prompt_library_render() {
        let mut lib = PromptLibrary::new();
        lib.add("greet", PromptTemplate::new("Hello, {{name}}!"));

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = lib.render("greet", &vars).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_prompt_config() {
        let config = PromptConfig::new("test", "Hello {{name}}")
            .with_description("A test template")
            .with_default("name", "World")
            .with_category("greetings")
            .with_tag("simple");

        assert_eq!(config.name, "test");
        assert_eq!(config.description, "A test template");
        assert_eq!(config.defaults.get("name"), Some(&"World".to_string()));
        assert_eq!(config.category, Some("greetings".to_string()));
        assert_eq!(config.tags, vec!["simple"]);
    }

    #[test]
    fn test_prompt_config_to_template() {
        let config = PromptConfig::new("test", "Hello {{name}}").with_default("name", "World");

        let template = config.into_template();
        let result = template.render(&HashMap::new()).unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .section("Context", "You are a helpful assistant.")
            .section("Task", "Answer the question: {{question}}")
            .var("question", "What is 2+2?")
            .build();

        assert!(prompt.contains("## Context"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("## Task"));
        assert!(prompt.contains("What is 2+2?"));
    }

    #[test]
    fn test_prompt_builder_no_header() {
        let prompt = PromptBuilder::new()
            .section("", "Just some content")
            .build();

        assert!(!prompt.contains("##"));
        assert!(prompt.contains("Just some content"));
    }

    #[test]
    fn test_common_prompts() {
        let summarize = CommonPrompts::summarize();
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "Long text here".to_string());

        let result = summarize.render(&vars).unwrap();
        assert!(result.contains("Long text here"));
        assert!(result.contains("concise")); // Default value

        let translate = CommonPrompts::translate();
        vars.insert("target_language".to_string(), "Spanish".to_string());
        let result = translate.render(&vars).unwrap();
        assert!(result.contains("Spanish"));
    }

    #[test]
    fn test_common_prompts_library() {
        let lib = CommonPrompts::library();

        assert!(lib.has("summarize"));
        assert!(lib.has("translate"));
        assert!(lib.has("qa"));
        assert!(lib.has("code_review"));
        assert!(lib.len() >= 8);
    }

    #[test]
    fn test_library_remove() {
        let mut lib = PromptLibrary::new();
        lib.add_with_category("test", PromptTemplate::new("Hello"), "cat1");

        assert!(lib.has("test"));
        assert_eq!(lib.list_by_category("cat1").len(), 1);

        lib.remove("test");

        assert!(!lib.has("test"));
        assert_eq!(lib.list_by_category("cat1").len(), 0);
    }
}
