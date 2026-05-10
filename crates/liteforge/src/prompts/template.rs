//! Prompt template engine with variable substitution.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error type for template operations.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Template not found.
    #[error("Template not found: {0}")]
    NotFound(String),

    /// Missing required variable.
    #[error("Missing required variable: {0}")]
    MissingVariable(String),

    /// Invalid template syntax.
    #[error("Invalid template syntax: {0}")]
    InvalidSyntax(String),

    /// Template rendering error.
    #[error("Render error: {0}")]
    RenderError(String),
}

/// Result type for template operations.
pub type TemplateResult<T> = Result<T, TemplateError>;

/// A prompt template with variable substitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// The raw template string.
    template: String,
    /// Default values for variables.
    #[serde(default)]
    defaults: HashMap<String, String>,
    /// Whether to fail on missing variables.
    #[serde(default = "default_strict")]
    strict: bool,
}

fn default_strict() -> bool {
    false
}

impl PromptTemplate {
    /// Create a new template.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            defaults: HashMap::new(),
            strict: false,
        }
    }

    /// Set a default value for a variable.
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Set multiple default values.
    pub fn with_defaults(mut self, defaults: HashMap<String, String>) -> Self {
        self.defaults.extend(defaults);
        self
    }

    /// Set strict mode (fail on missing variables).
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Get the raw template string.
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Get the default values.
    pub fn defaults(&self) -> &HashMap<String, String> {
        &self.defaults
    }

    /// Extract variable names from the template.
    pub fn variables(&self) -> Vec<String> {
        let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        re.captures_iter(&self.template)
            .map(|cap| cap[1].trim().to_string())
            .collect()
    }

    /// Check if all required variables are provided.
    pub fn validate(&self, variables: &HashMap<String, String>) -> TemplateResult<()> {
        let required = self.variables();
        for var in required {
            // Check for filters
            let var_name = var.split('|').next().unwrap_or(&var).trim();
            if !variables.contains_key(var_name) && !self.defaults.contains_key(var_name) {
                return Err(TemplateError::MissingVariable(var_name.to_string()));
            }
        }
        Ok(())
    }

    /// Render the template with the given variables.
    pub fn render(&self, variables: &HashMap<String, String>) -> TemplateResult<String> {
        if self.strict {
            self.validate(variables)?;
        }

        let mut result = self.template.clone();

        // First pass: handle {{ var | filter }} syntax
        let filter_re = Regex::new(r"\{\{\s*([^|}\s]+)\s*\|\s*([^}]+)\s*\}\}").unwrap();
        result = filter_re
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = caps[1].trim();
                let filter = caps[2].trim();

                let value = variables
                    .get(var_name)
                    .or_else(|| self.defaults.get(var_name))
                    .cloned()
                    .unwrap_or_else(|| format!("{{{{{}}}}}", var_name));

                apply_filter(&value, filter)
            })
            .to_string();

        // Second pass: handle {{ var }} syntax (no filters)
        let simple_re = Regex::new(r"\{\{\s*([^|}]+?)\s*\}\}").unwrap();
        result = simple_re
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = caps[1].trim();

                variables
                    .get(var_name)
                    .or_else(|| self.defaults.get(var_name))
                    .cloned()
                    .unwrap_or_else(|| {
                        if self.strict {
                            format!("{{{{ {} }}}}", var_name)
                        } else {
                            String::new()
                        }
                    })
            })
            .to_string();

        Ok(result)
    }

    /// Render with a builder-style interface.
    pub fn render_with(&self) -> RenderBuilder<'_> {
        RenderBuilder::new(self)
    }
}

