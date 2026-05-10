//! Test case types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A test case for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Test case name/ID.
    pub name: Option<String>,
    /// Input to the model/agent.
    pub input: String,
    /// Expected output.
    pub expected: String,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional context/metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Weight for scoring (default 1.0).
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

impl TestCase {
    /// Create a new test case.
    pub fn new(input: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            name: None,
            input: input.into(),
            expected: expected.into(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            weight: 1.0,
        }
    }

    /// Create a builder for a test case.
    pub fn builder(input: impl Into<String>, expected: impl Into<String>) -> TestCaseBuilder {
        TestCaseBuilder::new(input, expected)
    }

    /// Set the test case name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the weight.
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Check if the test case has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get the display name (name or truncated input).
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            let truncated = if self.input.len() > 30 {
                format!("{}...", &self.input[..30])
            } else {
                self.input.clone()
            };
            truncated
        })
    }
}

/// Builder for TestCase.
#[derive(Debug)]
pub struct TestCaseBuilder {
    input: String,
    expected: String,
    name: Option<String>,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
    weight: f64,
}

impl TestCaseBuilder {
    /// Create a new builder.
    pub fn new(input: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            expected: expected.into(),
            name: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            weight: 1.0,
        }
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the weight.
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Build the test case.
    pub fn build(self) -> TestCase {
        TestCase {
            name: self.name,
            input: self.input,
            expected: self.expected,
            tags: self.tags,
            metadata: self.metadata,
            weight: self.weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_case_new() {
        let tc = TestCase::new("input", "expected");
        assert_eq!(tc.input, "input");
        assert_eq!(tc.expected, "expected");
        assert!(tc.name.is_none());
        assert_eq!(tc.weight, 1.0);
    }

    #[test]
    fn test_test_case_with_name() {
        let tc = TestCase::new("input", "expected").name("my_test");
        assert_eq!(tc.name, Some("my_test".to_string()));
    }

    #[test]
    fn test_test_case_with_tags() {
        let tc = TestCase::new("input", "expected").tag("math").tag("basic");

        assert!(tc.has_tag("math"));
        assert!(tc.has_tag("basic"));
        assert!(!tc.has_tag("advanced"));
    }

    #[test]
    fn test_test_case_builder() {
        let tc = TestCase::builder("input", "expected")
            .name("test1")
            .tag("category1")
            .metadata("author", "test")
            .weight(2.0)
            .build();

        assert_eq!(tc.name, Some("test1".to_string()));
        assert!(tc.has_tag("category1"));
        assert_eq!(tc.metadata.get("author"), Some(&"test".to_string()));
        assert_eq!(tc.weight, 2.0);
    }

    #[test]
    fn test_display_name() {
        let tc1 = TestCase::new("short", "expected").name("named_test");
        assert_eq!(tc1.display_name(), "named_test");

        let tc2 = TestCase::new("short input", "expected");
        assert_eq!(tc2.display_name(), "short input");

        let tc3 = TestCase::new(
            "this is a very long input that should be truncated",
            "expected",
        );
        assert!(tc3.display_name().ends_with("..."));
        assert!(tc3.display_name().len() <= 33);
    }

    #[test]
    fn test_builder_multiple_tags() {
        let tc = TestCase::builder("in", "out")
            .tags(vec!["a", "b", "c"])
            .build();

        assert_eq!(tc.tags.len(), 3);
        assert!(tc.has_tag("a"));
        assert!(tc.has_tag("b"));
        assert!(tc.has_tag("c"));
    }
}
