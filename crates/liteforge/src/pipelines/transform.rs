//! Model transformations for pipelines.

use super::{PipelineContext, PipelineError, PipelineResult};
use std::sync::Arc;

/// Trait for model input/output transformations.
pub trait ModelTransform: Send + Sync {
    /// Get the transform name.
    fn name(&self) -> &str;

    /// Transform input before sending to model.
    fn transform_input(&self, input: &str, context: &PipelineContext) -> PipelineResult<String>;

    /// Transform output after receiving from model.
    fn transform_output(&self, output: &str, context: &PipelineContext) -> PipelineResult<String>;
}

/// Identity transform (no changes).
#[allow(dead_code)]
pub struct IdentityTransform;

impl ModelTransform for IdentityTransform {
    fn name(&self) -> &str {
        "identity"
    }

    fn transform_input(&self, input: &str, _context: &PipelineContext) -> PipelineResult<String> {
        Ok(input.to_string())
    }

    fn transform_output(&self, output: &str, _context: &PipelineContext) -> PipelineResult<String> {
        Ok(output.to_string())
    }
}

/// Transform that prepends/appends text.
#[allow(dead_code)]
pub struct WrapTransform {
    name: String,
    input_prefix: String,
    input_suffix: String,
    output_prefix: String,
    output_suffix: String,
}

#[allow(dead_code)]
impl WrapTransform {
    /// Create a new wrap transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            input_prefix: String::new(),
            input_suffix: String::new(),
            output_prefix: String::new(),
            output_suffix: String::new(),
        }
    }

    /// Set input prefix.
    pub fn with_input_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.input_prefix = prefix.into();
        self
    }

    /// Set input suffix.
    pub fn with_input_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.input_suffix = suffix.into();
        self
    }

    /// Set output prefix.
    pub fn with_output_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.output_prefix = prefix.into();
        self
    }

    /// Set output suffix.
    pub fn with_output_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.output_suffix = suffix.into();
        self
    }
}

impl ModelTransform for WrapTransform {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_input(&self, input: &str, _context: &PipelineContext) -> PipelineResult<String> {
        Ok(format!(
            "{}{}{}",
            self.input_prefix, input, self.input_suffix
        ))
    }

    fn transform_output(&self, output: &str, _context: &PipelineContext) -> PipelineResult<String> {
        Ok(format!(
            "{}{}{}",
            self.output_prefix, output, self.output_suffix
        ))
    }
}

/// Transform that applies regex replacements.
#[allow(dead_code)]
pub struct RegexTransform {
    name: String,
    input_patterns: Vec<(regex::Regex, String)>,
    output_patterns: Vec<(regex::Regex, String)>,
}

#[allow(dead_code)]
impl RegexTransform {
    /// Create a new regex transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            input_patterns: Vec::new(),
            output_patterns: Vec::new(),
        }
    }

    /// Add an input replacement pattern.
    pub fn with_input_replace(
        mut self,
        pattern: &str,
        replacement: impl Into<String>,
    ) -> PipelineResult<Self> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| PipelineError::TransformError(format!("Invalid regex: {}", e)))?;
        self.input_patterns.push((regex, replacement.into()));
        Ok(self)
    }

    /// Add an output replacement pattern.
    pub fn with_output_replace(
        mut self,
        pattern: &str,
        replacement: impl Into<String>,
    ) -> PipelineResult<Self> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| PipelineError::TransformError(format!("Invalid regex: {}", e)))?;
        self.output_patterns.push((regex, replacement.into()));
        Ok(self)
    }
}

impl ModelTransform for RegexTransform {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_input(&self, input: &str, _context: &PipelineContext) -> PipelineResult<String> {
        let mut result = input.to_string();
        for (pattern, replacement) in &self.input_patterns {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }
        Ok(result)
    }

    fn transform_output(&self, output: &str, _context: &PipelineContext) -> PipelineResult<String> {
        let mut result = output.to_string();
        for (pattern, replacement) in &self.output_patterns {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }
        Ok(result)
    }
}

