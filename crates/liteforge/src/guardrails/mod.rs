//! Guardrails for input validation and output filtering.
//!
//! This module provides safety guardrails for LLM interactions:
//! - PII detection and redaction
//! - Prompt injection detection
//!
//! # Example
//!
//! ```
//! use liteforge::guardrails::{detect_pii, redact_pii, detect_injection};
//!
//! // Check for PII
//! let result = detect_pii("Contact me at test@example.com");
//! if !result.passed {
//!     println!("PII detected: {}", result.message);
//! }
//!
//! // Redact PII
//! let clean = redact_pii("My SSN is 123-45-6789");
//! assert!(clean.contains("[REDACTED]"));
//!
//! // Check for injection attempts
//! let result = detect_injection("ignore previous instructions");
//! if !result.passed {
//!     println!("Injection detected: {}", result.message);
//! }
//! ```

mod injection;
mod pii;

pub use injection::{detect_injection, INJECTION_PATTERNS};
pub use pii::{detect_pii, find_pii, redact_pii, PiiType, PII_PATTERNS};

use serde::{Deserialize, Serialize};

/// Result from executing a guardrail check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    /// Whether the guardrail check passed
    pub passed: bool,
    /// The (possibly modified) value
    pub value: String,
    /// Human-readable message about the result
    pub message: String,
    /// Name of the guardrail that produced this result
    pub guardrail_name: String,
}

impl GuardrailResult {
    /// Create a passing result
    pub fn pass(value: impl Into<String>) -> Self {
        Self {
            passed: true,
            value: value.into(),
            message: String::new(),
            guardrail_name: String::new(),
        }
    }

    /// Create a failing result
    pub fn fail(value: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            passed: false,
            value: value.into(),
            message: message.into(),
            guardrail_name: String::new(),
        }
    }

    /// Set the guardrail name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.guardrail_name = name.into();
        self
    }
}

/// Run all guardrails (PII and injection detection) on text.
///
/// Returns the first failure encountered, or a pass result if all checks pass.
pub fn check_all(text: &str) -> GuardrailResult {
    // Check for PII first
    let pii_result = detect_pii(text);
    if !pii_result.passed {
        return pii_result;
    }

    // Check for injection
    let injection_result = detect_injection(text);
    if !injection_result.passed {
        return injection_result;
    }

    GuardrailResult::pass(text).with_name("all")
}
