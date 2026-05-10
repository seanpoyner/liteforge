//! Skill composition and chaining.

use super::{Skill, SkillConfig, SkillError, SkillInput, SkillOutput, SkillResult};
use crate::client::AsyncForgeClient;
use async_trait::async_trait;
use std::sync::Arc;

/// Strategy for composing skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionStrategy {
    /// Execute skills sequentially, passing output to input.
    Sequential,
    /// Execute skills in parallel and merge outputs.
    Parallel,
    /// Execute skills conditionally based on input/output.
    Conditional,
}

/// A composed skill that chains multiple skills together.
pub struct ComposedSkill {
    name: String,
    #[allow(dead_code)]
    description: String,
    skills: Vec<Arc<dyn Skill>>,
    strategy: CompositionStrategy,
}

impl ComposedSkill {
    /// Create a new composed skill.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            skills: Vec::new(),
            strategy: CompositionStrategy::Sequential,
        }
    }

    /// Set the composition strategy.
    pub fn with_strategy(mut self, strategy: CompositionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Add a skill to the composition.
    pub fn add_skill<S: Skill + 'static>(mut self, skill: S) -> Self {
        self.skills.push(Arc::new(skill));
        self
    }

    /// Add multiple skills.
    pub fn add_skills<S: Skill + 'static>(mut self, skills: impl IntoIterator<Item = S>) -> Self {
        self.skills
            .extend(skills.into_iter().map(|s| Arc::new(s) as Arc<dyn Skill>));
        self
    }

    /// Get the number of skills in the composition.
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    /// Get the composition strategy.
    pub fn strategy(&self) -> CompositionStrategy {
        self.strategy
    }
}

#[async_trait]
impl Skill for ComposedSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &SkillConfig {
        // Return a minimal config
        static DEFAULT_CONFIG: std::sync::OnceLock<SkillConfig> = std::sync::OnceLock::new();
        DEFAULT_CONFIG.get_or_init(SkillConfig::default)
    }

    async fn execute(
        &self,
        client: &AsyncForgeClient,
        input: SkillInput,
    ) -> SkillResult<SkillOutput> {
        if self.skills.is_empty() {
            return Err(SkillError::CompositionError(
                "No skills in composition".to_string(),
            ));
        }

        match self.strategy {
            CompositionStrategy::Sequential => self.execute_sequential(client, input).await,
            CompositionStrategy::Parallel => self.execute_parallel(client, input).await,
            CompositionStrategy::Conditional => {
                // For now, treat conditional as sequential
                self.execute_sequential(client, input).await
            }
        }
    }
}

impl ComposedSkill {
    async fn execute_sequential(
        &self,
        client: &AsyncForgeClient,
        mut input: SkillInput,
    ) -> SkillResult<SkillOutput> {
        let mut output = SkillOutput::new("");
        let mut accumulated_data: Vec<serde_json::Value> = Vec::new();

        for (i, skill) in self.skills.iter().enumerate() {
            // Validate input for this skill
            skill.validate_input(&input)?;

            // Execute
            output = skill.execute(client, input.clone()).await.map_err(|e| {
                SkillError::ExecutionFailed(format!("Skill {} ({}) failed: {}", i, skill.name(), e))
            })?;

            // Store data if present
            if let Some(data) = &output.data {
                accumulated_data.push(data.clone());
            }

            // Use output as input for next skill
            input = SkillInput::new(&output.text);
            if !accumulated_data.is_empty() {
                input = input.with_context(serde_json::json!(accumulated_data));
            }
        }

        // Add accumulated data to final output
        if !accumulated_data.is_empty() {
            output = output.with_data(serde_json::json!(accumulated_data));
        }

        Ok(output)
    }

    async fn execute_parallel(
        &self,
        client: &AsyncForgeClient,
        input: SkillInput,
    ) -> SkillResult<SkillOutput> {
        use futures::future::join_all;

        // Execute all skills in parallel
        let futures: Vec<_> = self
            .skills
            .iter()
            .map(|skill| {
                let skill = skill.clone();
                let input = input.clone();
                let client = client.clone();
                async move { skill.execute(&client, input).await }
            })
            .collect();

        let results = join_all(futures).await;

        // Collect outputs
        let mut texts = Vec::new();
        let mut data_items = Vec::new();

        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(output) => {
                    texts.push(output.text);
                    if let Some(data) = output.data {
                        data_items.push(data);
                    }
                }
                Err(e) => {
                    return Err(SkillError::ExecutionFailed(format!(
                        "Parallel skill {} failed: {}",
                        i, e
                    )));
                }
            }
        }

        // Merge outputs
        let merged_text = texts.join("\n\n---\n\n");
        let mut output = SkillOutput::new(merged_text);

        if !data_items.is_empty() {
            output = output.with_data(serde_json::json!(data_items));
        }

        Ok(output)
    }
}

/// Builder for skill compositions.
pub struct SkillComposer {
    compositions: Vec<ComposedSkill>,
}

impl Default for SkillComposer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillComposer {
    /// Create a new skill composer.
    pub fn new() -> Self {
        Self {
            compositions: Vec::new(),
        }
    }

