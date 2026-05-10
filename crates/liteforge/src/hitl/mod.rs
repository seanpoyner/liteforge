//! Human-in-the-Loop (HITL) support for agent approval workflows.
//!
//! This module provides mechanisms for requesting human approval before
//! agents execute sensitive operations.
//!
//! # Overview
//!
//! - **ApprovalRequest**: A request for human approval
//! - **ApprovalResult**: The result of an approval request
//! - **ApprovalHandler**: Trait for handling approval requests
//! - **AutoApprovalHandler**: Always approves (for testing)
//! - **QueueApprovalHandler**: Queues requests for async processing
//!
//! # Example
//!
//! ```rust
//! use liteforge::hitl::{ApprovalRequest, ApprovalHandler, AutoApprovalHandler};
//!
//! let handler = AutoApprovalHandler::new();
//! let request = ApprovalRequest::new("delete_file")
//!     .description("Delete important_file.txt")
//!     .context("path", "/home/user/important_file.txt");
//!
//! let result = handler.request_approval(request);
//! assert!(result.approved);
//! ```

mod approval;
mod handlers;

pub use approval::{ApprovalRequest, ApprovalResult, ApprovalStatus, RiskLevel};
pub use handlers::{
    ApprovalHandler, AutoApprovalHandler, DenyAllHandler, QueueApprovalHandler, RiskBasedHandler,
    TimeoutApprovalHandler,
};
