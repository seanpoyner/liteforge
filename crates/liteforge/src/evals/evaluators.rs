//! Evaluator implementations.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of an evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Whether the evaluation passed.
    pub passed: bool,
    /// Score (0.0 to 1.0).
    pub score: f64,
    /// Reason/explanation.
    pub reason: Option<String>,
    /// Additional details.
    #[serde(default)]
    pub details: HashMap<String, String>,
}

impl EvalResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self {
            passed: true,
            score: 1.0,
            reason: None,
            details: HashMap::new(),
        }
    }

    /// Create a failing result.
    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            score: 0.0,
            reason: Some(reason.into()),
            details: HashMap::new(),
        }
    }

    /// Create a partial result with a score.
    pub fn partial(score: f64, reason: impl Into<String>) -> Self {
        Self {
            passed: score >= 0.5, // Consider passing if score >= 0.5
            score: score.clamp(0.0, 1.0),
            reason: Some(reason.into()),
            details: HashMap::new(),
        }
    }

    /// Set a custom passing threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.passed = self.score >= threshold;
        self
    }

    /// Add a detail.
    pub fn detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Trait for evaluating model outputs.
pub trait Evaluator: Send + Sync {
    /// Evaluate actual output against expected output.
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult;

    /// Get the evaluator name.
    fn name(&self) -> &str;
}

/// Exact match evaluator.
#[derive(Debug, Clone, Default)]
pub struct ExactMatchEvaluator {
    /// Whether to ignore case.
    ignore_case: bool,
    /// Whether to trim whitespace.
    trim: bool,
}

impl ExactMatchEvaluator {
    /// Create a new exact match evaluator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ignore case when comparing.
    pub fn ignore_case(mut self) -> Self {
        self.ignore_case = true;
        self
    }

    /// Trim whitespace when comparing.
    pub fn trim(mut self) -> Self {
        self.trim = true;
        self
    }
}

impl Evaluator for ExactMatchEvaluator {
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult {
        let mut actual = actual.to_string();
        let mut expected = expected.to_string();

        if self.trim {
            actual = actual.trim().to_string();
            expected = expected.trim().to_string();
        }

        if self.ignore_case {
            actual = actual.to_lowercase();
            expected = expected.to_lowercase();
        }

        if actual == expected {
            EvalResult::pass()
        } else {
            EvalResult::fail(format!("Expected '{}', got '{}'", expected, actual))
        }
    }

    fn name(&self) -> &str {
        "exact_match"
    }
}

/// Contains evaluator - checks if actual contains expected.
#[derive(Debug, Clone, Default)]
pub struct ContainsEvaluator {
    /// Whether to ignore case.
    ignore_case: bool,
}

impl ContainsEvaluator {
    /// Create a new contains evaluator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ignore case when comparing.
    pub fn ignore_case(mut self) -> Self {
        self.ignore_case = true;
        self
    }
}

impl Evaluator for ContainsEvaluator {
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult {
        let (actual, expected) = if self.ignore_case {
            (actual.to_lowercase(), expected.to_lowercase())
        } else {
            (actual.to_string(), expected.to_string())
        };

        if actual.contains(&expected) {
            EvalResult::pass()
        } else {
            EvalResult::fail(format!("Output does not contain '{}'", expected))
        }
    }

    fn name(&self) -> &str {
        "contains"
    }
}

/// Regex evaluator - checks if actual matches a regex pattern.
#[derive(Debug, Clone)]
pub struct RegexEvaluator {
    /// Whether to ignore case.
    ignore_case: bool,
}

impl Default for RegexEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexEvaluator {
    /// Create a new regex evaluator.
    pub fn new() -> Self {
        Self { ignore_case: false }
    }

    /// Ignore case when matching.
    pub fn ignore_case(mut self) -> Self {
        self.ignore_case = true;
        self
    }
}

impl Evaluator for RegexEvaluator {
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult {
        let pattern = if self.ignore_case {
            format!("(?i){}", expected)
        } else {
            expected.to_string()
        };

        match Regex::new(&pattern) {
            Ok(re) => {
                if re.is_match(actual) {
                    EvalResult::pass()
                } else {
                    EvalResult::fail(format!("Output does not match pattern '{}'", expected))
                }
            }
            Err(e) => EvalResult::fail(format!("Invalid regex pattern: {}", e)),
        }
    }

