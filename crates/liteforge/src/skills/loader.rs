//! Skill loading from files and registries.

use super::{PromptSkill, SkillConfig, SkillError, SkillRegistry, SkillResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Source of a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    /// Skill loaded from a file.
    File(PathBuf),
    /// Skill loaded from a directory.
    Directory(PathBuf),
    /// Skill from inline configuration.
    Inline(Box<SkillConfig>),
    /// Built-in skill.
    Builtin(String),
}

/// Loader for skills from various sources.
pub struct SkillLoader {
    search_paths: Vec<PathBuf>,
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillLoader {
    /// Create a new skill loader.
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    /// Add a search path for skill files.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Add multiple search paths.
    pub fn with_paths(mut self, paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.search_paths
            .extend(paths.into_iter().map(|p| p.into()));
        self
    }

    /// Load a skill from a file.
    pub fn load_file(&self, path: impl AsRef<Path>) -> SkillResult<PromptSkill> {
        let path = path.as_ref();

        if !path.exists() {
            // Try search paths
            for search_path in &self.search_paths {
                let full_path = search_path.join(path);
                if full_path.exists() {
                    return self.load_file_internal(&full_path);
                }
            }
            return Err(SkillError::NotFound(path.display().to_string()));
        }

        self.load_file_internal(path)
    }

    fn load_file_internal(&self, path: &Path) -> SkillResult<PromptSkill> {
        let content = std::fs::read_to_string(path)?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let config: SkillConfig = match extension {
            "json" => serde_json::from_str(&content)
                .map_err(|e| SkillError::InvalidConfig(format!("JSON parse error: {}", e)))?,
            "yaml" | "yml" => {
                // YAML support requires serde_yaml crate
                return Err(SkillError::InvalidConfig(
                    "YAML format not supported. Use JSON instead.".to_string(),
                ));
            }
            "toml" => {
                // TOML support requires toml crate
                return Err(SkillError::InvalidConfig(
                    "TOML format not supported. Use JSON instead.".to_string(),
                ));
            }
            _ => {
                return Err(SkillError::InvalidConfig(format!(
                    "Unsupported file format: {}",
                    extension
                )));
            }
        };

        Ok(PromptSkill::new(config))
    }

    /// Load all skills from a directory.
    pub fn load_directory(&self, path: impl AsRef<Path>) -> SkillResult<Vec<PromptSkill>> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(SkillError::InvalidConfig(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let mut skills = Vec::new();

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                if matches!(extension, "json" | "yaml" | "yml" | "toml") {
                    match self.load_file_internal(&path) {
                        Ok(skill) => skills.push(skill),
                        Err(e) => {
                            // Log warning but continue loading other skills
                            eprintln!(
                                "Warning: Failed to load skill from {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Load a skill from inline configuration.
    pub fn load_config(&self, config: SkillConfig) -> PromptSkill {
        PromptSkill::new(config)
    }

    /// Load a built-in skill by name.
    pub fn load_builtin(&self, name: &str) -> SkillResult<PromptSkill> {
        match name {
            "summarize" => Ok(super::summarize_skill()),
            "translate" => Ok(super::translate_skill()),
            "extract" => Ok(super::extract_skill()),
            "rewrite" => Ok(super::rewrite_skill()),
            "qa" => Ok(super::qa_skill()),
            _ => Err(SkillError::NotFound(format!("Built-in skill: {}", name))),
        }
    }

    /// Load a skill from a source.
    pub fn load(&self, source: &SkillSource) -> SkillResult<PromptSkill> {
        match source {
            SkillSource::File(path) => self.load_file(path),
            SkillSource::Directory(_) => Err(SkillError::InvalidConfig(
                "Use load_directory for directory sources".to_string(),
            )),
            SkillSource::Inline(config) => Ok(self.load_config(*config.clone())),
            SkillSource::Builtin(name) => self.load_builtin(name),
        }
    }

    /// Load multiple skills and register them.
    pub fn load_into_registry(
        &self,
        sources: impl IntoIterator<Item = SkillSource>,
        registry: &mut SkillRegistry,
    ) -> SkillResult<usize> {
        let mut count = 0;

        for source in sources {
            match source {
                SkillSource::Directory(path) => {
                    let skills = self.load_directory(&path)?;
                    for skill in skills {
                        registry.register(skill);
                        count += 1;
                    }
                }
                _ => {
                    let skill = self.load(&source)?;
                    registry.register(skill);
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

/// Skill definition format for files.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Skill configuration.
    #[serde(flatten)]
    pub config: SkillConfig,
    /// Examples of input/output.
    #[serde(default)]
    pub examples: Vec<SkillExample>,
}

/// Example usage of a skill.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    /// Example input.
    pub input: String,
    /// Expected output.
    pub output: String,
    /// Description of the example.
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Skill;
    use tempfile::TempDir;

    #[test]
    fn test_skill_loader_new() {
        let loader = SkillLoader::new();
        assert!(loader.search_paths.is_empty());
    }

    #[test]
    fn test_skill_loader_with_paths() {
        let loader = SkillLoader::new()
            .with_path("/path/one")
            .with_path("/path/two");

        assert_eq!(loader.search_paths.len(), 2);
    }

    #[test]
    fn test_load_builtin_skills() {
        let loader = SkillLoader::new();

        let skill = loader.load_builtin("summarize").unwrap();
        assert_eq!(skill.name(), "summarize");

        let skill = loader.load_builtin("translate").unwrap();
        assert_eq!(skill.name(), "translate");

        assert!(loader.load_builtin("nonexistent").is_err());
    }

    #[test]
    fn test_load_config() {
        let loader = SkillLoader::new();
        let config = SkillConfig::new("custom", "Custom skill").with_system_prompt("Be helpful");

        let skill = loader.load_config(config);
        assert_eq!(skill.name(), "custom");
    }

    #[test]
    fn test_load_from_json_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("skill.json");

        let config = r#"{
            "name": "json-skill",
            "description": "A skill from JSON",
            "system_prompt": "You are helpful"
        }"#;

        std::fs::write(&file_path, config).unwrap();

        let loader = SkillLoader::new();
        let skill = loader.load_file(&file_path).unwrap();

        assert_eq!(skill.name(), "json-skill");
    }

    #[test]
    fn test_load_from_yaml_file_unsupported() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("skill.yaml");

        let config = r#"
name: yaml-skill
description: A skill from YAML
system_prompt: You are helpful
"#;

        std::fs::write(&file_path, config).unwrap();

        let loader = SkillLoader::new();
        let result = loader.load_file(&file_path);

        // YAML is not supported, should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_load_directory() {
        let dir = TempDir::new().unwrap();

        // Create two skill files
        let skill1 = r#"{"name": "skill1", "description": "First"}"#;
        let skill2 = r#"{"name": "skill2", "description": "Second"}"#;

        std::fs::write(dir.path().join("skill1.json"), skill1).unwrap();
        std::fs::write(dir.path().join("skill2.json"), skill2).unwrap();

        let loader = SkillLoader::new();
        let skills = loader.load_directory(dir.path()).unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn test_load_into_registry() {
        let dir = TempDir::new().unwrap();

        let skill = r#"{"name": "file-skill", "description": "From file"}"#;
        std::fs::write(dir.path().join("skill.json"), skill).unwrap();

        let loader = SkillLoader::new();
        let mut registry = SkillRegistry::new();

        let sources = vec![
            SkillSource::Builtin("summarize".to_string()),
            SkillSource::File(dir.path().join("skill.json")),
        ];

        let count = loader.load_into_registry(sources, &mut registry).unwrap();

        assert_eq!(count, 2);
        assert!(registry.has("summarize"));
        assert!(registry.has("file-skill"));
    }

    #[test]
    fn test_skill_source() {
        let loader = SkillLoader::new();

        // Test builtin source
        let source = SkillSource::Builtin("summarize".to_string());
        let skill = loader.load(&source).unwrap();
        assert_eq!(skill.name(), "summarize");

        // Test inline source
        let config = SkillConfig::new("inline", "Inline skill");
        let source = SkillSource::Inline(Box::new(config));
        let skill = loader.load(&source).unwrap();
        assert_eq!(skill.name(), "inline");
    }
}
