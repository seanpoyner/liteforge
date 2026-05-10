//! Sandbox for safe code execution.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Output;
use std::time::Duration;

/// Result of code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Whether execution was killed due to timeout.
    pub timed_out: bool,
    /// Any files created during execution.
    pub output_files: Vec<OutputFile>,
}

impl ExecutionResult {
    /// Check if execution was successful.
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }

    /// Get combined output (stdout + stderr).
    pub fn output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// An output file created during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// File path relative to working directory.
    pub path: String,
    /// File contents (if text) or base64-encoded (if binary).
    pub content: String,
    /// Whether content is base64-encoded.
    pub is_binary: bool,
}

/// Configuration for sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum execution time.
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Maximum memory in bytes.
    pub max_memory: usize,
    /// Maximum CPU time in seconds.
    pub max_cpu_time: u64,
    /// Working directory.
    pub working_dir: Option<String>,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Whether to allow network access.
    pub allow_network: bool,
    /// Whether to allow file system writes.
    pub allow_fs_write: bool,
    /// Allowed paths for reading (if restricted).
    pub allowed_read_paths: Vec<String>,
    /// Allowed paths for writing (if restricted).
    pub allowed_write_paths: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_memory: 256 * 1024 * 1024, // 256 MB
            max_cpu_time: 10,
            working_dir: None,
            env: HashMap::new(),
            allow_network: false,
            allow_fs_write: false,
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
        }
    }
}

impl SandboxConfig {
    /// Create a new sandbox config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set max memory.
    pub fn max_memory(mut self, bytes: usize) -> Self {
        self.max_memory = bytes;
        self
    }

    /// Set working directory.
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Allow network access.
    pub fn allow_network(mut self, allow: bool) -> Self {
        self.allow_network = allow;
        self
    }

    /// Allow filesystem writes.
    pub fn allow_fs_write(mut self, allow: bool) -> Self {
        self.allow_fs_write = allow;
        self
    }
}

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Ruby,
    Rust,
    Go,
    Shell,
}

impl Language {
    /// Get the file extension for this language.
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Ruby => "rb",
            Language::Rust => "rs",
            Language::Go => "go",
            Language::Shell => "sh",
        }
    }

    /// Get the default interpreter/compiler command.
    pub fn command(&self) -> &'static str {
        match self {
            Language::Python => "python3",
            Language::JavaScript => "node",
            Language::TypeScript => "npx ts-node",
            Language::Ruby => "ruby",
            Language::Rust => "rustc",
            Language::Go => "go run",
            Language::Shell => "bash",
        }
    }

    /// Detect language from code content.
    pub fn detect(code: &str) -> Option<Language> {
        let code = code.trim();

        // Check for shebangs
        if code.starts_with("#!/usr/bin/env python") || code.starts_with("#!/usr/bin/python") {
            return Some(Language::Python);
        }
        if code.starts_with("#!/usr/bin/env node") || code.starts_with("#!/usr/bin/node") {
            return Some(Language::JavaScript);
        }
        if code.starts_with("#!/bin/bash") || code.starts_with("#!/bin/sh") {
            return Some(Language::Shell);
        }

        // Heuristics based on syntax
        if code.contains("def ") && code.contains(":") && !code.contains("{") {
            return Some(Language::Python);
        }
        if code.contains("import ") && code.contains("from ") && code.contains(":") {
            return Some(Language::Python);
        }
        if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            if code.contains(": string") || code.contains(": number") || code.contains(": boolean")
            {
                return Some(Language::TypeScript);
            }
            return Some(Language::JavaScript);
        }
        if code.contains("fn ") && code.contains("->") {
            return Some(Language::Rust);
        }
        if code.contains("func ") && code.contains("package ") {
            return Some(Language::Go);
        }

        None
    }
}

