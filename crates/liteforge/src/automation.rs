//! Automation for running workflows on schedules.
//!
//! This module provides tools for automating workflow execution
//! based on time schedules, triggers, and conditions.

use crate::client::AsyncForgeClient;
use crate::error::ForgeError;
use crate::scheduler::{CronSchedule, IntervalSchedule, OnceSchedule, Schedule};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error type for automation operations.
#[derive(Debug, thiserror::Error)]
pub enum AutomationError {
    /// Task not found.
    #[error("Automation task not found: {0}")]
    NotFound(String),

    /// Task execution failed.
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Schedule error.
    #[error("Schedule error: {0}")]
    ScheduleError(String),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] ForgeError),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// Result type for automation operations.
pub type AutomationResult<T> = Result<T, AutomationError>;

/// Configuration for an automation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationConfig {
    /// Unique identifier for the automation.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this automation does.
    #[serde(default)]
    pub description: String,
    /// Whether this automation is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Schedule type.
    pub schedule: ScheduleConfig,
    /// Maximum number of retries on failure.
    #[serde(default)]
    pub max_retries: u32,
    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

impl AutomationConfig {
    /// Create a new automation config.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            enabled: true,
            schedule: ScheduleConfig::Once,
            max_retries: 0,
            timeout_secs: default_timeout(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the schedule.
    pub fn with_schedule(mut self, schedule: ScheduleConfig) -> Self {
        self.schedule = schedule;
        self
    }

    /// Set max retries.
    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set enabled status.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Schedule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ScheduleConfig {
    /// Run once immediately.
    Once,
    /// Run at regular intervals.
    Interval {
        /// Interval in seconds.
        seconds: u64,
    },
    /// Run on a cron schedule.
    Cron {
        /// Cron expression.
        expression: String,
    },
}

impl ScheduleConfig {
    /// Create an interval schedule.
    pub fn interval(seconds: u64) -> Self {
        Self::Interval { seconds }
    }

    /// Create a cron schedule.
    pub fn cron(expression: impl Into<String>) -> Self {
        Self::Cron {
            expression: expression.into(),
        }
    }

    /// Convert to a Schedule trait object.
    pub fn to_schedule(&self) -> AutomationResult<Box<dyn Schedule>> {
        match self {
            ScheduleConfig::Once => Ok(Box::new(OnceSchedule::now())),
            ScheduleConfig::Interval { seconds } => {
                Ok(Box::new(IntervalSchedule::from_secs(*seconds)))
            }
            ScheduleConfig::Cron { expression } => Ok(Box::new(CronSchedule::new(expression))),
        }
    }
}

/// Trait for automation tasks.
#[async_trait]
pub trait AutomationTask: Send + Sync {
    /// Get the task's configuration.
    fn config(&self) -> &AutomationConfig;

    /// Execute the task.
    async fn execute(
        &self,
        client: &AsyncForgeClient,
        context: &mut TaskContext,
    ) -> AutomationResult<TaskOutput>;
}

/// Context for task execution.
#[derive(Debug, Default, Clone)]
pub struct TaskContext {
    /// Input data for the task.
    pub input: HashMap<String, serde_json::Value>,
    /// Shared state between tasks.
    pub state: HashMap<String, serde_json::Value>,
    /// Execution metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TaskContext {
    /// Create a new task context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an input value.
    pub fn set_input(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.input.insert(key.into(), value);
    }

    /// Get an input value.
    pub fn get_input(&self, key: &str) -> Option<&serde_json::Value> {
        self.input.get(key)
    }

    /// Set a state value.
    pub fn set_state(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
    }

    /// Get a state value.
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }
}

/// Output from a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Whether the task succeeded.
    pub success: bool,
    /// Output message.
    pub message: String,
    /// Output data.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

impl TaskOutput {
    /// Create a successful output.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            duration_ms: 0,
        }
    }

    /// Create a failed output.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            duration_ms: 0,
        }
    }

    /// Set output data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Status of an automation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
}

/// Record of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Execution ID.
    pub execution_id: String,
    /// Task ID.
    pub task_id: String,
    /// Start time (Unix timestamp).
    pub started_at: u64,
    /// End time (Unix timestamp).
    pub ended_at: Option<u64>,
    /// Task status.
    pub status: TaskStatus,
    /// Task output.
    pub output: Option<TaskOutput>,
}

/// Runner for automation tasks.
pub struct AutomationRunner {
    tasks: Arc<RwLock<HashMap<String, Arc<dyn AutomationTask>>>>,
    executions: Arc<RwLock<Vec<ExecutionRecord>>>,
    execution_counter: Arc<std::sync::atomic::AtomicU64>,
    max_history: usize,
}

