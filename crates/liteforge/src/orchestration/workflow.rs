//! Workflow execution for multi-agent orchestration.
//!
//! Executes multi-step workflows with dependency resolution and parallel execution.

use super::types::{StepExecutionResult, StepStatus, Workflow, WorkflowResult, WorkflowStep};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Semaphore};

/// Error types for workflow execution.
#[derive(Debug, Clone)]
pub enum WorkflowError {
    /// A step failed.
    StepFailed { step_id: String, error: String },
    /// A dependency was not met.
    DependencyNotMet { step_id: String, dependency: String },
    /// A cycle was detected in the workflow.
    CycleDetected { step_id: String },
    /// The workflow timed out.
    Timeout,
    /// No executor available for the step's agent.
    NoExecutor { agent: String },
    /// Invalid workflow definition.
    InvalidWorkflow(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::StepFailed { step_id, error } => {
                write!(f, "Step '{}' failed: {}", step_id, error)
            }
            WorkflowError::DependencyNotMet {
                step_id,
                dependency,
            } => {
                write!(f, "Step '{}' dependency '{}' not met", step_id, dependency)
            }
            WorkflowError::CycleDetected { step_id } => {
                write!(f, "Cycle detected at step '{}'", step_id)
            }
            WorkflowError::Timeout => write!(f, "Workflow timed out"),
            WorkflowError::NoExecutor { agent } => {
                write!(f, "No executor for agent '{}'", agent)
            }
            WorkflowError::InvalidWorkflow(msg) => write!(f, "Invalid workflow: {}", msg),
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Result type for workflow operations.
pub type WorkflowResult2 = Result<WorkflowResult, WorkflowError>;

/// Trait for executing workflow steps.
#[async_trait]
pub trait StepExecutor: Send + Sync {
    /// Execute a step and return the result.
    async fn execute(
        &self,
        step: &WorkflowStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, String>;

    /// Get the agent name this executor handles.
    fn agent(&self) -> &str;
}

/// Context available during step execution.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Results from completed steps.
    pub step_outputs: HashMap<String, serde_json::Value>,
    /// Workflow variables.
    pub variables: HashMap<String, serde_json::Value>,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new() -> Self {
        Self {
            step_outputs: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// Get the output of a completed step.
    pub fn get_output(&self, step_id: &str) -> Option<&serde_json::Value> {
        self.step_outputs.get(step_id)
    }

    /// Set a variable.
    pub fn set_variable(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(key.into(), value);
    }

    /// Get a variable.
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow executor that runs multi-step workflows.
pub struct WorkflowExecutor {
    executors: HashMap<String, Arc<dyn StepExecutor>>,
    max_parallel: usize,
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowExecutor {
    /// Create a new workflow executor.
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            max_parallel: 4,
        }
    }

    /// Set maximum parallel step executions.
    pub fn max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = max.max(1);
        self
    }

    /// Register a step executor.
    pub fn register(mut self, executor: Arc<dyn StepExecutor>) -> Self {
        self.executors
            .insert(executor.agent().to_string(), executor);
        self
    }

    /// Execute a workflow.
    pub async fn execute(&self, workflow: &Workflow) -> WorkflowResult2 {
        let start = Instant::now();

        // Validate workflow
        self.validate_workflow(workflow)?;

        let max_parallel = workflow.max_parallel.unwrap_or(self.max_parallel);
        let semaphore = Arc::new(Semaphore::new(max_parallel));

        let context = Arc::new(Mutex::new(ExecutionContext::new()));
        let results = Arc::new(Mutex::new(WorkflowResult::new(&workflow.id)));
        let completed = Arc::new(Mutex::new(HashSet::new()));
        let failed = Arc::new(Mutex::new(false));

        // Build dependency graph
        let step_map: HashMap<String, &WorkflowStep> =
            workflow.steps.iter().map(|s| (s.id.clone(), s)).collect();

        // Execute steps respecting dependencies
        let mut pending: Vec<&WorkflowStep> = workflow.steps.iter().collect();

        while !pending.is_empty() && !*failed.lock().await {
            // Find steps that can be executed (all dependencies met)
            let completed_ids = completed.lock().await.clone();
            let ready: Vec<_> = pending
                .iter()
                .filter(|s| s.depends_on.iter().all(|d| completed_ids.contains(d)))
                .cloned()
                .collect();

            if ready.is_empty() && !pending.is_empty() {
                // Check for unmet dependencies
                for step in &pending {
                    for dep in &step.depends_on {
                        if !step_map.contains_key(dep) {
                            return Err(WorkflowError::DependencyNotMet {
                                step_id: step.id.clone(),
                                dependency: dep.clone(),
                            });
                        }
                    }
                }
                // This shouldn't happen with a valid workflow
                break;
            }

            // Remove ready steps from pending
            pending.retain(|s| !ready.iter().any(|r| r.id == s.id));

            // Execute ready steps in parallel
            let mut handles = Vec::new();
            for step in ready {
                let semaphore = semaphore.clone();
                let context = context.clone();
                let results = results.clone();
                let completed = completed.clone();
                let failed = failed.clone();
                let step = step.clone();
                let executor = self.executors.get(&step.agent).cloned();

                let handle = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;

                    if *failed.lock().await {
                        return;
                    }

                    let step_start = Instant::now();

                    // Check condition
                    let should_execute = step.condition.is_none(); // Simplified
                    if !should_execute {
                        let result = StepExecutionResult::skipped(&step.id);
                        results.lock().await.step_results.push(result);
                        completed.lock().await.insert(step.id.clone());
                        return;
                    }

                    // Execute step
                    let result = match executor {
                        Some(exec) => {
                            let ctx = context.lock().await.clone();
                            match exec.execute(&step, &ctx).await {
                                Ok(output) => {
                                    let duration = step_start.elapsed().as_millis() as u64;
                                    context
                                        .lock()
                                        .await
                                        .step_outputs
                                        .insert(step.id.clone(), output.clone());
                                    StepExecutionResult::success(&step.id, output, duration)
                                }
                                Err(error) => {
                                    let duration = step_start.elapsed().as_millis() as u64;
                                    *failed.lock().await = true;
                                    StepExecutionResult::failure(&step.id, error, duration)
                                }
                            }
                        }
                        None => {
                            *failed.lock().await = true;
                            StepExecutionResult::failure(
                                &step.id,
                                format!("No executor for agent '{}'", step.agent),
                                0,
                            )
                        }
                    };

                    results.lock().await.step_results.push(result);
                    completed.lock().await.insert(step.id.clone());
                });

                handles.push(handle);
            }

            // Wait for all concurrent steps
            for handle in handles {
                let _ = handle.await;
            }

            // Check for timeout
            if let Some(timeout) = workflow.timeout_secs {
                if start.elapsed().as_secs() >= timeout {
                    return Err(WorkflowError::Timeout);
                }
            }
        }

        // Finalize result
        let mut final_result = results.lock().await.clone();
        final_result.duration_ms = start.elapsed().as_millis() as u64;

        // Determine final status
        let has_failed = final_result
            .step_results
            .iter()
            .any(|r| r.status == StepStatus::Failed);
        final_result.status = if has_failed {
            StepStatus::Failed
        } else {
            StepStatus::Completed
        };

        // Set output from last completed step
        if let Some(last) = final_result
            .step_results
            .iter()
            .rev()
            .find(|r| r.status == StepStatus::Completed)
        {
            final_result.output = last.output.clone();
        }

        Ok(final_result)
    }

    /// Validate a workflow definition.
    fn validate_workflow(&self, workflow: &Workflow) -> Result<(), WorkflowError> {
        if workflow.steps.is_empty() {
            return Err(WorkflowError::InvalidWorkflow("No steps defined".into()));
        }

        let step_ids: HashSet<_> = workflow.steps.iter().map(|s| &s.id).collect();

        // Check for duplicate IDs
        if step_ids.len() != workflow.steps.len() {
            return Err(WorkflowError::InvalidWorkflow("Duplicate step IDs".into()));
        }

        // Check for invalid dependencies
        for step in &workflow.steps {
            for dep in &step.depends_on {
                if !step_ids.contains(dep) {
                    return Err(WorkflowError::DependencyNotMet {
                        step_id: step.id.clone(),
                        dependency: dep.clone(),
                    });
                }
                if dep == &step.id {
                    return Err(WorkflowError::CycleDetected {
                        step_id: step.id.clone(),
                    });
                }
            }
        }

        // Check for cycles using DFS
        self.detect_cycles(workflow)?;

        Ok(())
    }

    /// Detect cycles in the workflow graph.
    fn detect_cycles(&self, workflow: &Workflow) -> Result<(), WorkflowError> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for step in &workflow.steps {
            if !visited.contains(&step.id)
                && self.has_cycle(step, workflow, &mut visited, &mut in_stack)
            {
                return Err(WorkflowError::CycleDetected {
                    step_id: step.id.clone(),
                });
            }
        }

        Ok(())
    }

