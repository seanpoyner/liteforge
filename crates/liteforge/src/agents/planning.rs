//! Planning agent for multi-step task decomposition.

use super::context::{AgentContext, AgentState};
use super::step::{AgentStep, StepResult, StepType};
use super::traits::{Agent, AgentConfig, AgentError, AgentResult};
use crate::client::AsyncForgeClient;
use crate::tools::{ToolExecutor, ToolRegistry};
use crate::types::{ChatCompletionRequest, Message, ToolCall, ToolDefinition, ToolParameters};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Status of a plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlanStepStatus {
    /// Step is pending execution.
    #[default]
    Pending,
    /// Step is currently being executed.
    InProgress,
    /// Step completed successfully.
    Completed,
    /// Step failed.
    Failed,
    /// Step was skipped.
    Skipped,
}

/// A single step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique step ID.
    pub id: String,
    /// Step number (1-indexed).
    pub step_number: usize,
    /// Description of what this step does.
    pub description: String,
    /// Expected output or result.
    pub expected_output: Option<String>,
    /// Dependencies on other step IDs.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Current status.
    #[serde(default)]
    pub status: PlanStepStatus,
    /// Actual result/output.
    pub result: Option<String>,
    /// Tool to use for this step (if any).
    pub tool: Option<String>,
    /// Tool arguments (if using a tool).
    pub tool_args: Option<serde_json::Value>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl PlanStep {
    /// Create a new plan step.
    pub fn new(id: impl Into<String>, step_number: usize, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            step_number,
            description: description.into(),
            expected_output: None,
            dependencies: Vec::new(),
            status: PlanStepStatus::Pending,
            result: None,
            tool: None,
            tool_args: None,
            error: None,
        }
    }

    /// Set expected output.
    pub fn with_expected_output(mut self, output: impl Into<String>) -> Self {
        self.expected_output = Some(output.into());
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, step_id: impl Into<String>) -> Self {
        self.dependencies.push(step_id.into());
        self
    }

    /// Set the tool to use.
    pub fn with_tool(mut self, tool: impl Into<String>, args: serde_json::Value) -> Self {
        self.tool = Some(tool.into());
        self.tool_args = Some(args);
        self
    }

    /// Check if this step can be executed.
    pub fn can_execute(&self, completed_steps: &[String]) -> bool {
        self.status == PlanStepStatus::Pending
            && self
                .dependencies
                .iter()
                .all(|dep| completed_steps.contains(dep))
    }

    /// Mark as in progress.
    pub fn start(&mut self) {
        self.status = PlanStepStatus::InProgress;
    }

    /// Mark as completed with result.
    pub fn complete(&mut self, result: impl Into<String>) {
        self.status = PlanStepStatus::Completed;
        self.result = Some(result.into());
    }

    /// Mark as failed with error.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = PlanStepStatus::Failed;
        self.error = Some(error.into());
    }

    /// Mark as skipped.
    pub fn skip(&mut self) {
        self.status = PlanStepStatus::Skipped;
    }
}

/// A complete plan with multiple steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Plan ID.
    pub id: String,
    /// Plan name/title.
    pub name: String,
    /// Plan description.
    pub description: String,
    /// The original task/goal.
    pub goal: String,
    /// Steps in the plan.
    pub steps: Vec<PlanStep>,
    /// Whether the plan has been executed.
    #[serde(default)]
    pub executed: bool,
    /// Final result of the plan.
    pub final_result: Option<String>,
}