    fn name(&self) -> &str {
        "regex"
    }
}

/// JSON match evaluator - compares JSON values.
#[derive(Debug, Clone, Default)]
pub struct JsonMatchEvaluator {
    /// Whether to ignore extra fields in actual.
    ignore_extra: bool,
}

impl JsonMatchEvaluator {
    /// Create a new JSON match evaluator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ignore extra fields in actual output.
    pub fn ignore_extra(mut self) -> Self {
        self.ignore_extra = true;
        self
    }
}

impl Evaluator for JsonMatchEvaluator {
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult {
        let actual_json: Result<serde_json::Value, _> = serde_json::from_str(actual);
        let expected_json: Result<serde_json::Value, _> = serde_json::from_str(expected);

        match (actual_json, expected_json) {
            (Ok(actual_val), Ok(expected_val)) => {
                if self.ignore_extra {
                    if json_contains(&actual_val, &expected_val) {
                        EvalResult::pass()
                    } else {
                        EvalResult::fail("JSON does not contain expected structure")
                    }
                } else if actual_val == expected_val {
                    EvalResult::pass()
                } else {
                    EvalResult::fail("JSON values do not match")
                }
            }
            (Err(e), _) => EvalResult::fail(format!("Invalid actual JSON: {}", e)),
            (_, Err(e)) => EvalResult::fail(format!("Invalid expected JSON: {}", e)),
        }
    }

    fn name(&self) -> &str {
        "json_match"
    }
}

/// Check if actual JSON contains all fields from expected.
fn json_contains(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => e
            .iter()
            .all(|(k, v)| a.get(k).map(|av| json_contains(av, v)).unwrap_or(false)),
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) => {
            if a.len() != e.len() {
                return false;
            }
            a.iter().zip(e.iter()).all(|(av, ev)| json_contains(av, ev))
        }
        _ => actual == expected,
    }
}

/// Similarity evaluator using simple character-level comparison.
#[derive(Debug, Clone)]
pub struct SimilarityEvaluator {
    /// Minimum similarity threshold (0.0 to 1.0).
    threshold: f64,
}

impl Default for SimilarityEvaluator {
    fn default() -> Self {
        Self::new(0.8)
    }
}

impl SimilarityEvaluator {
    /// Create a new similarity evaluator with a threshold.
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Calculate Levenshtein distance.
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let m = s1_chars.len();
        let n = s2_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut dp = vec![vec![0; n + 1]; m + 1];

        for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate().take(n + 1) {
            *val = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[m][n]
    }

    /// Calculate similarity score (0.0 to 1.0).
    fn similarity_score(s1: &str, s2: &str) -> f64 {
        let max_len = s1.len().max(s2.len());
        if max_len == 0 {
            return 1.0;
        }
        let distance = Self::levenshtein_distance(s1, s2);
        1.0 - (distance as f64 / max_len as f64)
    }
}

impl Evaluator for SimilarityEvaluator {
    fn evaluate(&self, actual: &str, expected: &str) -> EvalResult {
        let score = Self::similarity_score(actual, expected);
        if score >= self.threshold {
            EvalResult::partial(score, format!("Similarity: {:.2}%", score * 100.0))
        } else {
            EvalResult::fail(format!(
                "Similarity {:.2}% below threshold {:.2}%",
                score * 100.0,
                self.threshold * 100.0
            ))
            .detail("score", format!("{:.4}", score))
        }
    }

