//! Agent framework for building LLM-powered agents.
//!
//! This module provides the core abstractions for building agents that can:
//! - Execute multi-step tasks with tool calls
//! - Maintain conversation history and state
//! - Handle errors and retries
//!
//! # Example
//!
//! ```no_run
//! use liteforge::agents::{Agent, AgentConfig, ToolCallingAgent};
//! use liteforge::{AsyncForgeClient, ToolRegistry};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = AsyncForgeClient::new();
//!     let tools = ToolRegistry::new();
//!
//!     let mut agent = ToolCallingAgent::new(client, tools)
//!         .with_system_prompt("You are a helpful assistant.");
//!
//!     let result = agent.run("What is 2 + 2?").await;
//! }
//! ```

mod code_agent;
mod context;
mod planning;
pub mod sandbox;
mod step;
mod tool_calling;
mod traits;

pub use code_agent::{CodeAgent, CodeAgentConfig, CodeExecution};
pub use context::{AgentContext, AgentMemory, AgentState};
pub use planning::{
    Plan, PlanStatusSummary, PlanStep, PlanStepStatus, PlanningAgent, PlanningAgentConfig,
};
pub use sandbox::{
    ExecutionResult, Language, MockSandbox, OutputFile, ProcessSandbox, Sandbox, SandboxConfig,
    SandboxError, SandboxErrorKind,
};
pub use step::{AgentStep, StepResult, StepType};
pub use tool_calling::ToolCallingAgent;
pub use traits::{Agent, AgentConfig, AgentError};
