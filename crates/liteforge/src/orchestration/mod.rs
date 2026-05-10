//! Multi-agent orchestration support.
//!
//! This module provides tools for coordinating multiple AI agents,
//! routing requests based on intent, and managing multi-step workflows.
//!
//! # Overview
//!
//! The orchestration module includes:
//! - **Intent routing**: Classify user intents and route to appropriate agents
//! - **Session management**: Maintain state across interactions
//! - **Workflow execution**: Run multi-step workflows with dependencies
//! - **Agent coordination**: Orchestrate multiple agents with different strategies
//!
//! # Example
//!
//! ```no_run
//! use liteforge::orchestration::{
//!     AgentOrchestrator, OrchestratorConfig, OrchestrationStrategy,
//!     IntentRouter, IntentRoute, CommonIntents,
//! };
//! use std::sync::Arc;
//!
//! # async fn example() {
//! // Create orchestrator with intent-based routing
//! let config = OrchestratorConfig::new()
//!     .strategy(OrchestrationStrategy::IntentBased)
//!     .default_agent("general");
//!
//! let orchestrator = AgentOrchestrator::new(config);
//!
//! // Register agents with routes
//! // orchestrator.register_agent_with_route(agent, route).await;
//!
//! // Process messages
//! // let result = orchestrator.process("session-123", "Hello!").await;
//! # }
//! ```
//!
//! # Workflow Example
//!
//! ```rust
//! use liteforge::orchestration::{
//!     Workflow, WorkflowStep, WorkflowExecutor, EchoExecutor,
//! };
//! use std::sync::Arc;
//!
//! # async fn example() {
//! // Define a workflow
//! let workflow = Workflow::new("analysis", "Data Analysis")
//!     .step(WorkflowStep::new("fetch", "Fetch Data", "data_agent"))
//!     .step(WorkflowStep::new("analyze", "Analyze", "analysis_agent")
//!         .depends_on("fetch"))
//!     .step(WorkflowStep::new("report", "Generate Report", "report_agent")
//!         .depends_on("analyze"));
//!
//! // Create executor with agents
//! let executor = WorkflowExecutor::new()
//!     .register(Arc::new(EchoExecutor::new("data_agent")))
//!     .register(Arc::new(EchoExecutor::new("analysis_agent")))
//!     .register(Arc::new(EchoExecutor::new("report_agent")));
//!
//! // Execute workflow
//! let result = executor.execute(&workflow).await;
//! # }
//! ```

mod intent;
mod orchestrator;
mod session;
mod types;
mod workflow;

pub use intent::{CommonIntents, IntentRoute, IntentRouter};
pub use orchestrator::{
    AgentOrchestrator, OrchestratedAgent, OrchestrationResult, OrchestrationStrategy,
    OrchestratorConfig, OrchestratorError, ToolCallingAgentWrapper,
};
pub use session::{get_or_create, SessionStore};
pub use types::{
    Intent, RoutingDecision, Session, SessionMessage, StepExecutionResult, StepStatus, Workflow,
    WorkflowResult, WorkflowStep,
};
pub use workflow::{EchoExecutor, ExecutionContext, StepExecutor, WorkflowError, WorkflowExecutor};
