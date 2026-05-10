//! Evaluation suite for running multiple test cases.

use super::case::TestCase;
use super::evaluators::{EvalResult, Evaluator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Statistics for a suite run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteStats {
    /// Total number of test cases.
    pub total: usize,
    /// Number of passed tests.
    pub passed: usize,
    /// Number of failed tests.
    pub failed: usize,
    /// Average score.
    pub avg_score: f64,
    /// Weighted average score.
    pub weighted_avg_score: f64,
    /// Pass rate (0.0 to 1.0).
    pub pass_rate: f64,
    /// Total run time.
    pub duration_ms: u64,
}

impl Default for SuiteStats {
    fn default() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            avg_score: 0.0,
            weighted_avg_score: 0.0,
            pass_rate: 0.0,
            duration_ms: 0,
        }
    }
}

/// Result of a single test case evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    /// Test case.
    pub case: TestCase,
    /// Actual output.
    pub actual: String,
    /// Evaluation result.
    pub eval: EvalResult,
    /// Time taken for evaluation.
    pub duration_ms: u64,
}

/// Result of running an evaluation suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    /// Suite name.
    pub name: String,
    /// Individual case results.
    pub results: Vec<CaseResult>,
    /// Statistics.
    pub stats: SuiteStats,
    /// Evaluator used.
    pub evaluator: String,
}

impl SuiteResult {
    /// Get failed test cases.
    pub fn failures(&self) -> Vec<&CaseResult> {
        self.results.iter().filter(|r| !r.eval.passed).collect()
    }

    /// Get passed test cases.
    pub fn passes(&self) -> Vec<&CaseResult> {
        self.results.iter().filter(|r| r.eval.passed).collect()
    }

    /// Get results by tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&CaseResult> {
        self.results
            .iter()
            .filter(|r| r.case.has_tag(tag))
            .collect()
    }

    /// Generate a summary report.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("=== {} ===", self.name));
        lines.push(format!("Evaluator: {}", self.evaluator));
        lines.push(format!(
            "Results: {}/{} passed ({:.1}%)",
            self.stats.passed,
            self.stats.total,
            self.stats.pass_rate * 100.0
        ));
        lines.push(format!("Average Score: {:.2}", self.stats.avg_score));
        lines.push(format!("Duration: {}ms", self.stats.duration_ms));

        if !self.failures().is_empty() {
            lines.push(String::new());
            lines.push("Failures:".to_string());
            for failure in self.failures() {
                lines.push(format!(
                    "  - {}: {}",
                    failure.case.display_name(),
                    failure.eval.reason.as_deref().unwrap_or("Unknown")
                ));
            }
        }

        lines.join("\n")
    }
}