impl Default for AutomationRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl AutomationRunner {
    /// Create a new automation runner.
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            execution_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            max_history: 100,
        }
    }

    /// Set the maximum number of execution records to keep.
    pub fn with_max_history(mut self, max_history: usize) -> Self {
        self.max_history = max_history;
        self
    }

    /// Register a task.
    pub async fn register<T: AutomationTask + 'static>(&self, task: T) {
        let id = task.config().id.clone();
        let mut tasks = self.tasks.write().await;
        tasks.insert(id, Arc::new(task));
    }

    /// Unregister a task.
    pub async fn unregister(&self, id: &str) -> Option<Arc<dyn AutomationTask>> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id)
    }

    /// Get a task by ID.
    pub async fn get(&self, id: &str) -> Option<Arc<dyn AutomationTask>> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// List all task IDs.
    pub async fn list(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.keys().cloned().collect()
    }

    /// List enabled task IDs.
    pub async fn list_enabled(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks
            .iter()
            .filter(|(_, t)| t.config().enabled)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Run a task by ID.
    pub async fn run(&self, id: &str, client: &AsyncForgeClient) -> AutomationResult<TaskOutput> {
        let task = self
            .get(id)
            .await
            .ok_or_else(|| AutomationError::NotFound(id.to_string()))?;

        if !task.config().enabled {
            return Err(AutomationError::ExecutionFailed(
                "Task is disabled".to_string(),
            ));
        }

        let started_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let counter = self
            .execution_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let execution_id = format!("exec-{}-{}", started_at, counter);

        // Record start
        {
            let mut executions = self.executions.write().await;
            executions.push(ExecutionRecord {
                execution_id: execution_id.clone(),
                task_id: id.to_string(),
                started_at,
                ended_at: None,
                status: TaskStatus::Running,
                output: None,
            });

            // Trim history
            if executions.len() > self.max_history {
                executions.remove(0);
            }
        }

        // Execute
        let mut context = TaskContext::new();
        let start = std::time::Instant::now();
        let result = task.execute(client, &mut context).await;
        let duration = start.elapsed();

        let ended_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update record
        {
            let mut executions = self.executions.write().await;
            if let Some(record) = executions
                .iter_mut()
                .find(|r| r.execution_id == execution_id)
            {
                record.ended_at = Some(ended_at);
                match &result {
                    Ok(output) => {
                        record.status = TaskStatus::Completed;
                        record.output =
                            Some(output.clone().with_duration(duration.as_millis() as u64));
                    }
                    Err(_) => {
                        record.status = TaskStatus::Failed;
                    }
                }
            }
        }

        result.map(|output| output.with_duration(duration.as_millis() as u64))
    }

    /// Get execution history.
    pub async fn history(&self) -> Vec<ExecutionRecord> {
        let executions = self.executions.read().await;
        executions.clone()
    }

    /// Get execution history for a specific task.
    pub async fn task_history(&self, task_id: &str) -> Vec<ExecutionRecord> {
        let executions = self.executions.read().await;
        executions
            .iter()
            .filter(|r| r.task_id == task_id)
            .cloned()
            .collect()
    }
}

/// A simple prompt-based automation task.
pub struct PromptTask {
    config: AutomationConfig,
    prompt: String,
    model: Option<String>,
}

impl PromptTask {
    /// Create a new prompt task.
    pub fn new(config: AutomationConfig, prompt: impl Into<String>) -> Self {
        Self {
            config,
            prompt: prompt.into(),
            model: None,
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[async_trait]
impl AutomationTask for PromptTask {
    fn config(&self) -> &AutomationConfig {
        &self.config
    }

    async fn execute(
        &self,
        client: &AsyncForgeClient,
        context: &mut TaskContext,
    ) -> AutomationResult<TaskOutput> {
        use crate::types::Message;

        // Substitute variables in prompt
        let mut prompt = self.prompt.clone();
        for (key, value) in &context.input {
            let placeholder = format!("{{{}}}", key);
            if let Some(val_str) = value.as_str() {
                prompt = prompt.replace(&placeholder, val_str);
            }
        }

        // Make the call
        let messages = vec![Message::user(&prompt)];

        let response = if let Some(ref model) = self.model {
            client.complete_with_model(model, messages).await
        } else {
            client.complete(messages).await
        };

        match response {
            Ok(completion) => {
                let text = completion
                    .choices
                    .first()
                    .and_then(|c| c.message.content.clone())
                    .unwrap_or_default();

                Ok(TaskOutput::success(text))
            }
            Err(e) => Err(AutomationError::LlmError(e)),
        }
    }
}

/// Builder for creating automation workflows.
pub struct AutomationBuilder {
    config: AutomationConfig,
}

impl AutomationBuilder {
    /// Create a new automation builder.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            config: AutomationConfig::new(id, "Unnamed Automation"),
        }
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.config.description = description.into();
        self
    }

    /// Set the schedule.
    pub fn schedule(mut self, schedule: ScheduleConfig) -> Self {
        self.config.schedule = schedule;
        self
    }

    /// Run every N seconds.
    pub fn every_seconds(self, seconds: u64) -> Self {
        self.schedule(ScheduleConfig::interval(seconds))
    }

    /// Run every N minutes.
    pub fn every_minutes(self, minutes: u64) -> Self {
        self.schedule(ScheduleConfig::interval(minutes * 60))
    }

    /// Run every N hours.
    pub fn every_hours(self, hours: u64) -> Self {
        self.schedule(ScheduleConfig::interval(hours * 3600))
    }

    /// Run on a cron schedule.
    pub fn cron(self, expression: impl Into<String>) -> Self {
        self.schedule(ScheduleConfig::cron(expression))
    }

    /// Set max retries.
    pub fn retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout_secs: u64) -> Self {
        self.config.timeout_secs = timeout_secs;
        self
    }

