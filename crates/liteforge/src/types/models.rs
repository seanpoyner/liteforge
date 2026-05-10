//! Model types.

use serde::{Deserialize, Serialize};

/// A model available for completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier.
    pub id: String,

    /// Object type (always "model").
    pub object: String,

    /// Unix timestamp when the model was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// The organization that owns the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

/// A list of models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    /// Object type (always "list").
    pub object: String,

    /// The list of models.
    pub data: Vec<Model>,
}

impl ModelList {
    /// Get model IDs as a vector.
    pub fn ids(&self) -> Vec<&str> {
        self.data.iter().map(|m| m.id.as_str()).collect()
    }

    /// Find a model by ID.
    pub fn find(&self, id: &str) -> Option<&Model> {
        self.data.iter().find(|m| m.id == id)
    }
}
