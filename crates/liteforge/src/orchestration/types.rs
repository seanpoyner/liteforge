//! Orchestration types.
//!
//! Core types for multi-agent orchestration and workflow management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An intent represents a user's goal that should be routed to an appropriate agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Intent name/category.
    pub name: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Extracted entities from the input.
    #[serde(default)]
    pub entities: HashMap<String, String>,
    /// Original input text.
    pub input: String,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Intent {
    /// Create a new intent.
    pub fn new(name: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            confidence: 1.0,
            entities: HashMap::new(),
            input: input.into(),
            metadata: HashMap::new(),
        }
    }

    /// Set the confidence score.
    pub fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add an entity.
    pub fn entity(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entities.insert(key.into(), value.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// A step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step identifier.
    pub id: String,
    /// Step name.
    pub name: String,
    /// Agent to execute this step.
    pub agent: String,
    /// Input for the step (can reference previous step outputs).
    pub input: serde_json::Value,
    /// Dependencies on other steps (step IDs that must complete first).
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Condition for executing this step (optional).
    pub condition: Option<String>,
    /// Timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Number of retries on failure.
    pub retries: Option<u32>,
}

impl WorkflowStep {
    /// Create a new workflow step.
    pub fn new(id: impl Into<String>, name: impl Into<String>, agent: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            agent: agent.into(),
            input: serde_json::Value::Null,
            depends_on: Vec::new(),
            condition: None,
            timeout_secs: None,
            retries: None,
        }
    }

    /// Set the input for this step.
    pub fn input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    /// Add a dependency on another step.
    pub fn depends_on(mut self, step_id: impl Into<String>) -> Self {
        self.depends_on.push(step_id.into());
        self
    }

    /// Set a condition for execution.
    pub fn condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Set timeout.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set retry count.
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }
}

/// A workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow identifier.
    pub id: String,
    /// Workflow name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Steps in the workflow.
    pub steps: Vec<WorkflowStep>,
    /// Global timeout for the entire workflow.
    pub timeout_secs: Option<u64>,
    /// Maximum parallel step executions.
    pub max_parallel: Option<usize>,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            steps: Vec::new(),
            timeout_secs: None,
            max_parallel: None,
        }
    }

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a step.
    pub fn step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set global timeout.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set max parallel executions.
    pub fn max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = Some(max);
        self
    }
}

/// Status of a step execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step is pending execution.
    Pending,
    /// Step is currently running.
    Running,
    /// Step completed successfully.
    Completed,
    /// Step failed.
    Failed,
    /// Step was skipped (condition not met).
    Skipped,
    /// Step was cancelled.
    Cancelled,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::Running => write!(f, "running"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Failed => write!(f, "failed"),
            StepStatus::Skipped => write!(f, "skipped"),
            StepStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Result of a single step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    /// Step ID.
    pub step_id: String,
    /// Execution status.
    pub status: StepStatus,
    /// Output from the step.
    pub output: Option<serde_json::Value>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// Number of retries attempted.
    pub retries: u32,
}

impl StepExecutionResult {
    /// Create a successful result.
    pub fn success(
        step_id: impl Into<String>,
        output: serde_json::Value,
        duration_ms: u64,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            status: StepStatus::Completed,
            output: Some(output),
            error: None,
            duration_ms,
            retries: 0,
        }
    }

    /// Create a failed result.
    pub fn failure(step_id: impl Into<String>, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.into(),
            status: StepStatus::Failed,
            output: None,
            error: Some(error.into()),
            duration_ms,
            retries: 0,
        }
    }

    /// Create a skipped result.
    pub fn skipped(step_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            status: StepStatus::Skipped,
            output: None,
            error: None,
            duration_ms: 0,
            retries: 0,
        }
    }
}

/// Result of a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow ID.
    pub workflow_id: String,
    /// Overall status.
    pub status: StepStatus,
    /// Results for each step.
    pub step_results: Vec<StepExecutionResult>,
    /// Final output (from the last successful step).
    pub output: Option<serde_json::Value>,
    /// Total execution time in milliseconds.
    pub duration_ms: u64,
}

impl WorkflowResult {
    /// Create a new workflow result.
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            status: StepStatus::Pending,
            step_results: Vec::new(),
            output: None,
            duration_ms: 0,
        }
    }

    /// Check if the workflow completed successfully.
    pub fn is_success(&self) -> bool {
        self.status == StepStatus::Completed
    }

    /// Get the result of a specific step.
    pub fn get_step_result(&self, step_id: &str) -> Option<&StepExecutionResult> {
        self.step_results.iter().find(|r| r.step_id == step_id)
    }
}

