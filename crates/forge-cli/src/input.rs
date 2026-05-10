use crate::error::CliError;
use std::io::Read;

/// Resolve the prompt from various sources.
///
/// Priority: file > explicit prompt > stdin (if forced or piped)
pub fn resolve_prompt(
    prompt_arg: Option<String>,
    file_path: Option<String>,
    force_stdin: bool,
) -> Result<String, CliError> {
    // File takes priority
    if let Some(path) = file_path {
        return std::fs::read_to_string(&path).map_err(CliError::Io);
    }

    // Explicit prompt argument
    if let Some(prompt) = prompt_arg {
        return Ok(prompt);
    }

    // Read from stdin if forced or if stdin is piped
    if force_stdin || !atty::is(atty::Stream::Stdin) {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            return Err(CliError::NoInput);
        }
        return Ok(trimmed);
    }

    Err(CliError::NoInput)
}
