//! Agent system for Forge CLI.

pub mod config;
pub mod execution;

pub use config::{discover_agents, AgentConfig, ToolConfig, ToolType};