/// An evaluation suite containing multiple test cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    /// Suite name.
    pub name: String,
    /// Test cases.
    pub cases: Vec<TestCase>,
    /// Suite metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl EvalSuite {
    /// Create a new evaluation suite.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cases: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a test case.
    pub fn add_case(mut self, case: TestCase) -> Self {
        self.cases.push(case);
        self
    }

    /// Add multiple test cases.
    pub fn add_cases(mut self, cases: impl IntoIterator<Item = TestCase>) -> Self {
        self.cases.extend(cases);
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the number of test cases.
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// Check if the suite is empty.
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// Filter cases by tag.
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&TestCase> {
        self.cases.iter().filter(|c| c.has_tag(tag)).collect()
    }

    /// Run the suite with an evaluator and actual outputs.
    pub fn run(&self, evaluator: &dyn Evaluator, outputs: &[String]) -> SuiteResult {
        assert_eq!(
            self.cases.len(),
            outputs.len(),
            "Number of outputs must match number of test cases"
        );

        let start = Instant::now();
        let mut results = Vec::with_capacity(self.cases.len());
        let mut total_score = 0.0;
        let mut weighted_score = 0.0;
        let mut total_weight = 0.0;
        let mut passed = 0;

        for (case, actual) in self.cases.iter().zip(outputs.iter()) {
            let case_start = Instant::now();
            let eval = evaluator.evaluate(actual, &case.expected);
            let duration_ms = case_start.elapsed().as_millis() as u64;

            total_score += eval.score;
            weighted_score += eval.score * case.weight;
            total_weight += case.weight;

            if eval.passed {
                passed += 1;
            }

            results.push(CaseResult {
                case: case.clone(),
                actual: actual.clone(),
                eval,
                duration_ms,
            });
        }

        let total = self.cases.len();
        let avg_score = if total > 0 {
            total_score / total as f64
        } else {
            0.0
        };
        let weighted_avg = if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            0.0
        };
        let pass_rate = if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        };

        SuiteResult {
            name: self.name.clone(),
            results,
            stats: SuiteStats {
                total,
                passed,
                failed: total - passed,
                avg_score,
                weighted_avg_score: weighted_avg,
                pass_rate,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            evaluator: evaluator.name().to_string(),
        }
    }

    /// Run the suite with a function that generates outputs.
    pub fn run_with<F>(&self, evaluator: &dyn Evaluator, generator: F) -> SuiteResult
    where
        F: Fn(&str) -> String,
    {
        let outputs: Vec<String> = self.cases.iter().map(|c| generator(&c.input)).collect();
        self.run(evaluator, &outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::super::evaluators::ExactMatchEvaluator;
    use super::*;

    #[test]
    fn test_eval_suite_new() {
        let suite = EvalSuite::new("test_suite");
        assert_eq!(suite.name, "test_suite");
        assert!(suite.is_empty());
    }

    #[test]
    fn test_eval_suite_add_cases() {
        let suite = EvalSuite::new("math")
            .add_case(TestCase::new("1+1", "2"))
            .add_case(TestCase::new("2+2", "4"));

        assert_eq!(suite.len(), 2);
    }

    #[test]
    fn test_eval_suite_run_all_pass() {
        let suite = EvalSuite::new("math")
            .add_case(TestCase::new("1+1", "2"))
            .add_case(TestCase::new("2+2", "4"));

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["2".to_string(), "4".to_string()];

        let result = suite.run(&evaluator, &outputs);

        assert_eq!(result.stats.total, 2);
        assert_eq!(result.stats.passed, 2);
        assert_eq!(result.stats.failed, 0);
        assert_eq!(result.stats.pass_rate, 1.0);
    }

    #[test]
    fn test_eval_suite_run_some_fail() {
        let suite = EvalSuite::new("math")
            .add_case(TestCase::new("1+1", "2"))
            .add_case(TestCase::new("2+2", "4"));

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["2".to_string(), "5".to_string()]; // Wrong answer

        let result = suite.run(&evaluator, &outputs);

        assert_eq!(result.stats.total, 2);
        assert_eq!(result.stats.passed, 1);
        assert_eq!(result.stats.failed, 1);
        assert_eq!(result.stats.pass_rate, 0.5);
    }

    #[test]
    fn test_eval_suite_run_with_generator() {
        let suite = EvalSuite::new("echo")
            .add_case(TestCase::new("hello", "hello"))
            .add_case(TestCase::new("world", "world"));

        let evaluator = ExactMatchEvaluator::new();
        let result = suite.run_with(&evaluator, |input| input.to_string());

        assert_eq!(result.stats.passed, 2);
    }

    #[test]
    fn test_suite_result_failures() {
        let suite = EvalSuite::new("test")
            .add_case(TestCase::new("a", "a"))
            .add_case(TestCase::new("b", "b"));

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["a".to_string(), "x".to_string()];

        let result = suite.run(&evaluator, &outputs);
        let failures = result.failures();

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].case.expected, "b");
    }

    #[test]
    fn test_suite_filter_by_tag() {
        let suite = EvalSuite::new("mixed")
            .add_case(TestCase::new("1", "1").tag("math"))
            .add_case(TestCase::new("hello", "hello").tag("text"))
            .add_case(TestCase::new("2", "2").tag("math"));

        let math_cases = suite.filter_by_tag("math");
        assert_eq!(math_cases.len(), 2);
    }

    #[test]
    fn test_suite_result_by_tag() {
        let suite = EvalSuite::new("mixed")
            .add_case(TestCase::new("1", "1").tag("math"))
            .add_case(TestCase::new("hello", "hello").tag("text"));

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["1".to_string(), "hello".to_string()];
        let result = suite.run(&evaluator, &outputs);

        let math_results = result.by_tag("math");
        assert_eq!(math_results.len(), 1);
    }

    #[test]
    fn test_suite_weighted_score() {
        let suite = EvalSuite::new("weighted")
            .add_case(TestCase::new("a", "a").weight(1.0))
            .add_case(TestCase::new("b", "b").weight(3.0)); // 3x weight

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["a".to_string(), "x".to_string()]; // a passes, b fails

        let result = suite.run(&evaluator, &outputs);

        // a: score 1.0 * weight 1.0 = 1.0
        // b: score 0.0 * weight 3.0 = 0.0
        // weighted avg = 1.0 / 4.0 = 0.25
        assert_eq!(result.stats.weighted_avg_score, 0.25);
    }

    #[test]
    fn test_suite_summary() {
        let suite = EvalSuite::new("test_suite")
            .add_case(TestCase::new("a", "a").name("test_a"))
            .add_case(TestCase::new("b", "b").name("test_b"));

        let evaluator = ExactMatchEvaluator::new();
        let outputs = vec!["a".to_string(), "x".to_string()];
        let result = suite.run(&evaluator, &outputs);

        let summary = result.summary();
        assert!(summary.contains("test_suite"));
        assert!(summary.contains("1/2 passed"));
        assert!(summary.contains("Failures:"));
        assert!(summary.contains("test_b"));
    }
}
