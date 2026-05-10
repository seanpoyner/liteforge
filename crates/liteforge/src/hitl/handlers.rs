//! Approval handlers for processing approval requests.

use super::approval::{ApprovalRequest, ApprovalResult, ApprovalStatus, RiskLevel};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Trait for handling approval requests.
pub trait ApprovalHandler: Send + Sync {
    /// Request approval for an operation.
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult;

    /// Check if this handler supports async approval.
    fn supports_async(&self) -> bool {
        false
    }
}

/// Handler that automatically approves all requests.
/// Useful for testing or low-risk environments.
#[derive(Debug, Clone, Default)]
pub struct AutoApprovalHandler {
    approver_name: Option<String>,
}

impl AutoApprovalHandler {
    /// Create a new auto-approval handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the approver name.
    pub fn with_approver(mut self, name: impl Into<String>) -> Self {
        self.approver_name = Some(name.into());
        self
    }
}

impl ApprovalHandler for AutoApprovalHandler {
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult {
        let mut result = ApprovalResult::approved(&request.id);
        if let Some(name) = &self.approver_name {
            result = result.approver(name);
        } else {
            result = result.approver("auto");
        }
        result.metadata("auto_approved", "true")
    }
}

/// Handler that denies all requests.
/// Useful for strict environments or testing denial flows.
#[derive(Debug, Clone, Default)]
pub struct DenyAllHandler {
    reason: String,
}

impl DenyAllHandler {
    /// Create a new deny-all handler.
    pub fn new() -> Self {
        Self {
            reason: "Denied by policy".to_string(),
        }
    }

    /// Set the denial reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

impl ApprovalHandler for DenyAllHandler {
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult {
        ApprovalResult::denied(&request.id, &self.reason)
    }
}

/// Handler that times out after a specified duration.
/// Useful for testing timeout behavior.
#[derive(Debug, Clone)]
pub struct TimeoutApprovalHandler {
    timeout: Duration,
}

impl TimeoutApprovalHandler {
    /// Create a handler with a specific timeout.
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Create a handler with timeout in seconds.
    pub fn with_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

impl ApprovalHandler for TimeoutApprovalHandler {
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult {
        std::thread::sleep(self.timeout);
        ApprovalResult::timed_out(&request.id)
    }
}

/// A queued approval request.
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    /// The request.
    pub request: ApprovalRequest,
    /// The result (once decided).
    #[allow(dead_code)]
    pub result: Option<ApprovalResult>,
}

/// Handler that queues requests for later processing.
/// Useful for async approval workflows.
#[derive(Debug, Clone)]
pub struct QueueApprovalHandler {
    queue: Arc<Mutex<VecDeque<QueuedRequest>>>,
    results: Arc<Mutex<std::collections::HashMap<String, ApprovalResult>>>,
    default_on_timeout: bool,
}

impl Default for QueueApprovalHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueApprovalHandler {
    /// Create a new queue handler.
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_on_timeout: false,
        }
    }

    /// Set default action on timeout (true = approve, false = deny).
    pub fn default_on_timeout(mut self, approve: bool) -> Self {
        self.default_on_timeout = approve;
        self
    }

    /// Get pending requests count.
    pub fn pending_count(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Get all pending requests.
    pub fn pending_requests(&self) -> Vec<ApprovalRequest> {
        self.queue
            .lock()
            .unwrap()
            .iter()
            .map(|q| q.request.clone())
            .collect()
    }

    /// Get the next pending request.
    pub fn next_request(&self) -> Option<ApprovalRequest> {
        self.queue
            .lock()
            .unwrap()
            .front()
            .map(|q| q.request.clone())
    }

    /// Approve a request by ID.
    pub fn approve(&self, request_id: &str, approver: Option<&str>) -> bool {
        self.decide(request_id, true, None, approver)
    }

    /// Deny a request by ID.
    pub fn deny(&self, request_id: &str, reason: &str, approver: Option<&str>) -> bool {
        self.decide(request_id, false, Some(reason), approver)
    }

    /// Make a decision on a request.
    fn decide(
        &self,
        request_id: &str,
        approved: bool,
        reason: Option<&str>,
        approver: Option<&str>,
    ) -> bool {
        let mut queue = self.queue.lock().unwrap();

        // Find and remove from queue
        let pos = queue.iter().position(|q| q.request.id == request_id);
        if let Some(idx) = pos {
            queue.remove(idx);

            // Store result
            let mut result = if approved {
                ApprovalResult::approved(request_id)
            } else {
                ApprovalResult::denied(request_id, reason.unwrap_or("Denied"))
            };

            if let Some(name) = approver {
                result = result.approver(name);
            }

            self.results
                .lock()
                .unwrap()
                .insert(request_id.to_string(), result);
            true
        } else {
            false
        }
    }

    /// Get a result for a request ID.
    pub fn get_result(&self, request_id: &str) -> Option<ApprovalResult> {
        self.results.lock().unwrap().get(request_id).cloned()
    }

    /// Clear all pending requests and results.
    pub fn clear(&self) {
        self.queue.lock().unwrap().clear();
        self.results.lock().unwrap().clear();
    }
}