    /// DFS helper to detect cycles.
    fn has_cycle(
        &self,
        step: &WorkflowStep,
        workflow: &Workflow,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(step.id.clone());
        in_stack.insert(step.id.clone());

        // Find steps that depend on this step
        for other in &workflow.steps {
            if other.depends_on.contains(&step.id) {
                if !visited.contains(&other.id) {
                    if self.has_cycle(other, workflow, visited, in_stack) {
                        return true;
                    }
                } else if in_stack.contains(&other.id) {
                    return true;
                }
            }
        }

        in_stack.remove(&step.id);
        false
    }
}

impl std::fmt::Debug for WorkflowExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowExecutor")
            .field("executors", &self.executors.keys().collect::<Vec<_>>())
            .field("max_parallel", &self.max_parallel)
            .finish()
    }
}

/// A simple step executor that just returns the input.
pub struct EchoExecutor {
    agent: String,
}

impl EchoExecutor {
    /// Create a new echo executor.
    pub fn new(agent: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
        }
    }
}

#[async_trait]
impl StepExecutor for EchoExecutor {
    async fn execute(
        &self,
        step: &WorkflowStep,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, String> {
        Ok(step.input.clone())
    }

    fn agent(&self) -> &str {
        &self.agent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_executor_simple() {
        let executor = WorkflowExecutor::new().register(Arc::new(EchoExecutor::new("echo")));

        let workflow = Workflow::new("test", "Test Workflow")
            .step(WorkflowStep::new("s1", "Step 1", "echo").input(serde_json::json!({"value": 1})))
            .step(
                WorkflowStep::new("s2", "Step 2", "echo")
                    .input(serde_json::json!({"value": 2}))
                    .depends_on("s1"),
            );

        let result = executor.execute(&workflow).await.unwrap();

        assert_eq!(result.status, StepStatus::Completed);
        assert_eq!(result.step_results.len(), 2);
    }

    #[tokio::test]
    async fn test_workflow_executor_parallel() {
        let executor = WorkflowExecutor::new()
            .max_parallel(4)
            .register(Arc::new(EchoExecutor::new("echo")));

        let workflow = Workflow::new("test", "Test Workflow")
            .step(WorkflowStep::new("s1", "Step 1", "echo").input(serde_json::json!(1)))
            .step(WorkflowStep::new("s2", "Step 2", "echo").input(serde_json::json!(2)))
            .step(WorkflowStep::new("s3", "Step 3", "echo").input(serde_json::json!(3)))
            .step(
                WorkflowStep::new("s4", "Final", "echo")
                    .input(serde_json::json!(4))
                    .depends_on("s1")
                    .depends_on("s2")
                    .depends_on("s3"),
            );

        let result = executor.execute(&workflow).await.unwrap();

        assert_eq!(result.status, StepStatus::Completed);
        assert_eq!(result.step_results.len(), 4);
    }

    #[tokio::test]
    async fn test_workflow_validation_duplicate_ids() {
        let executor = WorkflowExecutor::new();

        let workflow = Workflow::new("test", "Test")
            .step(WorkflowStep::new("s1", "Step 1", "agent"))
            .step(WorkflowStep::new("s1", "Step 2", "agent"));

        let result = executor.execute(&workflow).await;
        assert!(matches!(result, Err(WorkflowError::InvalidWorkflow(_))));
    }

    #[tokio::test]
    async fn test_workflow_validation_invalid_dependency() {
        let executor = WorkflowExecutor::new().register(Arc::new(EchoExecutor::new("echo")));

        let workflow = Workflow::new("test", "Test")
            .step(WorkflowStep::new("s1", "Step 1", "echo").depends_on("nonexistent"));

        let result = executor.execute(&workflow).await;
        assert!(matches!(
            result,
            Err(WorkflowError::DependencyNotMet { .. })
        ));
    }

    #[tokio::test]
    async fn test_workflow_validation_self_dependency() {
        let executor = WorkflowExecutor::new();

        let workflow = Workflow::new("test", "Test")
            .step(WorkflowStep::new("s1", "Step 1", "agent").depends_on("s1"));

        let result = executor.execute(&workflow).await;
        assert!(matches!(result, Err(WorkflowError::CycleDetected { .. })));
    }

    #[tokio::test]
    async fn test_execution_context() {
        let mut ctx = ExecutionContext::new();
        ctx.step_outputs
            .insert("s1".into(), serde_json::json!({"result": "ok"}));
        ctx.set_variable("count", serde_json::json!(42));

        assert_eq!(
            ctx.get_output("s1"),
            Some(&serde_json::json!({"result": "ok"}))
        );
        assert_eq!(ctx.get_variable("count"), Some(&serde_json::json!(42)));
    }
}