impl Plan {
    /// Create a new plan.
    pub fn new(id: impl Into<String>, name: impl Into<String>, goal: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            goal: goal.into(),
            steps: Vec::new(),
            executed: false,
            final_result: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a step.
    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    /// Get the next executable step.
    pub fn next_step(&self) -> Option<&PlanStep> {
        let completed: Vec<_> = self
            .steps
            .iter()
            .filter(|s| s.status == PlanStepStatus::Completed)
            .map(|s| s.id.clone())
            .collect();

        self.steps.iter().find(|s| s.can_execute(&completed))
    }

    /// Get a mutable reference to a step by ID.
    pub fn get_step_mut(&mut self, id: &str) -> Option<&mut PlanStep> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    /// Check if the plan is complete.
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(
                s.status,
                PlanStepStatus::Completed | PlanStepStatus::Skipped
            )
        })
    }

    /// Check if the plan has failed.
    pub fn has_failed(&self) -> bool {
        self.steps
            .iter()
            .any(|s| s.status == PlanStepStatus::Failed)
    }

    /// Get completion percentage.
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 100.0;
        }
        let done = self
            .steps
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    PlanStepStatus::Completed | PlanStepStatus::Skipped
                )
            })
            .count();
        (done as f32 / self.steps.len() as f32) * 100.0
    }

    /// Get a summary of step statuses.
    pub fn status_summary(&self) -> PlanStatusSummary {
        let mut summary = PlanStatusSummary::default();
        for step in &self.steps {
            match step.status {
                PlanStepStatus::Pending => summary.pending += 1,
                PlanStepStatus::InProgress => summary.in_progress += 1,
                PlanStepStatus::Completed => summary.completed += 1,
                PlanStepStatus::Failed => summary.failed += 1,
                PlanStepStatus::Skipped => summary.skipped += 1,
            }
        }
        summary.total = self.steps.len();
        summary
    }
}

/// Summary of plan step statuses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanStatusSummary {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Configuration for the planning agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningAgentConfig {
    /// Base agent configuration.
    #[serde(flatten)]
    pub agent: AgentConfig,
    /// Whether to auto-execute the plan after creation.
    #[serde(default)]
    pub auto_execute: bool,
    /// Maximum number of plan steps.
    #[serde(default = "default_max_plan_steps")]
    pub max_plan_steps: usize,
    /// Whether to allow plan revision during execution.
    #[serde(default = "default_true")]
    pub allow_revision: bool,
}

fn default_max_plan_steps() -> usize {
    20
}

fn default_true() -> bool {
    true
}

impl Default for PlanningAgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::new("planning-agent").with_system_prompt(DEFAULT_SYSTEM_PROMPT),
            auto_execute: false,
            max_plan_steps: default_max_plan_steps(),
            allow_revision: true,
        }
    }
}

