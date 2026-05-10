use std::fmt;
use liteforge::ForgeError;

pub enum CliError {
    Sdk(ForgeError),
    Io(std::io::Error),
    NoInput,
    Input(String),
    Config(String),
}

impl From<ForgeError> for CliError {
    fn from(err: ForgeError) -> Self {
        Self::Sdk(err)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sdk(err) => match err {
                ForgeError::Authentication { .. } => {
                    write!(f, "{}\n\nHint: Check your API key with `forge config show` or set it with `forge config set api-key <key>`", err)
                }
                ForgeError::Network { .. } => {
                    write!(f, "{}\n\nHint: Check your network connection and base URL with `forge config show`", err)
                }
                ForgeError::ModelNotFound { .. } => {
                    write!(
                        f,
                        "{}\n\nHint: Run `forge models list` to see available models",
                        err
                    )
                }
                ForgeError::Config { .. } => {
                    write!(
                        f,
                        "{}\n\nHint: Run `forge config show` to check your configuration",
                        err
                    )
                }
                _ => write!(f, "{}", err),
            },
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::NoInput => write!(
                f,
                "No input provided. Pass a prompt as an argument or pipe input via stdin."
            ),
            Self::Input(msg) => write!(f, "Input error: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Sdk(err) => match err {
                ForgeError::Authentication { .. } => 2,
                ForgeError::RateLimit { .. } => 3,
                ForgeError::Config { .. } => 4,
                _ => 1,
            },
            Self::Io(_) => 5,
            Self::NoInput => 6,
            Self::Input(_) => 7,
            Self::Config(_) => 8,
        }
    }
}
