//! Structured codebase-context features (Rust parity with `scripts/panel_features.py`).
//!
//! Coarse, robust signals describing the size and shape of the context attached to a
//! prompt: approximate token count, number of file-path-like references, and binary
//! has-code / has-diff / has-error flags. Used by the embedding-head selector.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

pub const CTX_TOKEN_SCALE: f32 = 2000.0;
pub const N_FILES_SCALE: f32 = 8.0;

static FILE_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*[-*]?\s*(?:src/|\./)?[\w./-]+\.[A-Za-z]{1,4}\b").unwrap());
static DIFF: Lazy<Regex> = Lazy::new(|| Regex::new(r"diff --git|(?m)^@@ ").unwrap());
static ERROR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Traceback|Exception|panic!|Error:|\bpanic\b|stack trace").unwrap()
});
static CODE_KW: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)```|^\s{4}\S|\bdef \b|\bfn \b|\bfunction \b|\bclass \b|\bimport \b|SELECT\s")
        .unwrap()
});

/// Raw structured features extracted from text.
#[derive(Debug, Clone, Copy)]
pub struct Features {
    /// Approximate token count (chars / 4).
    pub ctx_tokens: usize,
    /// Distinct file-path-like references.
    pub n_files: usize,
    /// Whether code is present.
    pub has_code: bool,
    /// Whether a diff is present.
    pub has_diff: bool,
    /// Whether an error / stack trace is present.
    pub has_error: bool,
}

/// Extract structured features from text (mirrors the Python extractor).
pub fn extract_features(text: &str) -> Features {
    let has_diff = DIFF.is_match(text);
    let has_error = ERROR.is_match(text);
    let has_code = text.contains("```") || CODE_KW.is_match(text);
    let files: HashSet<&str> = FILE_LINE
        .find_iter(text)
        .map(|m| m.as_str().trim_matches([' ', '-', '*']).trim())
        .collect();
    Features {
        ctx_tokens: (text.len() / 4).max(1),
        n_files: files.len(),
        has_code,
        has_diff,
        has_error,
    }
}

/// Normalize features into the 5-element vector the head consumes
/// (order matches `panel_features.STRUCT_FEATURES`).
pub fn norm_struct(f: &Features) -> [f32; 5] {
    [
        (f.ctx_tokens as f32 / CTX_TOKEN_SCALE).min(1.0),
        (f.n_files as f32 / N_FILES_SCALE).min(1.0),
        f.has_code as u8 as f32,
        f.has_diff as u8 as f32,
        f.has_error as u8 as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_code_and_files() {
        let t = "refactor this\nFiles:\n- src/a.rs\n- src/b.rs\n```rust\nfn x() {}\n```";
        let f = extract_features(t);
        assert!(f.has_code);
        assert!(f.n_files >= 2);
        assert!(!f.has_error);
    }

    #[test]
    fn detects_error_and_diff() {
        let t = "fix\ndiff --git a/x b/x\nTraceback (most recent call last):\nValueError";
        let f = extract_features(t);
        assert!(f.has_diff);
        assert!(f.has_error);
    }

    #[test]
    fn plain_prompt_is_empty() {
        let f = extract_features("hello there");
        assert!(!f.has_code && !f.has_diff && !f.has_error);
        assert_eq!(f.n_files, 0);
        let n = norm_struct(&f);
        assert_eq!(n[2], 0.0);
    }
}