impl PlanningAgentConfig {
    /// Create a new planning agent config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.agent.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.agent.model = Some(model.into());
        self
    }

    /// Enable auto-execution.
    pub fn with_auto_execute(mut self, auto_execute: bool) -> Self {
        self.auto_execute = auto_execute;
        self
    }

    /// Set maximum plan steps.
    pub fn with_max_plan_steps(mut self, max: usize) -> Self {
        self.max_plan_steps = max;
        self
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a planning assistant. When given a task:

1. Break it down into clear, actionable steps
2. Identify dependencies between steps
3. Use the create_plan tool to create a structured plan
4. Each step should have a clear description and expected output

Guidelines:
- Keep steps atomic and focused
- Identify what can be done in parallel
- Consider edge cases and potential failures
- Be specific about expected outputs"#;

/// An agent that creates and executes multi-step plans.
pub struct PlanningAgent {
    client: AsyncForgeClient,
    executor: Option<ToolExecutor>,
    config: PlanningAgentConfig,
    context: AgentContext,
    current_plan: Option<Plan>,
    plan_history: Vec<Plan>,
}

impl PlanningAgent {
    /// Create a new planning agent.
    pub fn new(client: AsyncForgeClient) -> Self {
        Self::with_config(client, PlanningAgentConfig::default())
    }

    /// Create a planning agent with configuration.
    pub fn with_config(client: AsyncForgeClient, config: PlanningAgentConfig) -> Self {
        let name = config.agent.name.clone();
        let max_steps = config.agent.max_steps;
        let mut context = AgentContext::new(&name);
        context.max_steps = max_steps;
        Self {
            client,
            executor: None,
            config,
            context,
            current_plan: None,
            plan_history: Vec::new(),
        }
    }

    /// Add tools for plan execution.
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.executor = Some(ToolExecutor::new(tools));
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.agent.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.agent.model = Some(model.into());
        self
    }

    /// Get the current plan.
    pub fn current_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    /// Get plan history.
    pub fn plan_history(&self) -> &[Plan] {
        &self.plan_history
    }

    /// Create a plan for a task.
    pub async fn create_plan(&mut self, task: &str) -> AgentResult<Plan> {
        let start = Instant::now();

        // Build messages
        let mut messages = Vec::new();
        if let Some(ref prompt) = self.config.agent.system_prompt {
            messages.push(Message::system(prompt));
        }
        messages.push(Message::user(format!(
            "Create a detailed plan to accomplish this task:\n\n{}",
            task
        )));

        let model = self
            .config
            .agent
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4".to_string());

        let mut request = ChatCompletionRequest::new(model, messages);
        request = request.tools(vec![Self::create_plan_tool()]);

        if let Some(temp) = self.config.agent.temperature {
            request = request.temperature(temp);
        }

        let response = self.client.chat_completions(request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| AgentError::Other("No response from LLM".to_string()))?;

        // Check for tool calls to create_plan
        if let Some(ref tool_calls) = choice.message.tool_calls {
            for tool_call in tool_calls {
                if tool_call.function.name == "create_plan" {
                    let plan = self.parse_plan_from_tool_call(tool_call, task)?;
                    self.current_plan = Some(plan.clone());
                    return Ok(plan);
                }
            }
        }

        // If no tool call, try to parse from content
        if let Some(ref content) = choice.message.content {
            // Create a simple single-step plan from the response
            let mut plan = Plan::new(
                format!("plan-{}", start.elapsed().as_millis()),
                "Generated Plan",
                task,
            );
            plan.description = content.clone();
            plan.add_step(PlanStep::new("step-1", 1, content.clone()));
            self.current_plan = Some(plan.clone());
            return Ok(plan);
        }

        Err(AgentError::Other("Failed to create plan".to_string()))
    }

    /// Execute the current plan.
    pub async fn execute_plan(&mut self) -> AgentResult<String> {
        if self.current_plan.is_none() {
            return Err(AgentError::Other("No plan to execute".to_string()));
        }

        loop {
            // Get the next step to execute
            let next_step = {
                let plan = self.current_plan.as_ref().unwrap();
                let completed: Vec<_> = plan
                    .steps
                    .iter()
                    .filter(|s| s.status == PlanStepStatus::Completed)
                    .map(|s| s.id.clone())
                    .collect();
                plan.steps
                    .iter()
                    .find(|s| s.can_execute(&completed))
                    .map(|s| s.id.clone())
            };

            let Some(step_id) = next_step else {
                break;
            };

            // Execute the step
            let step_result = self.execute_step(&step_id).await;

            // Update the step status
            let allow_revision = self.config.allow_revision;
            if let Some(plan) = &mut self.current_plan {
                if let Some(step) = plan.get_step_mut(&step_id) {
                    match step_result {
                        Ok(result) => step.complete(result),
                        Err(e) => {
                            step.fail(e.to_string());
                            if !allow_revision {
                                return Err(AgentError::Other(format!(
                                    "Plan execution failed at step {}: {}",
                                    step_id, e
                                )));
                            }
                        }
                    }
                }
            }
        }

        // Compile final result
        let plan = self
            .current_plan
            .as_mut()
            .ok_or_else(|| AgentError::Other("Plan disappeared".to_string()))?;

        let results: Vec<&str> = plan
            .steps
            .iter()
            .filter_map(|s| s.result.as_deref())
            .collect();

        let final_result = if results.is_empty() {
            "Plan executed but no results produced".to_string()
        } else {
            results.join("\n\n")
        };

        plan.executed = true;
        plan.final_result = Some(final_result.clone());

        // Archive the plan
        self.plan_history.push(plan.clone());

        Ok(final_result)
    }

    /// Execute a single step.
    async fn execute_step(&mut self, step_id: &str) -> AgentResult<String> {
        let plan = self
            .current_plan
            .as_ref()
            .ok_or_else(|| AgentError::Other("No plan".to_string()))?;

        let step = plan
            .steps
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| AgentError::Other(format!("Step {} not found", step_id)))?;

        // If step has a tool, execute it
        if let (Some(tool_name), Some(ref args)) = (&step.tool, &step.tool_args) {
            if let Some(ref executor) = self.executor {
                let call = ToolCall::new(
                    format!("{}-call", step_id),
                    tool_name.clone(),
                    args.to_string(),
                );
                let result = executor.execute_call(&call);
                return Ok(result.to_message_content());
            }
        }

        // Otherwise, use LLM to execute the step
        let mut messages = Vec::new();
        if let Some(ref prompt) = self.config.agent.system_prompt {
            messages.push(Message::system(prompt));
        }

        // Add context from completed steps
        let completed_context: Vec<_> = plan
            .steps
            .iter()
            .filter(|s| s.status == PlanStepStatus::Completed)
            .filter_map(|s| {
                s.result
                    .as_ref()
                    .map(|r| format!("Step {}: {}", s.step_number, r))
            })
            .collect();

        let context_str = if completed_context.is_empty() {
            String::new()
        } else {
            format!("\n\nCompleted steps:\n{}", completed_context.join("\n"))
        };

        messages.push(Message::user(format!(
            "Execute this step: {}\n\nExpected output: {}\n\nOriginal goal: {}{}",
            step.description,
            step.expected_output.as_deref().unwrap_or("Not specified"),
            plan.goal,
            context_str
        )));

        let model = self
            .config
            .agent
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4".to_string());

        let mut request = ChatCompletionRequest::new(model, messages);

        // Add available tools if we have them
        if let Some(ref executor) = self.executor {
            let tools = executor.registry().definitions();
            if !tools.is_empty() {
                request = request.tools(tools);
            }
        }

        if let Some(temp) = self.config.agent.temperature {
            request = request.temperature(temp);
        }

        let response = self.client.chat_completions(request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| AgentError::Other("No response from LLM".to_string()))?;

        // Handle tool calls
        if let Some(ref tool_calls) = choice.message.tool_calls {
            if let Some(ref executor) = self.executor {
                let mut results = Vec::new();
                for call in tool_calls {
                    let result = executor.execute_call(call);
                    results.push(result.to_message_content());
                }
                return Ok(results.join("\n"));
            }
        }

        // Return the content
        choice
            .message
            .content
            .clone()
            .ok_or_else(|| AgentError::Other("No content in response".to_string()))
    }

    fn create_plan_tool() -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "name".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Name of the plan"
            }),
        );
        properties.insert(
            "description".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Description of the overall plan"
            }),
        );
        properties.insert(
            "steps".to_string(),
            serde_json::json!({
                "type": "array",
                "description": "List of steps in the plan",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Unique step ID"},
                        "description": {"type": "string", "description": "What this step does"},
                        "expected_output": {"type": "string", "description": "Expected output"},
                        "dependencies": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "IDs of steps this depends on"
                        },
                        "tool": {"type": "string", "description": "Tool to use (optional)"},
                        "tool_args": {"type": "object", "description": "Tool arguments (optional)"}
                    },
                    "required": ["id", "description"]
                }
            }),
        );

        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::types::FunctionDefinition {
                name: "create_plan".to_string(),
                description: Some(
                    "Create a structured plan with multiple steps to accomplish a task".to_string(),
                ),
                parameters: Some(ToolParameters {
                    schema_type: "object".to_string(),
                    properties,
                    required: Some(vec![
                        "name".to_string(),
                        "description".to_string(),
                        "steps".to_string(),
                    ]),
                }),
            },
        }
    }

    fn parse_plan_from_tool_call(&self, tool_call: &ToolCall, goal: &str) -> AgentResult<Plan> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .map_err(|e| AgentError::Other(format!("Failed to parse plan arguments: {}", e)))?;

        let name = args["name"].as_str().unwrap_or("Unnamed Plan");
        let description = args["description"].as_str().unwrap_or("");

        let mut plan = Plan::new(format!("plan-{}", uuid_simple()), name, goal);
        plan.description = description.to_string();

        if let Some(steps) = args["steps"].as_array() {
            for (i, step_data) in steps.iter().enumerate() {
                let id = step_data["id"]
                    .as_str()
                    .unwrap_or(&format!("step-{}", i + 1))
                    .to_string();
                let desc = step_data["description"]
                    .as_str()
                    .unwrap_or("No description")
                    .to_string();

                let mut step = PlanStep::new(&id, i + 1, desc);

                if let Some(expected) = step_data["expected_output"].as_str() {
                    step.expected_output = Some(expected.to_string());
                }

                if let Some(deps) = step_data["dependencies"].as_array() {
                    step.dependencies = deps
                        .iter()
                        .filter_map(|d| d.as_str().map(String::from))
                        .collect();
                }

                if let Some(tool) = step_data["tool"].as_str() {
                    step.tool = Some(tool.to_string());
                    step.tool_args = step_data.get("tool_args").cloned();
                }

                plan.add_step(step);
            }
        }

        if plan.steps.len() > self.config.max_plan_steps {
            return Err(AgentError::ConfigError(format!(
                "Plan has {} steps, exceeding maximum of {}",
                plan.steps.len(),
                self.config.max_plan_steps
            )));
        }

        Ok(plan)
    }
}

