//! Error types for LiteForge.

use std::fmt;

/// Result type alias for LiteForge operations.
pub type Result<T> = std::result::Result<T, ForgeError>;

/// Base error type for all LiteForge errors.
#[derive(Debug)]
pub enum ForgeError {
    /// Authentication failed (invalid API key, etc.).
    Authentication {
        message: String,
        response: Option<serde_json::Value>,
    },

    /// Rate limits exceeded.
    RateLimit {
        message: String,
        response: Option<serde_json::Value>,
    },

    /// Invalid request (bad parameters, etc.).
    InvalidRequest {
        message: String,
        response: Option<serde_json::Value>,
    },

    /// Server error (5xx).
    Server {
        status_code: u16,
        message: String,
        response: Option<serde_json::Value>,
    },

    /// Network or connection error.
    Network { message: String },

    /// Request timed out.
    Timeout { message: String },

    /// Error parsing SSE stream.
    Stream { message: String },

    /// Model not found.
    ModelNotFound {
        model: String,
        response: Option<serde_json::Value>,
    },

    /// JSON serialization/deserialization error.
    Json { message: String },

    /// Configuration error (missing API key, etc.).
    Config { message: String },

    /// Internal SDK error.
    Internal { message: String },

    /// Generic error for other cases.
    Other { message: String },
}

impl ForgeError {
    /// Get the HTTP status code if applicable.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Authentication { .. } => Some(401),
            Self::RateLimit { .. } => Some(429),
            Self::InvalidRequest { .. } => Some(400),
            Self::Server { status_code, .. } => Some(*status_code),
            Self::ModelNotFound { .. } => Some(404),
            _ => None,
        }
    }

    /// Create an error from an HTTP status code and response.
    pub fn from_status(
        status: u16,
        message: impl Into<String>,
        response: Option<serde_json::Value>,
    ) -> Self {
        let message = message.into();
        match status {
            401 => Self::Authentication { message, response },
            429 => Self::RateLimit { message, response },
            400 => Self::InvalidRequest { message, response },
            404 => Self::ModelNotFound {
                model: message,
                response,
            },
            500..=599 => Self::Server {
                status_code: status,
                message,
                response,
            },
            _ => Self::Server {
                status_code: status,
                message,
                response,
            },
        }
    }

    /// Create an authentication error.
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
            response: None,
        }
    }

    /// Create a rate limit error.
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit {
            message: message.into(),
            response: None,
        }
    }

    /// Create a network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    /// Create a stream error.
    pub fn stream(message: impl Into<String>) -> Self {
        Self::Stream {
            message: message.into(),
        }
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

impl fmt::Display for ForgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication { message, .. } => write!(f, "[401] {}", message),
            Self::RateLimit { message, .. } => write!(f, "[429] {}", message),
            Self::InvalidRequest { message, .. } => write!(f, "[400] {}", message),
            Self::Server {
                status_code,
                message,
                ..
            } => write!(f, "[{}] {}", status_code, message),
            Self::Network { message } => write!(f, "Network error: {}", message),
            Self::Timeout { message } => write!(f, "Timeout: {}", message),
            Self::Stream { message } => write!(f, "Stream error: {}", message),
            Self::ModelNotFound { model, .. } => write!(f, "[404] Model '{}' not found", model),
            Self::Json { message } => write!(f, "JSON error: {}", message),
            Self::Config { message } => write!(f, "Config error: {}", message),
            Self::Internal { message } => write!(f, "Internal error: {}", message),
            Self::Other { message } => write!(f, "Error: {}", message),
        }
    }
}

impl std::error::Error for ForgeError {}

impl From<reqwest::Error> for ForgeError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout {
                message: err.to_string(),
            }
        } else if err.is_connect() {
            Self::Network {
                message: format!("Failed to connect: {}", err),
            }
        } else {
            Self::Network {
                message: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for ForgeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            message: err.to_string(),
        }
    }
}
