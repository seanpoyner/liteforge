//! Centralized path utilities for Forge CLI.
//!
//! Provides cross-platform paths for configuration, agents, skills, MCP servers, and tools.
//! Uses the `dirs` crate for platform-appropriate directories.

use std::path::PathBuf;

/// Get the LiteForge config directory.
///
/// Platform paths:
/// - Linux: `~/.config/forge/`
/// - macOS: `~/Library/Application Support/forge/`
/// - Windows: `%APPDATA%\forge\`
pub fn forge_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("forge")
}

/// Get the LiteForge data directory (for usage.db, etc).
///
/// Platform paths:
/// - Linux: `~/.local/share/forge/`
/// - macOS: `~/Library/Application Support/forge/`
/// - Windows: `%LOCALAPPDATA%\forge\`
pub fn forge_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("forge")
}

/// Get the LiteForge config file path.
pub fn forge_config_path() -> PathBuf {
    forge_config_dir().join("config.toml")
}

/// Get the default agents directory.
pub fn default_agents_dir() -> PathBuf {
    forge_config_dir().join("agents")
}

/// Get the default skills directory.
pub fn default_skills_dir() -> PathBuf {
    forge_config_dir().join("skills")
}

/// Get the default MCP config directory.
pub fn default_mcp_dir() -> PathBuf {
    forge_config_dir().join("mcp")
}

/// Get the default tools directory (for future standalone tool definitions).
pub fn default_tools_dir() -> PathBuf {
    forge_config_dir().join("tools")
}

/// Get the usage database path.
pub fn usage_db_path() -> PathBuf {
    forge_data_dir().join("usage.db")
}

/// Ensure the LiteForge config directory and all subdirectories exist.
pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
    let dir = forge_config_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("agents"))?;
    std::fs::create_dir_all(dir.join("skills"))?;
    std::fs::create_dir_all(dir.join("mcp"))?;
    std::fs::create_dir_all(dir.join("tools"))?;
    Ok(dir)
}

/// Ensure the LiteForge data directory exists.
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = forge_data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_not_empty() {
        let dir = forge_config_dir();
        assert!(dir.ends_with("forge"));
    }

    #[test]
    fn test_data_dir_not_empty() {
        let dir = forge_data_dir();
        assert!(dir.ends_with("forge"));
    }

    #[test]
    fn test_config_path() {
        let path = forge_config_path();
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_default_dirs() {
        assert!(default_agents_dir().ends_with("agents"));
        assert!(default_skills_dir().ends_with("skills"));
        assert!(default_mcp_dir().ends_with("mcp"));
        assert!(default_tools_dir().ends_with("tools"));
    }

    #[test]
    fn test_usage_db_path() {
        let path = usage_db_path();
        assert!(path.ends_with("usage.db"));
    }
}
