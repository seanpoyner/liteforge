//! Persistent configuration for Forge CLI.
//!
//! Manages the config.toml file in the LiteForge config directory.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Forge CLI configuration stored in config.toml.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ForgeCliConfig {
    /// Path configuration overrides.
    #[serde(default)]
    pub paths: PathsConfig,
}

/// Path configuration for customizing resource directories.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Custom agents directory (overrides default ~/.forge/agents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_dir: Option<PathBuf>,

    /// Custom skills directory (overrides default ~/.forge/skills).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills_dir: Option<PathBuf>,

    /// Custom MCP config directory (overrides default ~/.forge/mcp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_dir: Option<PathBuf>,

    /// Custom tools directory (overrides default ~/.forge/tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_dir: Option<PathBuf>,
}

impl ForgeCliConfig {
    /// Load configuration from the default config file.
    ///
    /// Returns default config if file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        Self::load_from(&paths::forge_config_path())
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save configuration to the default config file.
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&paths::forge_config_path())
    }

    /// Save configuration to a specific path.
    pub fn save_to(&self, path: &PathBuf) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;

        std::fs::write(path, content)
    }

    /// Get the effective agents directory.
    ///
    /// Returns the configured override, or the default ~/.forge/agents.
    pub fn agents_dir(&self) -> PathBuf {
        self.paths
            .agents_dir
            .clone()
            .unwrap_or_else(paths::default_agents_dir)
    }

    /// Get the effective skills directory.
    ///
    /// Returns the configured override, or the default ~/.forge/skills.
    pub fn skills_dir(&self) -> PathBuf {
        self.paths
            .skills_dir
            .clone()
            .unwrap_or_else(paths::default_skills_dir)
    }

    /// Get the effective MCP config directory.
    ///
    /// Returns the configured override, or the default ~/.forge/mcp.
    pub fn mcp_dir(&self) -> PathBuf {
        self.paths
            .mcp_dir
            .clone()
            .unwrap_or_else(paths::default_mcp_dir)
    }

    /// Get the effective tools directory.
    ///
    /// Returns the configured override, or the default ~/.forge/tools.
    pub fn tools_dir(&self) -> PathBuf {
        self.paths
            .tools_dir
            .clone()
            .unwrap_or_else(paths::default_tools_dir)
    }

    /// Set a path by key name.
    ///
    /// Valid keys: `agents-dir`, `skills-dir`, `mcp-dir`, `tools-dir`.
    pub fn set_path(&mut self, key: &str, value: PathBuf) -> Result<(), String> {
        match key {
            "agents-dir" => self.paths.agents_dir = Some(value),
            "skills-dir" => self.paths.skills_dir = Some(value),
            "mcp-dir" => self.paths.mcp_dir = Some(value),
            "tools-dir" => self.paths.tools_dir = Some(value),
            _ => return Err(format!("Unknown path key: {}", key)),
        }
        Ok(())
    }

    /// Get a path by key name.
    ///
    /// Valid keys: `agents-dir`, `skills-dir`, `mcp-dir`, `tools-dir`.
    pub fn get_path(&self, key: &str) -> Result<PathBuf, String> {
        match key {
            "agents-dir" => Ok(self.agents_dir()),
            "skills-dir" => Ok(self.skills_dir()),
            "mcp-dir" => Ok(self.mcp_dir()),
            "tools-dir" => Ok(self.tools_dir()),
            _ => Err(format!("Unknown path key: {}", key)),
        }
    }

    /// Check if a key is a path key (vs an env key).
    pub fn is_path_key(key: &str) -> bool {
        matches!(key, "agents-dir" | "skills-dir" | "mcp-dir" | "tools-dir")
    }
}

/// Generate a default config.toml content with documentation comments.
pub fn default_config_content() -> String {
    r#"# Forge CLI Configuration
# This file stores persistent CLI settings.
# Environment variables (api-key, base-url, etc.) are stored in .env files.

[paths]
# Uncomment and modify to override default paths:
# agents_dir = "/path/to/custom/agents"
# skills_dir = "/path/to/custom/skills"
# mcp_dir = "/path/to/custom/mcp"
# tools_dir = "/path/to/custom/tools"
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ForgeCliConfig::default();
        assert!(config.paths.agents_dir.is_none());
        assert!(config.paths.skills_dir.is_none());
    }

    #[test]
    fn test_effective_dirs_use_defaults() {
        let config = ForgeCliConfig::default();
        assert!(config.agents_dir().ends_with("agents"));
        assert!(config.skills_dir().ends_with("skills"));
        assert!(config.mcp_dir().ends_with("mcp"));
        assert!(config.tools_dir().ends_with("tools"));
    }

    #[test]
    fn test_effective_dirs_use_overrides() {
        let mut config = ForgeCliConfig::default();
        config.paths.agents_dir = Some(PathBuf::from("/custom/agents"));

        assert_eq!(config.agents_dir(), PathBuf::from("/custom/agents"));
        // Others still use defaults
        assert!(config.skills_dir().ends_with("skills"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        let mut config = ForgeCliConfig::default();
        config.paths.agents_dir = Some(PathBuf::from("/my/agents"));

        config.save_to(&config_path).unwrap();
        let loaded = ForgeCliConfig::load_from(&config_path);

        assert_eq!(loaded.paths.agents_dir, Some(PathBuf::from("/my/agents")));
    }

    #[test]
    fn test_set_and_get_path() {
        let mut config = ForgeCliConfig::default();

        config
            .set_path("agents-dir", PathBuf::from("/test"))
            .unwrap();
        assert_eq!(
            config.get_path("agents-dir").unwrap(),
            PathBuf::from("/test")
        );

        assert!(config.set_path("unknown", PathBuf::from("/x")).is_err());
        assert!(config.get_path("unknown").is_err());
    }

    #[test]
    fn test_is_path_key() {
        assert!(ForgeCliConfig::is_path_key("agents-dir"));
        assert!(ForgeCliConfig::is_path_key("skills-dir"));
        assert!(!ForgeCliConfig::is_path_key("api-key"));
        assert!(!ForgeCliConfig::is_path_key("unknown"));
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/config.toml");
        let config = ForgeCliConfig::load_from(&path);
        assert!(config.paths.agents_dir.is_none());
    }
}
