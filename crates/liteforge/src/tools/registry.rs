//! Tool registry for managing collections of tools.

use super::{Tool, ToolDefinition};
use std::collections::HashMap;
use std::sync::Arc;

/// A registry for storing and managing tools.
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry.
    ///
    /// If a tool with the same name already exists, it will be replaced.
    pub fn register(&mut self, tool: Box<dyn Tool>) -> &mut Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::from(tool));
        self
    }

    /// Register a tool wrapped in an Arc.
    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) -> &mut Self {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
        self
    }

    /// Unregister a tool by name.
    ///
    /// Returns the tool if it was found.
    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.remove(name)
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists in the registry.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Get all tool names.
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get all registered tools.
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Get tool definitions for all registered tools.
    ///
    /// Returns definitions in the OpenAI API format.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.to_definition()).collect()
    }

    /// Merge another registry into this one.
    ///
    /// Tools from the other registry will overwrite tools with the same name.
    pub fn merge(&mut self, other: ToolRegistry) -> &mut Self {
        self.tools.extend(other.tools);
        self
    }

    /// Create a new registry containing only tools that match the filter.
    pub fn filter<F>(&self, predicate: F) -> ToolRegistry
    where
        F: Fn(&dyn Tool) -> bool,
    {
        let tools = self
            .tools
            .iter()
            .filter(|(_, tool)| predicate(tool.as_ref()))
            .map(|(name, tool)| (name.clone(), tool.clone()))
            .collect();

        ToolRegistry { tools }
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    struct DummyTool {
        name: String,
    }

    impl Tool for DummyTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A dummy tool"
        }

        fn parameters_schema(&self) -> Value {
            json!({"type": "object", "properties": {}})
        }

        fn execute(&self, _args: Value) -> Result<Value, String> {
            Ok(json!({"status": "ok"}))
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool {
            name: "tool1".to_string(),
        }));

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("tool1"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool {
            name: "tool1".to_string(),
        }));

        let tool = registry.get("tool1");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "tool1");

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool {
            name: "tool1".to_string(),
        }));

        let removed = registry.unregister("tool1");
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_names() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Box::new(DummyTool {
                name: "tool1".to_string(),
            }))
            .register(Box::new(DummyTool {
                name: "tool2".to_string(),
            }));

        let names = registry.names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool1"));
        assert!(names.contains(&"tool2"));
    }

    #[test]
    fn test_registry_definitions() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool {
            name: "tool1".to_string(),
        }));

        let defs = registry.definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "tool1");
    }

    #[test]
    fn test_registry_filter() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Box::new(DummyTool {
                name: "get_weather".to_string(),
            }))
            .register(Box::new(DummyTool {
                name: "set_alarm".to_string(),
            }))
            .register(Box::new(DummyTool {
                name: "get_time".to_string(),
            }));

        let filtered = registry.filter(|t| t.name().starts_with("get_"));
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains("get_weather"));
        assert!(filtered.contains("get_time"));
        assert!(!filtered.contains("set_alarm"));
    }
}