/// Transform that extracts JSON from output.
#[allow(dead_code)]
pub struct JsonExtractTransform {
    name: String,
}

#[allow(dead_code)]
impl JsonExtractTransform {
    /// Create a new JSON extract transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl ModelTransform for JsonExtractTransform {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_input(&self, input: &str, _context: &PipelineContext) -> PipelineResult<String> {
        Ok(input.to_string())
    }

    fn transform_output(&self, output: &str, _context: &PipelineContext) -> PipelineResult<String> {
        // Try to find JSON in the output
        // Look for content between ```json and ``` or just {} or []

        // Check for markdown code blocks
        if let Some(start) = output.find("```json") {
            if let Some(end) = output[start + 7..].find("```") {
                let json_str = output[start + 7..start + 7 + end].trim();
                // Validate it's valid JSON
                if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                    return Ok(json_str.to_string());
                }
            }
        }

        // Check for plain code blocks
        if let Some(start) = output.find("```") {
            let after_start = start + 3;
            // Skip any language identifier
            let content_start = output[after_start..]
                .find('\n')
                .map(|i| after_start + i + 1)
                .unwrap_or(after_start);

            if let Some(end) = output[content_start..].find("```") {
                let json_str = output[content_start..content_start + end].trim();
                if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                    return Ok(json_str.to_string());
                }
            }
        }

        // Try to find raw JSON object or array
        for (start_char, end_char) in [("{", "}"), ("[", "]")] {
            if let Some(start) = output.find(start_char) {
                // Find matching bracket
                let mut depth = 0;
                let mut end = None;
                for (i, c) in output[start..].char_indices() {
                    if c.to_string() == start_char {
                        depth += 1;
                    } else if c.to_string() == end_char {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(start + i + 1);
                            break;
                        }
                    }
                }

                if let Some(end_pos) = end {
                    let json_str = &output[start..end_pos];
                    if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
                        return Ok(json_str.to_string());
                    }
                }
            }
        }

        // Return original if no JSON found
        Ok(output.to_string())
    }
}

/// A chain of transforms applied in sequence.
pub struct TransformChain {
    name: String,
    transforms: Vec<Arc<dyn ModelTransform>>,
}

impl TransformChain {
    /// Create a new transform chain.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transforms: Vec::new(),
        }
    }

    /// Add a transform to the chain.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, transform: impl ModelTransform + 'static) -> Self {
        self.transforms.push(Arc::new(transform));
        self
    }

    /// Get the number of transforms in the chain.
    pub fn len(&self) -> usize {
        self.transforms.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.transforms.is_empty()
    }
}

impl ModelTransform for TransformChain {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_input(&self, input: &str, context: &PipelineContext) -> PipelineResult<String> {
        let mut result = input.to_string();
        for transform in &self.transforms {
            result = transform.transform_input(&result, context)?;
        }
        Ok(result)
    }

    fn transform_output(&self, output: &str, context: &PipelineContext) -> PipelineResult<String> {
        let mut result = output.to_string();
        // Apply in reverse order for output
        for transform in self.transforms.iter().rev() {
            result = transform.transform_output(&result, context)?;
        }
        Ok(result)
    }
}

/// Type alias for transform functions to reduce complexity.
type TransformFn = Box<dyn Fn(&str, &PipelineContext) -> PipelineResult<String> + Send + Sync>;

/// Custom transform using closures.
#[allow(dead_code)]
pub struct CustomTransform {
    name: String,
    input_fn: TransformFn,
    output_fn: TransformFn,
}

#[allow(dead_code)]
impl CustomTransform {
    /// Create a new custom transform.
    pub fn new<I, O>(name: impl Into<String>, input_fn: I, output_fn: O) -> Self
    where
        I: Fn(&str, &PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
        O: Fn(&str, &PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            input_fn: Box::new(input_fn),
            output_fn: Box::new(output_fn),
        }
    }

    /// Create a transform that only modifies input.
    pub fn input_only<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&str, &PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
    {
        Self::new(name, f, |s, _| Ok(s.to_string()))
    }