    /// Start a new sequential composition.
    pub fn sequential(name: impl Into<String>, description: impl Into<String>) -> ComposedSkill {
        ComposedSkill::new(name, description).with_strategy(CompositionStrategy::Sequential)
    }

    /// Start a new parallel composition.
    pub fn parallel(name: impl Into<String>, description: impl Into<String>) -> ComposedSkill {
        ComposedSkill::new(name, description).with_strategy(CompositionStrategy::Parallel)
    }

    /// Create a pipeline (sequential chain) from skills.
    pub fn pipeline<S: Skill + 'static>(
        name: impl Into<String>,
        skills: impl IntoIterator<Item = S>,
    ) -> ComposedSkill {
        let mut composed =
            ComposedSkill::new(name, "Pipeline").with_strategy(CompositionStrategy::Sequential);

        for skill in skills {
            composed = composed.add_skill(skill);
        }

        composed
    }

    /// Create a fan-out (parallel execution) from skills.
    pub fn fanout<S: Skill + 'static>(
        name: impl Into<String>,
        skills: impl IntoIterator<Item = S>,
    ) -> ComposedSkill {
        let mut composed =
            ComposedSkill::new(name, "Fanout").with_strategy(CompositionStrategy::Parallel);

        for skill in skills {
            composed = composed.add_skill(skill);
        }

        composed
    }

    /// Add a composition to the composer.
    pub fn add(&mut self, composition: ComposedSkill) {
        self.compositions.push(composition);
    }

    /// Get a composition by name.
    pub fn get(&self, name: &str) -> Option<&ComposedSkill> {
        self.compositions.iter().find(|c| c.name == name)
    }

    /// List all compositions.
    pub fn list(&self) -> Vec<&str> {
        self.compositions.iter().map(|c| c.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::PromptSkill;

    fn mock_skill(name: &str) -> PromptSkill {
        PromptSkill::simple(name, format!("{} skill", name), "Be helpful")
    }

    #[test]
    fn test_composed_skill_new() {
        let composed = ComposedSkill::new("test", "Test composition");

        assert_eq!(composed.name, "test");
        assert_eq!(composed.description, "Test composition");
        assert!(composed.skills.is_empty());
        assert_eq!(composed.strategy, CompositionStrategy::Sequential);
    }

    #[test]
    fn test_composed_skill_add_skills() {
        let composed = ComposedSkill::new("test", "Test")
            .add_skill(mock_skill("skill1"))
            .add_skill(mock_skill("skill2"));

        assert_eq!(composed.skill_count(), 2);
    }

    #[test]
    fn test_composed_skill_strategy() {
        let sequential =
            ComposedSkill::new("seq", "Sequential").with_strategy(CompositionStrategy::Sequential);
        assert_eq!(sequential.strategy(), CompositionStrategy::Sequential);

        let parallel =
            ComposedSkill::new("par", "Parallel").with_strategy(CompositionStrategy::Parallel);
        assert_eq!(parallel.strategy(), CompositionStrategy::Parallel);
    }

    #[test]
    fn test_skill_composer_sequential() {
        let composed = SkillComposer::sequential("my-pipeline", "A test pipeline")
            .add_skill(mock_skill("step1"))
            .add_skill(mock_skill("step2"));

        assert_eq!(composed.strategy(), CompositionStrategy::Sequential);
        assert_eq!(composed.skill_count(), 2);
    }

    #[test]
    fn test_skill_composer_parallel() {
        let composed = SkillComposer::parallel("my-fanout", "A test fanout")
            .add_skill(mock_skill("task1"))
            .add_skill(mock_skill("task2"));

        assert_eq!(composed.strategy(), CompositionStrategy::Parallel);
        assert_eq!(composed.skill_count(), 2);
    }

    #[test]
    fn test_skill_composer_pipeline() {
        let skills = vec![
            mock_skill("first"),
            mock_skill("second"),
            mock_skill("third"),
        ];

        let pipeline = SkillComposer::pipeline("my-pipeline", skills);

        assert_eq!(pipeline.skill_count(), 3);
        assert_eq!(pipeline.strategy(), CompositionStrategy::Sequential);
    }

    #[test]
    fn test_skill_composer_fanout() {
        let skills = vec![mock_skill("branch1"), mock_skill("branch2")];

        let fanout = SkillComposer::fanout("my-fanout", skills);

        assert_eq!(fanout.skill_count(), 2);
        assert_eq!(fanout.strategy(), CompositionStrategy::Parallel);
    }

    #[test]
    fn test_skill_composer_storage() {
        let mut composer = SkillComposer::new();

        composer.add(SkillComposer::sequential("seq1", "First").add_skill(mock_skill("a")));
        composer.add(SkillComposer::parallel("par1", "Second").add_skill(mock_skill("b")));

        let list = composer.list();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"seq1"));
        assert!(list.contains(&"par1"));

        assert!(composer.get("seq1").is_some());
        assert!(composer.get("par1").is_some());
        assert!(composer.get("nonexistent").is_none());
    }
}
