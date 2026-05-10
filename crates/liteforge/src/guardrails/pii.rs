//! PII (Personally Identifiable Information) detection and redaction.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::GuardrailResult;

/// Types of PII that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiType {
    /// Social Security Number (XXX-XX-XXXX)
    Ssn,
    /// Phone number
    Phone,
    /// Email address
    Email,
    /// Credit card number
    CreditCard,
    /// IP address
    IpAddress,
}

impl PiiType {
    /// Get the human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            PiiType::Ssn => "SSN",
            PiiType::Phone => "phone number",
            PiiType::Email => "email address",
            PiiType::CreditCard => "credit card",
            PiiType::IpAddress => "IP address",
        }
    }
}

/// PII patterns with their corresponding types.
pub static PII_PATTERNS: Lazy<Vec<(PiiType, Regex)>> = Lazy::new(|| {
    vec![
        (PiiType::Ssn, Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap()),
        (
            PiiType::Phone,
            Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap(),
        ),
        (
            PiiType::Email,
            Regex::new(r"(?i)\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
        ),
        (
            PiiType::CreditCard,
            Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap(),
        ),
        (
            PiiType::IpAddress,
            Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
        ),
    ]
});

/// Detect PII in text.
///
/// Returns a GuardrailResult indicating whether PII was found.
///
/// # Example
///
/// ```
/// use liteforge::guardrails::detect_pii;
///
/// let result = detect_pii("Contact me at test@example.com");
/// assert!(!result.passed);
/// assert!(result.message.contains("email"));
/// ```
pub fn detect_pii(text: &str) -> GuardrailResult {
    for (pii_type, pattern) in PII_PATTERNS.iter() {
        if pattern.is_match(text) {
            return GuardrailResult::fail(text, format!("PII detected: {}", pii_type.name()))
                .with_name("detect_pii");
        }
    }

    GuardrailResult::pass(text).with_name("detect_pii")
}

/// Redact PII from text by replacing with [REDACTED].
///
/// # Example
///
/// ```
/// use liteforge::guardrails::redact_pii;
///
/// let clean = redact_pii("My SSN is 123-45-6789");
/// assert_eq!(clean, "My SSN is [REDACTED]");
/// ```
pub fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();

    for (_, pattern) in PII_PATTERNS.iter() {
        result = pattern.replace_all(&result, "[REDACTED]").to_string();
    }

    result
}

/// Detect specific types of PII.
///
/// Returns a list of (PiiType, match) tuples for all PII found.
pub fn find_pii(text: &str) -> Vec<(PiiType, String)> {
    let mut found = Vec::new();

    for (pii_type, pattern) in PII_PATTERNS.iter() {
        for mat in pattern.find_iter(text) {
            found.push((*pii_type, mat.as_str().to_string()));
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ssn() {
        let result = detect_pii("SSN: 123-45-6789");
        assert!(!result.passed);
        assert!(result.message.contains("SSN"));
    }

    #[test]
    fn test_detect_email() {
        let result = detect_pii("Email: test@example.com");
        assert!(!result.passed);
        assert!(result.message.contains("email"));
    }

    #[test]
    fn test_detect_phone() {
        let result = detect_pii("Call me at 555-123-4567");
        assert!(!result.passed);
        assert!(result.message.contains("phone"));
    }

    #[test]
    fn test_no_pii() {
        let result = detect_pii("Hello, this is a normal message.");
        assert!(result.passed);
    }

    #[test]
    fn test_redact_pii() {
        let text = "My SSN is 123-45-6789 and email is test@example.com";
        let clean = redact_pii(text);
        assert!(clean.contains("[REDACTED]"));
        assert!(!clean.contains("123-45-6789"));
        assert!(!clean.contains("test@example.com"));
    }

    #[test]
    fn test_find_pii() {
        let text = "Contact: test@example.com, 555-123-4567";
        let found = find_pii(text);
        assert_eq!(found.len(), 2);
    }
}
