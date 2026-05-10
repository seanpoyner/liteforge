//! Approval request and result types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Risk level for an operation requiring approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk, minimal impact if wrong.
    Low,
    /// Medium risk, some impact.
    #[default]
    Medium,
    /// High risk, significant impact.
    High,
    /// Critical risk, severe impact.
    Critical,
}

/// Status of an approval request.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Waiting for approval.
    #[default]
    Pending,
    /// Approved by a human.
    Approved,
    /// Denied by a human.
    Denied,
    /// Request timed out.
    TimedOut,
    /// Request was cancelled.
    Cancelled,
}

/// A request for human approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique request ID.
    pub id: String,
    /// Operation name/type.
    pub operation: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Risk level.
    pub risk_level: RiskLevel,
    /// Additional context.
    pub context: HashMap<String, String>,
    /// Timestamp when created.
    pub created_at: u64,
    /// Timeout in seconds (None = no timeout).
    pub timeout_secs: Option<u64>,
    /// Agent/component making the request.
    pub requester: Option<String>,
}

impl ApprovalRequest {
    /// Create a new approval request.
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            id: generate_request_id(),
            operation: operation.into(),
            description: None,
            risk_level: RiskLevel::Medium,
            context: HashMap::new(),
            created_at: now_millis(),
            timeout_secs: None,
            requester: None,
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the risk level.
    pub fn risk_level(mut self, level: RiskLevel) -> Self {
        self.risk_level = level;
        self
    }

    /// Add context.
    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Set timeout.
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set requester.
    pub fn requester(mut self, req: impl Into<String>) -> Self {
        self.requester = Some(req.into());
        self
    }

    /// Check if the request has timed out.
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout_secs {
            let elapsed = now_millis().saturating_sub(self.created_at);
            elapsed > timeout * 1000
        } else {
            false
        }
    }

    /// Get a summary for display.
    pub fn summary(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("No description");
        format!(
            "[{}] {} - {} (Risk: {:?})",
            self.id, self.operation, desc, self.risk_level
        )
    }
}

/// Result of an approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    /// Request ID.
    pub request_id: String,
    /// Whether approved.
    pub approved: bool,
    /// Status.
    pub status: ApprovalStatus,
    /// Reason (if denied or cancelled).
    pub reason: Option<String>,
    /// Who approved/denied (if applicable).
    pub approver: Option<String>,
    /// Timestamp when decided.
    pub decided_at: u64,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl ApprovalResult {
    /// Create an approved result.
    pub fn approved(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            approved: true,
            status: ApprovalStatus::Approved,
            reason: None,
            approver: None,
            decided_at: now_millis(),
            metadata: HashMap::new(),
        }
    }

    /// Create a denied result.
    pub fn denied(request_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            approved: false,
            status: ApprovalStatus::Denied,
            reason: Some(reason.into()),
            approver: None,
            decided_at: now_millis(),
            metadata: HashMap::new(),
        }
    }

    /// Create a timed out result.
    pub fn timed_out(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            approved: false,
            status: ApprovalStatus::TimedOut,
            reason: Some("Request timed out".to_string()),
            approver: None,
            decided_at: now_millis(),
            metadata: HashMap::new(),
        }
    }

    /// Create a cancelled result.
    pub fn cancelled(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            approved: false,
            status: ApprovalStatus::Cancelled,
            reason: Some("Request cancelled".to_string()),
            approver: None,
            decided_at: now_millis(),
            metadata: HashMap::new(),
        }
    }

    /// Set approver.
    pub fn approver(mut self, approver: impl Into<String>) -> Self {
        self.approver = Some(approver.into());
        self
    }

    /// Set reason.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Generate a unique request ID.
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = now_millis();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("apr-{:x}-{:04x}", ts, count as u16)
}

/// Get current time in milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_request_new() {
        let req = ApprovalRequest::new("delete_file");
        assert_eq!(req.operation, "delete_file");
        assert!(!req.id.is_empty());
        assert_eq!(req.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_approval_request_builder() {
        let req = ApprovalRequest::new("send_email")
            .description("Send welcome email")
            .risk_level(RiskLevel::Low)
            .context("recipient", "user@example.com")
            .timeout(60)
            .requester("email_agent");

        assert_eq!(req.operation, "send_email");
        assert_eq!(req.description, Some("Send welcome email".to_string()));
        assert_eq!(req.risk_level, RiskLevel::Low);
        assert_eq!(
            req.context.get("recipient"),
            Some(&"user@example.com".to_string())
        );
        assert_eq!(req.timeout_secs, Some(60));
        assert_eq!(req.requester, Some("email_agent".to_string()));
    }

    #[test]
    fn test_approval_result_approved() {
        let result = ApprovalResult::approved("req-123").approver("admin");

        assert!(result.approved);
        assert_eq!(result.status, ApprovalStatus::Approved);
        assert_eq!(result.approver, Some("admin".to_string()));
    }

    #[test]
    fn test_approval_result_denied() {
        let result = ApprovalResult::denied("req-123", "Too risky");

        assert!(!result.approved);
        assert_eq!(result.status, ApprovalStatus::Denied);
        assert_eq!(result.reason, Some("Too risky".to_string()));
    }

    #[test]
    fn test_approval_result_timed_out() {
        let result = ApprovalResult::timed_out("req-123");

        assert!(!result.approved);
        assert_eq!(result.status, ApprovalStatus::TimedOut);
    }

    #[test]
    fn test_request_summary() {
        let req = ApprovalRequest::new("delete_file")
            .description("Delete config.json")
            .risk_level(RiskLevel::High);

        let summary = req.summary();
        assert!(summary.contains("delete_file"));
        assert!(summary.contains("Delete config.json"));
        assert!(summary.contains("High"));
    }

    #[test]
    fn test_is_timed_out() {
        let req = ApprovalRequest::new("test").timeout(0); // 0 second timeout

        // Should time out immediately
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(req.is_timed_out());
    }

    #[test]
    fn test_not_timed_out() {
        let req = ApprovalRequest::new("test").timeout(3600); // 1 hour timeout

        assert!(!req.is_timed_out());
    }

    #[test]
    fn test_no_timeout() {
        let req = ApprovalRequest::new("test");
        // No timeout set, should never time out
        assert!(!req.is_timed_out());
    }
}