/// Error from sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxError {
    /// Error message.
    pub message: String,
    /// Error kind.
    pub kind: SandboxErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxErrorKind {
    /// Timeout exceeded.
    Timeout,
    /// Memory limit exceeded.
    MemoryLimit,
    /// Security violation.
    SecurityViolation,
    /// Language not supported.
    UnsupportedLanguage,
    /// Execution failed.
    ExecutionFailed,
    /// Configuration error.
    ConfigError,
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for SandboxError {}

impl SandboxError {
    pub fn new(kind: SandboxErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(SandboxErrorKind::Timeout, message)
    }

    pub fn security(message: impl Into<String>) -> Self {
        Self::new(SandboxErrorKind::SecurityViolation, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(SandboxErrorKind::UnsupportedLanguage, message)
    }

    pub fn execution(message: impl Into<String>) -> Self {
        Self::new(SandboxErrorKind::ExecutionFailed, message)
    }
}

/// Trait for sandbox implementations.
#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Execute code in the sandbox.
    async fn execute(
        &self,
        code: &str,
        language: Language,
        config: &SandboxConfig,
    ) -> Result<ExecutionResult, SandboxError>;

    /// Check if a language is supported.
    fn supports_language(&self, language: Language) -> bool;

    /// Get supported languages.
    fn supported_languages(&self) -> Vec<Language>;
}

/// A simple process-based sandbox (not fully isolated).
///
/// WARNING: This is NOT a secure sandbox. For production use,
/// consider using Docker, gVisor, or WASM-based isolation.
#[derive(Debug, Clone, Default)]
pub struct ProcessSandbox {
    /// Base directory for temporary files.
    pub temp_dir: Option<String>,
}

impl ProcessSandbox {
    /// Create a new process sandbox.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the temp directory.
    pub fn with_temp_dir(mut self, dir: impl Into<String>) -> Self {
        self.temp_dir = Some(dir.into());
        self
    }

    async fn run_command(
        &self,
        cmd: &str,
        args: &[&str],
        config: &SandboxConfig,
    ) -> Result<Output, SandboxError> {
        use std::process::Command;

        let mut command = Command::new(cmd);
        command.args(args);

        // Set working directory
        if let Some(ref dir) = config.working_dir {
            command.current_dir(dir);
        }

        // Set environment
        command.env_clear();
        for (key, value) in &config.env {
            command.env(key, value);
        }

        // Add minimal required env vars
        command.env("PATH", "/usr/local/bin:/usr/bin:/bin");
        command.env("HOME", "/tmp");

        let output = command
            .output()
            .map_err(|e| SandboxError::execution(format!("Failed to execute: {}", e)))?;

        Ok(output)
    }
}

#[async_trait]
impl Sandbox for ProcessSandbox {
    async fn execute(
        &self,
        code: &str,
        language: Language,
        config: &SandboxConfig,
    ) -> Result<ExecutionResult, SandboxError> {
        use std::io::Write;
        use std::time::Instant;

        // Create temp file for the code
        let temp_dir = self.temp_dir.as_deref().unwrap_or("/tmp");
        let file_name = format!(
            "{}/sandbox_code_{}.{}",
            temp_dir,
            std::process::id(),
            language.extension()
        );

        // Write code to file
        let mut file = std::fs::File::create(&file_name)
            .map_err(|e| SandboxError::execution(format!("Failed to create temp file: {}", e)))?;
        file.write_all(code.as_bytes())
            .map_err(|e| SandboxError::execution(format!("Failed to write code: {}", e)))?;

        let start = Instant::now();

        // Execute based on language
        let output = match language {
            Language::Python => self.run_command("python3", &[&file_name], config).await?,
            Language::JavaScript => self.run_command("node", &[&file_name], config).await?,
            Language::Shell => self.run_command("bash", &[&file_name], config).await?,
            Language::Ruby => self.run_command("ruby", &[&file_name], config).await?,
            _ => {
                // Clean up temp file
                let _ = std::fs::remove_file(&file_name);
                return Err(SandboxError::unsupported(format!(
                    "{:?} is not supported by ProcessSandbox",
                    language
                )));
            }
        };

        let execution_time = start.elapsed();

        // Clean up temp file
        let _ = std::fs::remove_file(&file_name);

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms: execution_time.as_millis() as u64,
            timed_out: false,
            output_files: Vec::new(),
        })
    }

    fn supports_language(&self, language: Language) -> bool {
        matches!(
            language,
            Language::Python | Language::JavaScript | Language::Shell | Language::Ruby
        )
    }

    fn supported_languages(&self) -> Vec<Language> {
        vec![
            Language::Python,
            Language::JavaScript,
            Language::Shell,
            Language::Ruby,
        ]
    }
}

