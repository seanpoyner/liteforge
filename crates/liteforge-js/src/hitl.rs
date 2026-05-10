use liteforge::hitl::{ApprovalRequest as RustApprovalRequest, RiskLevel as RustRiskLevel};

#[napi(string_enum)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

fn rust_risk_to_js(r: &RustRiskLevel) -> RiskLevel {
    match r {
        RustRiskLevel::Low => RiskLevel::Low,
        RustRiskLevel::Medium => RiskLevel::Medium,
        RustRiskLevel::High => RiskLevel::High,
        RustRiskLevel::Critical => RiskLevel::Critical,
    }
}

fn js_risk_to_rust(r: &RiskLevel) -> RustRiskLevel {
    match r {
        RiskLevel::Low => RustRiskLevel::Low,
        RiskLevel::Medium => RustRiskLevel::Medium,
        RiskLevel::High => RustRiskLevel::High,
        RiskLevel::Critical => RustRiskLevel::Critical,
    }
}

#[napi(string_enum)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Timeout,
}

#[napi(object)]
pub struct JsApprovalRequest {
    pub id: String,
    pub action: String,
    pub description: String,
    pub risk_level: String,
    pub tool_name: Option<String>,
    pub arguments: Option<serde_json::Value>,
}

#[napi(object)]
pub struct JsApprovalResult {
    pub approved: bool,
    pub reason: Option<String>,
    pub modified_args: Option<serde_json::Value>,
}

#[napi]
pub fn create_approval_request(
    action: String,
    description: String,
    risk_level: RiskLevel,
    tool_name: Option<String>,
    arguments: Option<serde_json::Value>,
) -> JsApprovalRequest {
    let req = RustApprovalRequest::new(action.clone());
    JsApprovalRequest {
        id: req.id.clone(),
        action,
        description,
        risk_level: format!("{:?}", js_risk_to_rust(&risk_level)),
        tool_name,
        arguments,
    }
}