    /// Build a prompt task.
    pub fn prompt(self, prompt: impl Into<String>) -> PromptTask {
        PromptTask::new(self.config, prompt)
    }

    /// Get the configuration.
    pub fn build(self) -> AutomationConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_config() {
        let config = AutomationConfig::new("test-1", "Test Automation")
            .with_description("A test automation")
            .with_schedule(ScheduleConfig::interval(60))
            .with_retries(3)
            .with_timeout(120)
            .with_tag("test");

        assert_eq!(config.id, "test-1");
        assert_eq!(config.name, "Test Automation");
        assert_eq!(config.description, "A test automation");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 120);
        assert!(config.enabled);
    }

    #[test]
    fn test_schedule_config() {
        let once = ScheduleConfig::Once;
        assert!(once.to_schedule().is_ok());

        let interval = ScheduleConfig::interval(60);
        assert!(interval.to_schedule().is_ok());

        let cron = ScheduleConfig::cron("0 0 * * *");
        assert!(cron.to_schedule().is_ok());
    }

    #[test]
    fn test_task_context() {
        let mut ctx = TaskContext::new();

        ctx.set_input("key", serde_json::json!("value"));
        assert_eq!(ctx.get_input("key"), Some(&serde_json::json!("value")));

        ctx.set_state("state_key", serde_json::json!(123));
        assert_eq!(ctx.get_state("state_key"), Some(&serde_json::json!(123)));
    }

    #[test]
    fn test_task_output() {
        let success = TaskOutput::success("Done")
            .with_data(serde_json::json!({"count": 5}))
            .with_duration(100);

        assert!(success.success);
        assert_eq!(success.message, "Done");
        assert!(success.data.is_some());
        assert_eq!(success.duration_ms, 100);

        let failure = TaskOutput::failure("Error occurred");
        assert!(!failure.success);
    }

    #[test]
    fn test_automation_builder() {
        let config = AutomationBuilder::new("daily-task")
            .name("Daily Report")
            .description("Generate daily report")
            .every_hours(24)
            .retries(2)
            .timeout(600)
            .build();

        assert_eq!(config.id, "daily-task");
        assert_eq!(config.name, "Daily Report");
        assert!(matches!(
            config.schedule,
            ScheduleConfig::Interval { seconds: 86400 }
        ));
    }

    #[test]
    fn test_automation_builder_cron() {
        let config = AutomationBuilder::new("cron-task")
            .name("Cron Task")
            .cron("0 9 * * 1-5")
            .build();

        assert!(matches!(config.schedule, ScheduleConfig::Cron { .. }));
    }

    #[tokio::test]
    async fn test_automation_runner() {
        let runner = AutomationRunner::new();

        // Create a simple task
        struct TestTask {
            config: AutomationConfig,
        }

        #[async_trait]
        impl AutomationTask for TestTask {
            fn config(&self) -> &AutomationConfig {
                &self.config
            }

            async fn execute(
                &self,
                _client: &AsyncForgeClient,
                _context: &mut TaskContext,
            ) -> AutomationResult<TaskOutput> {
                Ok(TaskOutput::success("Test completed"))
            }
        }

        let task = TestTask {
            config: AutomationConfig::new("test", "Test"),
        };

        runner.register(task).await;

        let tasks = runner.list().await;
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains(&"test".to_string()));

        let enabled = runner.list_enabled().await;
        assert_eq!(enabled.len(), 1);
    }

    #[tokio::test]
    async fn test_automation_runner_unregister() {
        let runner = AutomationRunner::new();

        struct TestTask {
            config: AutomationConfig,
        }

        #[async_trait]
        impl AutomationTask for TestTask {
            fn config(&self) -> &AutomationConfig {
                &self.config
            }

            async fn execute(
                &self,
                _client: &AsyncForgeClient,
                _context: &mut TaskContext,
            ) -> AutomationResult<TaskOutput> {
                Ok(TaskOutput::success("Test"))
            }
        }

        let task = TestTask {
            config: AutomationConfig::new("removable", "Removable"),
        };

        runner.register(task).await;
        assert!(runner.get("removable").await.is_some());

        runner.unregister("removable").await;
        assert!(runner.get("removable").await.is_none());
    }

    #[test]
    fn test_execution_record() {
        let record = ExecutionRecord {
            execution_id: "exec-1".to_string(),
            task_id: "task-1".to_string(),
            started_at: 1000,
            ended_at: Some(1005),
            status: TaskStatus::Completed,
            output: Some(TaskOutput::success("Done")),
        };

        assert_eq!(record.status, TaskStatus::Completed);
        assert!(record.output.is_some());
    }
}
