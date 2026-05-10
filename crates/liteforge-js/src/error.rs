use napi::bindgen_prelude::*;
use liteforge::ForgeError as RustForgeError;

#[napi(string_enum)]
pub enum TipErrorKind {
    Authentication,
    RateLimit,
    InvalidRequest,
    Server,
    Network,
    Timeout,
    Stream,
    ModelNotFound,
    Json,
    Config,
    Internal,
    Other,
}

pub fn forge_error_to_napi(err: RustForgeError) -> Error {
    let status_code = err.status_code();
    let kind = match &err {
        RustForgeError::Authentication { .. } => "Authentication",
        RustForgeError::RateLimit { .. } => "RateLimit",
        RustForgeError::InvalidRequest { .. } => "InvalidRequest",
        RustForgeError::Server { .. } => "Server",
        RustForgeError::Network { .. } => "Network",
        RustForgeError::Timeout { .. } => "Timeout",
        RustForgeError::Stream { .. } => "Stream",
        RustForgeError::ModelNotFound { .. } => "ModelNotFound",
        RustForgeError::Json { .. } => "Json",
        RustForgeError::Config { .. } => "Config",
        RustForgeError::Internal { .. } => "Internal",
        RustForgeError::Other { .. } => "Other",
    };
    let msg = if let Some(code) = status_code {
        format!("[{}][{}] {}", kind, code, err)
    } else {
        format!("[{}] {}", kind, err)
    };
    Error::from_reason(msg)
}