/// A session for maintaining state across agent interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session identifier.
    pub id: String,
    /// User identifier.
    pub user_id: Option<String>,
    /// Session state/variables.
    #[serde(default)]
    pub state: HashMap<String, serde_json::Value>,
    /// Conversation history.
    #[serde(default)]
    pub history: Vec<SessionMessage>,
    /// Session metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp (Unix epoch seconds).
    pub created_at: i64,
    /// Last update timestamp.
    pub updated_at: i64,
    /// Session expiry timestamp (optional).
    pub expires_at: Option<i64>,
}

impl Session {
    /// Create a new session.
    pub fn new(id: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            id: id.into(),
            user_id: None,
            state: HashMap::new(),
            history: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    /// Set the user ID.
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set a state variable.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
        self.touch();
    }

    /// Get a state variable.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Add a message to history.
    pub fn add_message(&mut self, message: SessionMessage) {
        self.history.push(message);
        self.touch();
    }

    /// Update the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }

    /// Set expiry time.
    pub fn expires_in_secs(mut self, secs: i64) -> Self {
        self.expires_at = Some(self.created_at + secs);
        self
    }

    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            now >= expires_at
        } else {
            false
        }
    }
}

/// A message in the session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message role (user, assistant, system).
    pub role: String,
    /// Message content.
    pub content: String,
    /// Agent that processed this message (if any).
    pub agent: Option<String>,
    /// Timestamp.
    pub timestamp: i64,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionMessage {
    /// Create a new session message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            agent: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            metadata: HashMap::new(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>, agent: Option<String>) -> Self {
        let mut msg = Self::new("assistant", content);
        msg.agent = agent;
        msg
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }
}

/// Routing decision from intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// The selected agent name.
    pub agent: String,
    /// The matched intent.
    pub intent: Intent,
    /// Confidence in this routing decision.
    pub confidence: f32,
    /// Alternative agents that could handle this intent.
    pub alternatives: Vec<(String, f32)>,
}

impl RoutingDecision {
    /// Create a new routing decision.
    pub fn new(agent: impl Into<String>, intent: Intent) -> Self {
        let confidence = intent.confidence;
        Self {
            agent: agent.into(),
            intent,
            confidence,
            alternatives: Vec::new(),
        }
    }

    /// Add an alternative agent.
    pub fn alternative(mut self, agent: impl Into<String>, confidence: f32) -> Self {
        self.alternatives.push((agent.into(), confidence));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_creation() {
        let intent = Intent::new("greeting", "Hello there!")
            .confidence(0.95)
            .entity("name", "Alice");

        assert_eq!(intent.name, "greeting");
        assert_eq!(intent.confidence, 0.95);
        assert_eq!(intent.entities.get("name"), Some(&"Alice".to_string()));
    }

    #[test]
    fn test_workflow_step() {
        let step = WorkflowStep::new("step1", "Analyze", "analyzer_agent")
            .input(serde_json::json!({"query": "test"}))
            .depends_on("step0")
            .timeout_secs(30)
            .retries(2);

        assert_eq!(step.id, "step1");
        assert_eq!(step.agent, "analyzer_agent");
        assert_eq!(step.depends_on, vec!["step0"]);
        assert_eq!(step.timeout_secs, Some(30));
        assert_eq!(step.retries, Some(2));
    }

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("wf1", "Test Workflow")
            .description("A test workflow")
            .step(WorkflowStep::new("s1", "Step 1", "agent1"))
            .step(WorkflowStep::new("s2", "Step 2", "agent2").depends_on("s1"))
            .max_parallel(2);

        assert_eq!(workflow.steps.len(), 2);
        assert_eq!(workflow.max_parallel, Some(2));
    }

    #[test]
    fn test_session() {
        let mut session = Session::new("sess123")
            .user_id("user456")
            .expires_in_secs(3600);

        session.set("counter", serde_json::json!(0));
        session.add_message(SessionMessage::user("Hello"));

        assert_eq!(session.id, "sess123");
        assert_eq!(session.user_id, Some("user456".to_string()));
        assert_eq!(session.get("counter"), Some(&serde_json::json!(0)));
        assert_eq!(session.history.len(), 1);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_step_execution_result() {
        let success =
            StepExecutionResult::success("step1", serde_json::json!({"result": "ok"}), 100);
        assert_eq!(success.status, StepStatus::Completed);
        assert!(success.output.is_some());

        let failure = StepExecutionResult::failure("step2", "Something went wrong", 50);
        assert_eq!(failure.status, StepStatus::Failed);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_routing_decision() {
        let intent = Intent::new("question", "What's the weather?").confidence(0.9);
        let decision =
            RoutingDecision::new("weather_agent", intent).alternative("general_agent", 0.6);

        assert_eq!(decision.agent, "weather_agent");
        assert_eq!(decision.confidence, 0.9);
        assert_eq!(decision.alternatives.len(), 1);
    }
}