impl ApprovalHandler for QueueApprovalHandler {
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult {
        let request_id = request.id.clone();

        // Add to queue
        self.queue.lock().unwrap().push_back(QueuedRequest {
            request: request.clone(),
            result: None,
        });

        // If there's a timeout, check it
        if let Some(timeout_secs) = request.timeout_secs {
            let start = std::time::Instant::now();
            let timeout = Duration::from_secs(timeout_secs);

            // Poll for result
            loop {
                if let Some(result) = self.get_result(&request_id) {
                    return result;
                }

                if start.elapsed() > timeout {
                    // Remove from queue if still there
                    self.queue
                        .lock()
                        .unwrap()
                        .retain(|q| q.request.id != request_id);

                    return if self.default_on_timeout {
                        ApprovalResult::approved(&request_id).metadata("timeout_default", "true")
                    } else {
                        ApprovalResult::timed_out(&request_id)
                    };
                }

                std::thread::sleep(Duration::from_millis(100));
            }
        } else {
            // No timeout - return immediately with pending status
            // In real usage, caller would need to poll for result
            ApprovalResult {
                request_id,
                approved: false,
                status: ApprovalStatus::Pending,
                reason: None,
                approver: None,
                decided_at: 0,
                metadata: std::collections::HashMap::new(),
            }
        }
    }

    fn supports_async(&self) -> bool {
        true
    }
}

/// Handler that approves based on risk level.
#[derive(Clone)]
pub struct RiskBasedHandler {
    /// Maximum risk level to auto-approve.
    max_auto_approve: RiskLevel,
    /// Fallback handler for higher risk.
    fallback: Arc<dyn ApprovalHandler>,
}

impl std::fmt::Debug for RiskBasedHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RiskBasedHandler")
            .field("max_auto_approve", &self.max_auto_approve)
            .field("fallback", &"<dyn ApprovalHandler>")
            .finish()
    }
}

impl RiskBasedHandler {
    /// Create a new risk-based handler.
    pub fn new(max_auto_approve: RiskLevel, fallback: Arc<dyn ApprovalHandler>) -> Self {
        Self {
            max_auto_approve,
            fallback,
        }
    }

    /// Check if risk level should be auto-approved.
    fn should_auto_approve(&self, level: RiskLevel) -> bool {
        matches!(
            (level, self.max_auto_approve),
            (RiskLevel::Low, _)
                | (
                    RiskLevel::Medium,
                    RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical
                )
                | (RiskLevel::High, RiskLevel::High | RiskLevel::Critical)
                | (RiskLevel::Critical, RiskLevel::Critical)
        )
    }
}