    fn name(&self) -> &str {
        "similarity"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_result_pass() {
        let result = EvalResult::pass();
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_eval_result_fail() {
        let result = EvalResult::fail("wrong answer");
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
        assert_eq!(result.reason, Some("wrong answer".to_string()));
    }

    #[test]
    fn test_eval_result_partial() {
        let result = EvalResult::partial(0.7, "close enough");
        assert!(result.passed); // 0.7 >= 0.5
        assert_eq!(result.score, 0.7);
    }

    #[test]
    fn test_exact_match_pass() {
        let eval = ExactMatchEvaluator::new();
        let result = eval.evaluate("hello", "hello");
        assert!(result.passed);
    }

    #[test]
    fn test_exact_match_fail() {
        let eval = ExactMatchEvaluator::new();
        let result = eval.evaluate("hello", "world");
        assert!(!result.passed);
    }

    #[test]
    fn test_exact_match_ignore_case() {
        let eval = ExactMatchEvaluator::new().ignore_case();
        let result = eval.evaluate("HELLO", "hello");
        assert!(result.passed);
    }

    #[test]
    fn test_exact_match_trim() {
        let eval = ExactMatchEvaluator::new().trim();
        let result = eval.evaluate("  hello  ", "hello");
        assert!(result.passed);
    }

    #[test]
    fn test_contains_pass() {
        let eval = ContainsEvaluator::new();
        let result = eval.evaluate("The answer is 42", "42");
        assert!(result.passed);
    }

    #[test]
    fn test_contains_fail() {
        let eval = ContainsEvaluator::new();
        let result = eval.evaluate("The answer is 42", "43");
        assert!(!result.passed);
    }

    #[test]
    fn test_contains_ignore_case() {
        let eval = ContainsEvaluator::new().ignore_case();
        let result = eval.evaluate("HELLO world", "hello");
        assert!(result.passed);
    }

    #[test]
    fn test_regex_pass() {
        let eval = RegexEvaluator::new();
        let result = eval.evaluate("The answer is 42", r"\d+");
        assert!(result.passed);
    }

    #[test]
    fn test_regex_fail() {
        let eval = RegexEvaluator::new();
        let result = eval.evaluate("No numbers here", r"\d+");
        assert!(!result.passed);
    }

    #[test]
    fn test_regex_ignore_case() {
        let eval = RegexEvaluator::new().ignore_case();
        let result = eval.evaluate("HELLO world", "hello");
        assert!(result.passed);
    }

    #[test]
    fn test_json_match_pass() {
        let eval = JsonMatchEvaluator::new();
        let result = eval.evaluate(r#"{"a": 1, "b": 2}"#, r#"{"a": 1, "b": 2}"#);
        assert!(result.passed);
    }

    #[test]
    fn test_json_match_fail() {
        let eval = JsonMatchEvaluator::new();
        let result = eval.evaluate(r#"{"a": 1}"#, r#"{"a": 2}"#);
        assert!(!result.passed);
    }

    #[test]
    fn test_json_match_ignore_extra() {
        let eval = JsonMatchEvaluator::new().ignore_extra();
        let result = eval.evaluate(r#"{"a": 1, "b": 2}"#, r#"{"a": 1}"#);
        assert!(result.passed);
    }

    #[test]
    fn test_similarity_pass() {
        let eval = SimilarityEvaluator::new(0.8);
        let result = eval.evaluate("hello", "hello");
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_similarity_close() {
        let eval = SimilarityEvaluator::new(0.7);
        let result = eval.evaluate("hello", "hallo");
        assert!(result.passed); // 80% similar
        assert!(result.score >= 0.7);
    }

    #[test]
    fn test_similarity_fail() {
        let eval = SimilarityEvaluator::new(0.9);
        let result = eval.evaluate("hello", "world");
        assert!(!result.passed);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(SimilarityEvaluator::levenshtein_distance("", ""), 0);
        assert_eq!(SimilarityEvaluator::levenshtein_distance("a", ""), 1);
        assert_eq!(SimilarityEvaluator::levenshtein_distance("", "a"), 1);
        assert_eq!(
            SimilarityEvaluator::levenshtein_distance("hello", "hello"),
            0
        );
        assert_eq!(
            SimilarityEvaluator::levenshtein_distance("hello", "hallo"),
            1
        );
        assert_eq!(
            SimilarityEvaluator::levenshtein_distance("kitten", "sitting"),
            3
        );
    }

    #[test]
    fn test_evaluator_names() {
        assert_eq!(ExactMatchEvaluator::new().name(), "exact_match");
        assert_eq!(ContainsEvaluator::new().name(), "contains");
        assert_eq!(RegexEvaluator::new().name(), "regex");
        assert_eq!(JsonMatchEvaluator::new().name(), "json_match");
        assert_eq!(SimilarityEvaluator::new(0.8).name(), "similarity");
    }
}
