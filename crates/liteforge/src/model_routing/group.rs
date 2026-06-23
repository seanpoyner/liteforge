//! Model groups and capability tiers.
//!
//! A [`GroupCatalog`] describes the deployment groups a selector can route to,
//! with an ordered [`CapabilityTier`] and optional cost signal. The catalog is a
//! Layer-2 concept: Layer-1 only ever sees group *names*, while selectors use the
//! tier/cost metadata to map a difficulty score to a concrete group.

use serde::{Deserialize, Serialize};

/// Capability/cost tier, ordered low to high. The `Ord` derive gives tier
/// comparison for free (used by the MF tier policy).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[non_exhaustive]
pub enum CapabilityTier {
    /// Smallest / cheapest tier.
    #[serde(alias = "nano", alias = "NANO")]
    Nano,
    /// Small models.
    #[serde(alias = "small", alias = "SMALL")]
    Small,
    /// Mid-range models.
    #[serde(alias = "medium", alias = "MEDIUM")]
    Medium,
    /// Large models.
    #[serde(alias = "large", alias = "LARGE")]
    Large,
    /// Frontier / most capable (and most expensive) tier.
    #[serde(alias = "frontier", alias = "FRONTIER")]
    Frontier,
}

impl CapabilityTier {
    /// Ordinal rank (0 = lowest tier).
    pub fn rank(&self) -> u8 {
        match self {
            CapabilityTier::Nano => 0,
            CapabilityTier::Small => 1,
            CapabilityTier::Medium => 2,
            CapabilityTier::Large => 3,
            CapabilityTier::Frontier => 4,
        }
    }
}

/// One routable model group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGroup {
    /// Group name (matches a `model_name` in the router's model_list).
    pub name: String,
    /// Concrete model ids in this group (Layer-1 load-balances among these).
    #[serde(default)]
    pub models: Vec<String>,
    /// Capability/cost tier.
    pub tier: CapabilityTier,
    /// Optional cost-per-1k-tokens signal.
    #[serde(default)]
    pub cost_per_1k: Option<f32>,
    /// Free-form tags (e.g. `code`, `vision`).
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ModelGroup {
    /// Create a group with a name and tier.
    pub fn new(name: impl Into<String>, tier: CapabilityTier) -> Self {
        Self {
            name: name.into(),
            models: Vec::new(),
            tier,
            cost_per_1k: None,
            tags: Vec::new(),
        }
    }
}

/// An ordered collection of model groups.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupCatalog {
    /// The groups, in declaration order.
    pub groups: Vec<ModelGroup>,
}

impl GroupCatalog {
    /// Build a catalog from a list of groups.
    pub fn new(groups: Vec<ModelGroup>) -> Self {
        Self { groups }
    }

    /// Group names in declaration order.
    pub fn names(&self) -> Vec<&str> {
        self.groups.iter().map(|g| g.name.as_str()).collect()
    }

    /// Look up a group by name.
    pub fn by_name(&self, name: &str) -> Option<&ModelGroup> {
        self.groups.iter().find(|g| g.name == name)
    }

    /// Groups sorted ascending by tier (ties keep declaration order).
    pub fn ordered_by_tier(&self) -> Vec<&ModelGroup> {
        let mut out: Vec<&ModelGroup> = self.groups.iter().collect();
        out.sort_by_key(|g| g.tier.rank());
        out
    }

    /// Distinct tiers present, ascending.
    pub fn tiers_present(&self) -> Vec<CapabilityTier> {
        let mut tiers: Vec<CapabilityTier> = self.groups.iter().map(|g| g.tier).collect();
        tiers.sort_by_key(|t| t.rank());
        tiers.dedup();
        tiers
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_by_tier_sorts_ascending() {
        let cat = GroupCatalog::new(vec![
            ModelGroup::new("premium", CapabilityTier::Frontier),
            ModelGroup::new("cheap", CapabilityTier::Small),
            ModelGroup::new("balanced", CapabilityTier::Medium),
        ]);
        let ordered: Vec<&str> = cat.ordered_by_tier().iter().map(|g| g.name.as_str()).collect();
        assert_eq!(ordered, vec!["cheap", "balanced", "premium"]);
        assert_eq!(
            cat.tiers_present(),
            vec![
                CapabilityTier::Small,
                CapabilityTier::Medium,
                CapabilityTier::Frontier
            ]
        );
    }

    #[test]
    fn tier_deserializes_case_insensitively() {
        let t: CapabilityTier = serde_yaml::from_str("frontier").unwrap();
        assert_eq!(t, CapabilityTier::Frontier);
        let t2: CapabilityTier = serde_yaml::from_str("Medium").unwrap();
        assert_eq!(t2, CapabilityTier::Medium);
    }
}
