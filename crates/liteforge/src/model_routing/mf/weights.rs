//! Matrix-factorization router weights.
//!
//! The weights are a native port of RouteLLM's `MFModel` (Apache-2.0): an
//! embedding row per anchor model, an optional projection of the prompt
//! embedding, and a linear classifier. They are tiny (a few hundred KB), so we
//! store them as plain JSON loaded with the existing `serde_json` dependency
//! rather than pulling a tensor format.

use crate::error::{ForgeError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Current weights schema version understood by this loader.
pub const MF_WEIGHTS_VERSION: u32 = 1;

/// Learned MF weights, retrained against a specific embedding model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfWeights {
    /// Schema version.
    pub version: u32,
    /// Embedding model the weights were trained against (e.g. `bge-m3`).
    pub embedding_model: String,
    /// Prompt embedding dimensionality (e.g. 1024 for bge-m3).
    pub text_dim: usize,
    /// Latent dimension `d`.
    pub d: usize,
    /// Number of classifier output classes.
    pub num_classes: usize,

    /// Learned embedding row for the strong anchor model (length `d`).
    pub strong_row: Vec<f32>,
    /// Learned embedding row for the weak anchor model (length `d`).
    pub weak_row: Vec<f32>,

    /// Whether the prompt embedding is projected `text_dim -> d`.
    pub use_proj: bool,
    /// Projection weight, row-major `[text_dim * d]` (required if `use_proj`).
    #[serde(default)]
    pub proj_w: Option<Vec<f32>>,
    /// Projection bias, length `d` (optional even when `use_proj`).
    #[serde(default)]
    pub proj_b: Option<Vec<f32>>,

    /// Classifier weight, row-major `[d * num_classes]`.
    pub cls_w: Vec<f32>,
    /// Classifier bias, length `num_classes`.
    pub cls_b: Vec<f32>,

    /// Class index used for the strong-model logit.
    pub strong_class: usize,
    /// Class index used for the weak-model logit.
    pub weak_class: usize,
}

impl MfWeights {
    /// Load weights from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| {
            ForgeError::config(format!(
                "could not read MF weights {:?}: {e}",
                path.as_ref()
            ))
        })?;
        let w: MfWeights = serde_json::from_slice(&bytes)?;
        w.validate()?;
        Ok(w)
    }

    /// Parse weights from a JSON string.
    pub fn from_json_str(s: &str) -> Result<Self> {
        let w: MfWeights = serde_json::from_str(s)?;
        w.validate()?;
        Ok(w)
    }

    /// Validate internal shape consistency.
    pub fn validate(&self) -> Result<()> {
        if self.version != MF_WEIGHTS_VERSION {
            return Err(ForgeError::config(format!(
                "unsupported MF weights version {} (expected {})",
                self.version, MF_WEIGHTS_VERSION
            )));
        }
        let check = |name: &str, got: usize, want: usize| -> Result<()> {
            if got != want {
                Err(ForgeError::config(format!(
                    "MF weights: {name} length {got}, expected {want}"
                )))
            } else {
                Ok(())
            }
        };
        check("strong_row", self.strong_row.len(), self.d)?;
        check("weak_row", self.weak_row.len(), self.d)?;
        check("cls_w", self.cls_w.len(), self.d * self.num_classes)?;
        check("cls_b", self.cls_b.len(), self.num_classes)?;
        if self.strong_class >= self.num_classes || self.weak_class >= self.num_classes {
            return Err(ForgeError::config(
                "MF weights: strong_class/weak_class out of range",
            ));
        }
        if self.use_proj {
            match &self.proj_w {
                Some(w) => check("proj_w", w.len(), self.text_dim * self.d)?,
                None => {
                    return Err(ForgeError::config(
                        "MF weights: use_proj is true but proj_w is missing",
                    ))
                }
            }
            if let Some(b) = &self.proj_b {
                check("proj_b", b.len(), self.d)?;
            }
        } else if self.text_dim != self.d {
            return Err(ForgeError::config(
                "MF weights: use_proj is false but text_dim != d",
            ));
        }
        Ok(())
    }
}