    /// Create a transform that only modifies output.
    pub fn output_only<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&str, &PipelineContext) -> PipelineResult<String> + Send + Sync + 'static,
    {
        Self::new(name, |s, _| Ok(s.to_string()), f)
    }
}

impl ModelTransform for CustomTransform {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform_input(&self, input: &str, context: &PipelineContext) -> PipelineResult<String> {
        (self.input_fn)(input, context)
    }

    fn transform_output(&self, output: &str, context: &PipelineContext) -> PipelineResult<String> {
        (self.output_fn)(output, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform() {
        let transform = IdentityTransform;
        let ctx = PipelineContext::new();

        assert_eq!(transform.name(), "identity");
        assert_eq!(transform.transform_input("hello", &ctx).unwrap(), "hello");
        assert_eq!(transform.transform_output("world", &ctx).unwrap(), "world");
    }

    #[test]
    fn test_wrap_transform() {
        let transform = WrapTransform::new("wrapper")
            .with_input_prefix("<<")
            .with_input_suffix(">>")
            .with_output_prefix("[")
            .with_output_suffix("]");

        let ctx = PipelineContext::new();

        assert_eq!(transform.transform_input("text", &ctx).unwrap(), "<<text>>");
        assert_eq!(
            transform.transform_output("result", &ctx).unwrap(),
            "[result]"
        );
    }

    #[test]
    fn test_regex_transform() {
        let transform = RegexTransform::new("regex")
            .with_input_replace(r"\bfoo\b", "bar")
            .unwrap()
            .with_output_replace(r"\d+", "NUMBER")
            .unwrap();

        let ctx = PipelineContext::new();

        assert_eq!(
            transform.transform_input("foo is foo", &ctx).unwrap(),
            "bar is bar"
        );
        assert_eq!(
            transform.transform_output("count: 42", &ctx).unwrap(),
            "count: NUMBER"
        );
    }

    #[test]
    fn test_json_extract_transform() {
        let transform = JsonExtractTransform::new("json");
        let ctx = PipelineContext::new();

        // Test markdown code block
        let output = r#"Here's the JSON:
```json
{"key": "value"}
```
Done!"#;
        assert_eq!(
            transform.transform_output(output, &ctx).unwrap(),
            r#"{"key": "value"}"#
        );

        // Test raw JSON
        let output = r#"The result is {"a": 1, "b": 2} which is good."#;
        assert_eq!(
            transform.transform_output(output, &ctx).unwrap(),
            r#"{"a": 1, "b": 2}"#
        );

        // Test no JSON
        let output = "Just plain text";
        assert_eq!(
            transform.transform_output(output, &ctx).unwrap(),
            "Just plain text"
        );
    }

    #[test]
    fn test_transform_chain() {
        let chain = TransformChain::new("chain")
            .add(WrapTransform::new("wrap1").with_input_prefix("A"))
            .add(WrapTransform::new("wrap2").with_input_prefix("B"));

        let ctx = PipelineContext::new();

        // Transforms applied in order: A then B
        assert_eq!(chain.transform_input("X", &ctx).unwrap(), "BAX");
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_custom_transform() {
        let transform = CustomTransform::new(
            "custom",
            |s, _| Ok(s.to_uppercase()),
            |s, _| Ok(s.to_lowercase()),
        );

        let ctx = PipelineContext::new();

        assert_eq!(transform.name(), "custom");
        assert_eq!(transform.transform_input("Hello", &ctx).unwrap(), "HELLO");
        assert_eq!(transform.transform_output("WORLD", &ctx).unwrap(), "world");
    }

    #[test]
    fn test_custom_transform_input_only() {
        let transform = CustomTransform::input_only("input-only", |s, _| Ok(format!("[{}]", s)));

        let ctx = PipelineContext::new();

        assert_eq!(transform.transform_input("test", &ctx).unwrap(), "[test]");
        assert_eq!(transform.transform_output("test", &ctx).unwrap(), "test");
    }
}
