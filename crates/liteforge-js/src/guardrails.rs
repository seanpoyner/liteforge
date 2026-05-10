use liteforge::guardrails::{
    check_all as rust_check_all, detect_injection as rust_detect_injection,
    detect_pii as rust_detect_pii, find_pii as rust_find_pii, redact_pii as rust_redact_pii,
    GuardrailResult as RustGuardrailResult,
};

#[napi(object)]
pub struct GuardrailResult {
    pub passed: bool,
    pub value: String,
    pub message: String,
    pub guardrail_name: String,
}

fn rust_guardrail_to_js(r: &RustGuardrailResult) -> GuardrailResult {
    GuardrailResult {
        passed: r.passed,
        value: r.value.clone(),
        message: r.message.clone(),
        guardrail_name: r.guardrail_name.clone(),
    }
}

#[napi(object)]
pub struct PiiMatch {
    pub pii_type: String,
    pub value: String,
}

#[napi]
pub fn detect_pii(text: String) -> GuardrailResult {
    rust_guardrail_to_js(&rust_detect_pii(&text))
}

#[napi]
pub fn redact_pii(text: String) -> String {
    rust_redact_pii(&text)
}

#[napi]
pub fn find_pii(text: String) -> Vec<PiiMatch> {
    rust_find_pii(&text)
        .into_iter()
        .map(|(pii_type, value)| PiiMatch {
            pii_type: pii_type.name().to_string(),
            value,
        })
        .collect()
}

#[napi]
pub fn detect_injection(text: String) -> GuardrailResult {
    rust_guardrail_to_js(&rust_detect_injection(&text))
}

#[napi]
pub fn check_all(text: String) -> GuardrailResult {
    rust_guardrail_to_js(&rust_check_all(&text))
}
