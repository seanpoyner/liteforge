//! Evaluation framework for testing agent outputs.
//!
//! This module provides tools for evaluating AI model outputs against
//! expected results using various evaluation strategies.
//!
//! # Overview
//!
//! - **TestCase**: A single test with input and expected output
//! - **EvalSuite**: A collection of test cases
//! - **Evaluator**: Trait for implementing evaluation logic
//! - **EvalResult**: Result of running an evaluation
//!
//! # Example
//!
//! ```rust
//! use liteforge::evals::{TestCase, EvalSuite, ExactMatchEvaluator, Evaluator};
//!
//! let suite = EvalSuite::new("math_tests")
//!     .add_case(TestCase::new("2+2", "4"))
//!     .add_case(TestCase::new("3*3", "9"));
//!
//! let evaluator = ExactMatchEvaluator::new();
//!
//! // Run evaluation with actual outputs
//! let result = evaluator.evaluate("4", "4");
//! assert!(result.passed);
//! ```

mod case;
mod evaluators;
mod suite;

pub use case::{TestCase, TestCaseBuilder};
pub use evaluators::{
    ContainsEvaluator, EvalResult, Evaluator, ExactMatchEvaluator, JsonMatchEvaluator,
    RegexEvaluator, SimilarityEvaluator,
};
pub use suite::{EvalSuite, SuiteResult, SuiteStats};