/// Apply a filter to a value.
fn apply_filter(value: &str, filter: &str) -> String {
    match filter.to_lowercase().as_str() {
        "upper" | "uppercase" => value.to_uppercase(),
        "lower" | "lowercase" => value.to_lowercase(),
        "capitalize" => {
            let mut chars = value.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
        "title" => value
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => {
                        f.to_uppercase().collect::<String>()
                            + chars.as_str().to_lowercase().as_str()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        "trim" => value.trim().to_string(),
        "strip" => value.trim().to_string(),
        "length" => value.len().to_string(),
        "reverse" => value.chars().rev().collect(),
        "first" => value
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_default(),
        "last" => value
            .chars()
            .last()
            .map(|c| c.to_string())
            .unwrap_or_default(),
        "wordcount" => value.split_whitespace().count().to_string(),
        "default" => {
            // Handle default(value) syntax
            if filter.starts_with("default(") && filter.ends_with(")") {
                let default_val = &filter[8..filter.len() - 1];
                if value.is_empty() {
                    default_val.trim_matches('"').trim_matches('\'').to_string()
                } else {
                    value.to_string()
                }
            } else {
                value.to_string()
            }
        }
        _ => {
            // Handle truncate(n) syntax
            if filter.starts_with("truncate(") && filter.ends_with(")") {
                if let Ok(n) = filter[9..filter.len() - 1].trim().parse::<usize>() {
                    if value.len() > n {
                        return format!("{}...", &value[..n]);
                    }
                }
            }
            // Handle replace(old, new) syntax
            if filter.starts_with("replace(") && filter.ends_with(")") {
                let args = &filter[8..filter.len() - 1];
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() == 2 {
                    let old = parts[0].trim().trim_matches('"').trim_matches('\'');
                    let new = parts[1].trim().trim_matches('"').trim_matches('\'');
                    return value.replace(old, new);
                }
            }
            value.to_string()
        }
    }
}

/// Builder for rendering templates.
pub struct RenderBuilder<'a> {
    template: &'a PromptTemplate,
    variables: HashMap<String, String>,
}

impl<'a> RenderBuilder<'a> {
    fn new(template: &'a PromptTemplate) -> Self {
        Self {
            template,
            variables: HashMap::new(),
        }
    }

    /// Set a variable value.
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Set multiple variable values.
    pub fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (k, v) in vars {
            self.variables.insert(k.into(), v.into());
        }
        self
    }

    /// Render the template.
    pub fn render(self) -> TemplateResult<String> {
        self.template.render(&self.variables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_template() {
        let template = PromptTemplate::new("Hello, {{name}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_template_with_defaults() {
        let template = PromptTemplate::new("Hello, {{name}}!").with_default("name", "World");

        let result = template.render(&HashMap::new()).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_template_override_defaults() {
        let template = PromptTemplate::new("Hello, {{name}}!").with_default("name", "World");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Rust".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello, Rust!");
    }

    #[test]
    fn test_template_variables() {
        let template = PromptTemplate::new("{{greeting}}, {{name}}! {{message}}");
        let vars = template.variables();

        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"greeting".to_string()));
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"message".to_string()));
    }

    #[test]
    fn test_strict_mode() {
        let template = PromptTemplate::new("Hello, {{name}}!").strict();

        let result = template.validate(&HashMap::new());
        assert!(result.is_err());

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        let result = template.validate(&vars);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_upper() {
        let template = PromptTemplate::new("Hello, {{name | upper}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello, WORLD!");
    }

    #[test]
    fn test_filter_lower() {
        let template = PromptTemplate::new("Hello, {{name | lower}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "WORLD".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_filter_capitalize() {
        let template = PromptTemplate::new("{{text | capitalize}}");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "hello world".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_filter_title() {
        let template = PromptTemplate::new("{{text | title}}");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "hello world".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_filter_trim() {
        let template = PromptTemplate::new("[{{text | trim}}]");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "  hello  ".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "[hello]");
    }

    #[test]
    fn test_filter_length() {
        let template = PromptTemplate::new("Length: {{text | length}}");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "hello".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Length: 5");
    }

    #[test]
    fn test_filter_truncate() {
        let template = PromptTemplate::new("{{text | truncate(5)}}");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "hello world".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_filter_replace() {
        let template = PromptTemplate::new("{{text | replace(\"world\", \"Rust\")}}");
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), "hello world".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "hello Rust");
    }

    #[test]
    fn test_render_builder() {
        let template = PromptTemplate::new("{{greeting}}, {{name}}!");

        let result = template
            .render_with()
            .var("greeting", "Hello")
            .var("name", "World")
            .render()
            .unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_builder_vars() {
        let template = PromptTemplate::new("{{a}} {{b}} {{c}}");

        let result = template
            .render_with()
            .vars([("a", "1"), ("b", "2"), ("c", "3")])
            .render()
            .unwrap();

        assert_eq!(result, "1 2 3");
    }

    #[test]
    fn test_multiple_same_variable() {
        let template = PromptTemplate::new("{{name}} says hello. {{name}} is happy.");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Alice says hello. Alice is happy.");
    }

    #[test]
    fn test_nested_braces() {
        let template = PromptTemplate::new("JSON: {\"key\": \"{{value}}\"}");
        let mut vars = HashMap::new();
        vars.insert("value".to_string(), "test".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "JSON: {\"key\": \"test\"}");
    }

    #[test]
    fn test_whitespace_handling() {
        let template = PromptTemplate::new("Hello, {{  name  }}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_complex_template() {
        let template = PromptTemplate::new(
            "Task: {{task | upper}}\nContext: {{context}}\nStyle: {{style | title}}",
        )
        .with_default("style", "professional");

        let mut vars = HashMap::new();
        vars.insert("task".to_string(), "summarize".to_string());
        vars.insert("context".to_string(), "Some text here".to_string());

        let result = template.render(&vars).unwrap();
        assert!(result.contains("SUMMARIZE"));
        assert!(result.contains("Some text here"));
        assert!(result.contains("Professional"));
    }
}