impl ApprovalHandler for RiskBasedHandler {
    fn request_approval(&self, request: ApprovalRequest) -> ApprovalResult {
        if self.should_auto_approve(request.risk_level) {
            ApprovalResult::approved(&request.id)
                .approver("risk_policy")
                .metadata("auto_approved_risk", format!("{:?}", request.risk_level))
        } else {
            self.fallback.request_approval(request)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_approval_handler() {
        let handler = AutoApprovalHandler::new();
        let request = ApprovalRequest::new("test_op");

        let result = handler.request_approval(request);
        assert!(result.approved);
        assert_eq!(result.status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_auto_approval_with_name() {
        let handler = AutoApprovalHandler::new().with_approver("test_system");
        let request = ApprovalRequest::new("test_op");

        let result = handler.request_approval(request);
        assert_eq!(result.approver, Some("test_system".to_string()));
    }

    #[test]
    fn test_deny_all_handler() {
        let handler = DenyAllHandler::new().with_reason("Testing");
        let request = ApprovalRequest::new("test_op");

        let result = handler.request_approval(request);
        assert!(!result.approved);
        assert_eq!(result.status, ApprovalStatus::Denied);
        assert_eq!(result.reason, Some("Testing".to_string()));
    }

    #[test]
    fn test_queue_handler_pending() {
        let handler = QueueApprovalHandler::new();
        let request = ApprovalRequest::new("test_op");

        // Without timeout, returns pending immediately
        let result = handler.request_approval(request);
        assert!(!result.approved);
        assert_eq!(result.status, ApprovalStatus::Pending);
        assert_eq!(handler.pending_count(), 1);
    }

    #[test]
    fn test_queue_handler_approve() {
        let handler = QueueApprovalHandler::new();
        let request = ApprovalRequest::new("test_op");
        let request_id = request.id.clone();

        handler.request_approval(request);
        assert!(handler.approve(&request_id, Some("admin")));

        let result = handler.get_result(&request_id).unwrap();
        assert!(result.approved);
        assert_eq!(handler.pending_count(), 0);
    }

    #[test]
    fn test_queue_handler_deny() {
        let handler = QueueApprovalHandler::new();
        let request = ApprovalRequest::new("test_op");
        let request_id = request.id.clone();

        handler.request_approval(request);
        assert!(handler.deny(&request_id, "Not allowed", None));

        let result = handler.get_result(&request_id).unwrap();
        assert!(!result.approved);
        assert_eq!(result.reason, Some("Not allowed".to_string()));
    }

    #[test]
    fn test_queue_handler_with_timeout() {
        let handler = QueueApprovalHandler::new();
        let request = ApprovalRequest::new("test_op").timeout(1); // 1 second timeout

        // This will wait for timeout
        let result = handler.request_approval(request);
        assert!(!result.approved);
        assert_eq!(result.status, ApprovalStatus::TimedOut);
    }

    #[test]
    fn test_risk_based_handler_auto_approve_low() {
        let fallback = Arc::new(DenyAllHandler::new());
        let handler = RiskBasedHandler::new(RiskLevel::Low, fallback);

        let request = ApprovalRequest::new("test").risk_level(RiskLevel::Low);
        let result = handler.request_approval(request);
        assert!(result.approved);
    }

    #[test]
    fn test_risk_based_handler_fallback() {
        let fallback = Arc::new(DenyAllHandler::new());
        let handler = RiskBasedHandler::new(RiskLevel::Low, fallback);

        let request = ApprovalRequest::new("test").risk_level(RiskLevel::High);
        let result = handler.request_approval(request);
        assert!(!result.approved); // Falls back to DenyAll
    }

    #[test]
    fn test_queue_pending_requests() {
        let handler = QueueApprovalHandler::new();

        handler.request_approval(ApprovalRequest::new("op1"));
        handler.request_approval(ApprovalRequest::new("op2"));
        handler.request_approval(ApprovalRequest::new("op3"));

        let pending = handler.pending_requests();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].operation, "op1");
        assert_eq!(pending[2].operation, "op3");
    }

    #[test]
    fn test_queue_clear() {
        let handler = QueueApprovalHandler::new();

        handler.request_approval(ApprovalRequest::new("op1"));
        handler.request_approval(ApprovalRequest::new("op2"));

        handler.clear();
        assert_eq!(handler.pending_count(), 0);
    }

    #[test]
    fn test_supports_async() {
        let auto = AutoApprovalHandler::new();
        assert!(!auto.supports_async());

        let queue = QueueApprovalHandler::new();
        assert!(queue.supports_async());
    }
}
