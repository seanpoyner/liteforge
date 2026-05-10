//! Hook system for intercepting and modifying behavior.
//!
//! Hooks allow you to intercept events at key points in the SDK's
//! execution flow, such as before/after tool calls or LLM requests.
//!
//! # Example
//!
//! ```
//! use liteforge::hooks::{Hook, HookContext, HookResult, HookManager};
//!
//! // Create a logging hook
//! struct LoggingHook;
//!
//! impl Hook for LoggingHook {
//!     fn name(&self) -> &str { "logging" }
//!
//!     fn on_before_tool_call(&self, ctx: &HookContext) -> HookResult {
//!         println!("Tool called: {:?}", ctx.data);
//!         HookResult::Continue
//!     }
//! }
//!
//! let manager = HookManager::new();
//! manager.register(Box::new(LoggingHook));
//! ```

mod manager;
mod types;

pub use manager::HookManager;
pub use types::{Hook, HookContext, HookEvent, HookResult};