/// Simple UUID generation (no external dependency).
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_secs(), now.subsec_nanos())
}

#[async_trait]
impl Agent for PlanningAgent {
    fn name(&self) -> &str {
        &self.config.agent.name
    }

    fn config(&self) -> &AgentConfig {
        &self.config.agent
    }

    fn context_mut(&mut self) -> &mut AgentContext {
        &mut self.context
    }

    fn context(&self) -> &AgentContext {
        &self.context
    }

    async fn step(&mut self) -> AgentResult<AgentStep> {
        let start = Instant::now();
        let step_number = self.context.current_step;

        // If we don't have a plan, create one from the last user message
        if self.current_plan.is_none() {
            let last_message = self
                .context
                .memory
                .messages()
                .iter()
                .rev()
                .find(|m| m.role == "user")
                .map(|m| m.content.clone().unwrap_or_default())
                .unwrap_or_default();

            let plan = self.create_plan(&last_message).await?;

            let step = AgentStep::new(step_number, StepType::Think)
                .with_result(StepResult::Continue)
                .with_duration(start.elapsed())
                .with_output(serde_json::to_value(&plan).unwrap_or_default());

            self.context.memory.add_message(Message::assistant(format!(
                "I've created a plan with {} steps:\n\n{}",
                plan.steps.len(),
                plan.steps
                    .iter()
                    .map(|s| format!("{}. {}", s.step_number, s.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            )));

            if self.config.auto_execute {
                return Ok(step);
            } else {
                self.context.state = AgentState::Completed;
                return Ok(AgentStep::new(step_number, StepType::Response)
                    .with_result(StepResult::Done {
                        response: format!(
                            "Plan created with {} steps. Use execute_plan() to run it.",
                            plan.steps.len()
                        ),
                    })
                    .with_duration(start.elapsed()));
            }
        }

        // Execute the next step in the plan
        if let Some(plan) = &self.current_plan {
            let completed: Vec<_> = plan
                .steps
                .iter()
                .filter(|s| s.status == PlanStepStatus::Completed)
                .map(|s| s.id.clone())
                .collect();

            if let Some(next) = plan.steps.iter().find(|s| s.can_execute(&completed)) {
                let step_id = next.id.clone();
                let step_desc = next.description.clone();

                let result = self.execute_step(&step_id).await;

                if let Some(plan) = &mut self.current_plan {
                    if let Some(step) = plan.get_step_mut(&step_id) {
                        match result {
                            Ok(ref r) => step.complete(r),
                            Err(ref e) => step.fail(e.to_string()),
                        }
                    }
                }

                match result {
                    Ok(result_str) => {
                        self.context.memory.add_message(Message::assistant(format!(
                            "Completed step: {}\nResult: {}",
                            step_desc, result_str
                        )));

                        return Ok(AgentStep::new(
                            step_number,
                            StepType::ToolCall {
                                tool_name: "plan_step".to_string(),
                                call_id: step_id,
                            },
                        )
                        .with_result(StepResult::Continue)
                        .with_duration(start.elapsed()));
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

        // Plan is complete
        let final_result = if let Some(plan) = &self.current_plan {
            let results: Vec<_> = plan
                .steps
                .iter()
                .filter_map(|s| s.result.as_ref().map(|r| format!("- {}", r)))
                .collect();
            if results.is_empty() {
                "Plan completed with no results".to_string()
            } else {
                format!("Plan completed:\n{}", results.join("\n"))
            }
        } else {
            "No plan to complete".to_string()
        };

        self.context.state = AgentState::Completed;

        Ok(AgentStep::new(step_number, StepType::Response)
            .with_result(StepResult::Done {
                response: final_result,
            })
            .with_duration(start.elapsed()))
    }

    fn reset(&mut self) {
        self.context.reset();
        if let Some(plan) = self.current_plan.take() {
            self.plan_history.push(plan);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_step_new() {
        let step = PlanStep::new("step-1", 1, "Do something");
        assert_eq!(step.id, "step-1");
        assert_eq!(step.step_number, 1);
        assert_eq!(step.description, "Do something");
        assert_eq!(step.status, PlanStepStatus::Pending);
    }

    #[test]
    fn test_plan_step_lifecycle() {
        let mut step = PlanStep::new("step-1", 1, "Test step");

        assert!(step.can_execute(&[]));

        step.start();
        assert_eq!(step.status, PlanStepStatus::InProgress);

        step.complete("Done!");
        assert_eq!(step.status, PlanStepStatus::Completed);
        assert_eq!(step.result, Some("Done!".to_string()));
    }

    #[test]
    fn test_plan_step_dependencies() {
        let step = PlanStep::new("step-2", 2, "Step with deps").with_dependency("step-1");

        assert!(!step.can_execute(&[]));
        assert!(step.can_execute(&["step-1".to_string()]));
    }

    #[test]
    fn test_plan_new() {
        let plan = Plan::new("plan-1", "Test Plan", "Do something complex");
        assert_eq!(plan.id, "plan-1");
        assert_eq!(plan.name, "Test Plan");
        assert_eq!(plan.goal, "Do something complex");
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = Plan::new("plan-1", "Test", "Goal");

        plan.add_step(PlanStep::new("step-1", 1, "First"));
        plan.add_step(PlanStep::new("step-2", 2, "Second"));

        assert_eq!(plan.progress(), 0.0);

        if let Some(step) = plan.get_step_mut("step-1") {
            step.complete("Done");
        }

        assert_eq!(plan.progress(), 50.0);

        if let Some(step) = plan.get_step_mut("step-2") {
            step.complete("Done too");
        }

        assert_eq!(plan.progress(), 100.0);
        assert!(plan.is_complete());
    }

    #[test]
    fn test_plan_next_step() {
        let mut plan = Plan::new("plan-1", "Test", "Goal");

        let step1 = PlanStep::new("step-1", 1, "First");
        let step2 = PlanStep::new("step-2", 2, "Second").with_dependency("step-1");

        plan.add_step(step1);
        plan.add_step(step2);

        // First step should be available
        let next = plan.next_step();
        assert_eq!(next.map(|s| s.id.as_str()), Some("step-1"));

        // Complete first step
        if let Some(step) = plan.get_step_mut("step-1") {
            step.complete("Done");
        }

        // Now second step should be available
        let next = plan.next_step();
        assert_eq!(next.map(|s| s.id.as_str()), Some("step-2"));
    }

    #[test]
    fn test_plan_status_summary() {
        let mut plan = Plan::new("plan-1", "Test", "Goal");

        plan.add_step(PlanStep::new("step-1", 1, "First"));
        plan.add_step(PlanStep::new("step-2", 2, "Second"));
        plan.add_step(PlanStep::new("step-3", 3, "Third"));

        if let Some(step) = plan.get_step_mut("step-1") {
            step.complete("Done");
        }
        if let Some(step) = plan.get_step_mut("step-2") {
            step.fail("Error");
        }

        let summary = plan.status_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.pending, 1);
    }

    #[test]
    fn test_planning_agent_config() {
        let config = PlanningAgentConfig::new()
            .with_model("gpt-4")
            .with_auto_execute(true)
            .with_max_plan_steps(10);

        assert_eq!(config.agent.model, Some("gpt-4".to_string()));
        assert!(config.auto_execute);
        assert_eq!(config.max_plan_steps, 10);
    }

    #[test]
    fn test_planning_agent_creation() {
        let client = AsyncForgeClient::new();
        let agent = PlanningAgent::new(client)
            .with_model("gpt-4")
            .with_system_prompt("Custom prompt");

        assert!(agent.current_plan().is_none());
        assert!(agent.plan_history().is_empty());
    }
}