/// A no-op sandbox that just returns mock results (for testing).
#[derive(Debug, Clone, Default)]
pub struct MockSandbox {
    /// Predefined result to return.
    pub result: Option<ExecutionResult>,
}

impl MockSandbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_result(mut self, result: ExecutionResult) -> Self {
        self.result = Some(result);
        self
    }
}

#[async_trait]
impl Sandbox for MockSandbox {
    async fn execute(
        &self,
        _code: &str,
        _language: Language,
        _config: &SandboxConfig,
    ) -> Result<ExecutionResult, SandboxError> {
        Ok(self.result.clone().unwrap_or(ExecutionResult {
            exit_code: 0,
            stdout: "Mock execution successful".to_string(),
            stderr: String::new(),
            execution_time_ms: 1,
            timed_out: false,
            output_files: Vec::new(),
        }))
    }

    fn supports_language(&self, _language: Language) -> bool {
        true
    }

    fn supported_languages(&self) -> Vec<Language> {
        vec![
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Ruby,
            Language::Rust,
            Language::Go,
            Language::Shell,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult {
            exit_code: 0,
            stdout: "Hello".to_string(),
            stderr: String::new(),
            execution_time_ms: 10,
            timed_out: false,
            output_files: Vec::new(),
        };

        assert!(result.success());
        assert_eq!(result.output(), "Hello");
    }

    #[test]
    fn test_execution_result_failed() {
        let result = ExecutionResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "Error".to_string(),
            execution_time_ms: 10,
            timed_out: false,
            output_files: Vec::new(),
        };

        assert!(!result.success());
        assert_eq!(result.output(), "Error");
    }

    #[test]
    fn test_sandbox_config() {
        let config = SandboxConfig::new()
            .timeout(Duration::from_secs(60))
            .max_memory(512 * 1024 * 1024)
            .working_dir("/tmp/sandbox")
            .env("FOO", "bar")
            .allow_network(true);

        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_memory, 512 * 1024 * 1024);
        assert_eq!(config.working_dir, Some("/tmp/sandbox".to_string()));
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
        assert!(config.allow_network);
    }

    #[test]
    fn test_language_extension() {
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::JavaScript.extension(), "js");
        assert_eq!(Language::Rust.extension(), "rs");
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            Language::detect("def foo():\n    pass"),
            Some(Language::Python)
        );
        assert_eq!(
            Language::detect("function foo() { }"),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::detect("fn main() -> () { }"),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::detect("#!/bin/bash\necho hello"),
            Some(Language::Shell)
        );
    }

    #[tokio::test]
    async fn test_mock_sandbox() {
        let sandbox = MockSandbox::new().with_result(ExecutionResult {
            exit_code: 0,
            stdout: "42".to_string(),
            stderr: String::new(),
            execution_time_ms: 5,
            timed_out: false,
            output_files: Vec::new(),
        });

        let config = SandboxConfig::default();
        let result = sandbox
            .execute("print(42)", Language::Python, &config)
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout, "42");
    }

    #[test]
    fn test_sandbox_error() {
        let err = SandboxError::timeout("Execution took too long");
        assert_eq!(err.kind, SandboxErrorKind::Timeout);
        assert!(err.to_string().contains("Execution took too long"));
    }
}
