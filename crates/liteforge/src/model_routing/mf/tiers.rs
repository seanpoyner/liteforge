//! Tier policy: map the MF scalar hardness to N ordered model groups.
//!
//! MF is intrinsically binary (strong vs weak), so it emits a single hardness
//! score in (0, 1). The tier policy buckets that score across the capability
//! tiers present in the catalog and returns the chosen tier's group(s) first,
//! then neighbouring tiers as ranked fallbacks. This yields an N-way ranking
//! from a single scalar without needing N-way training data.

use crate::model_routing::group::GroupCatalog;
use crate::routing::ScoredGroup;
use serde::Deserialize;

/// Whether a higher hardness score means a higher (more capable) tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TierDirection {
    /// Higher score routes to a higher tier (the usual case).
    #[default]
    #[serde(alias = "higher", alias = "higher-is-harder")]
    HigherIsHarder,
    /// Higher score routes to a lower tier.
    #[serde(alias = "lower", alias = "lower-is-harder")]
    LowerIsHarder,
}

/// Buckets a hardness score into ordered tiers.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TierPolicy {
    /// Ascending cut points in (0, 1). For `n` tiers, provide `n-1` thresholds;
    /// when empty, the (0, 1) range is split evenly across the tiers present.
    #[serde(default)]
    pub thresholds: Vec<f32>,
    /// Direction mapping score to tier.
    #[serde(default)]
    pub direction: TierDirection,
}

impl TierPolicy {
    /// Create a policy with explicit thresholds.
    pub fn new(thresholds: Vec<f32>) -> Self {
        Self {
            thresholds,
            direction: TierDirection::default(),
        }
    }

    /// Rank the catalog's groups for a given hardness score `s`.
    ///
    /// The chosen tier ranks first (score 1.0); other tiers rank by their
    /// distance from the chosen tier so the router can degrade gracefully.
    pub fn rank(&self, s: f32, catalog: &GroupCatalog) -> Vec<ScoredGroup> {
        let tiers = catalog.tiers_present();
        let n = tiers.len();
        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return catalog
                .groups
                .iter()
                .map(|g| ScoredGroup::new(&g.name, 1.0).with_reason("mf: single tier"))
                .collect();
        }

        // Effective thresholds: explicit, or an even split of (0, 1).
        let thresholds: Vec<f32> = if self.thresholds.is_empty() {
            (1..n).map(|i| i as f32 / n as f32).collect()
        } else {
            self.thresholds.clone()
        };

        let bucket = thresholds.iter().filter(|&&t| s >= t).count().min(n - 1);
        let chosen_index = match self.direction {
            TierDirection::HigherIsHarder => bucket,
            TierDirection::LowerIsHarder => n - 1 - bucket,
        };
        let chosen_tier = tiers[chosen_index];

        // Score each group by how close its tier is to the chosen tier.
        let mut out: Vec<ScoredGroup> = catalog
            .groups
            .iter()
            .map(|g| {
                let g_index = tiers.iter().position(|t| *t == g.tier).unwrap_or(0);
                let distance = g_index.abs_diff(chosen_index);
                let score = 1.0 - (distance as f32) / (n as f32);
                let reason = if g.tier == chosen_tier {
                    format!("mf: hardness {s:.3} -> chosen tier")
                } else {
                    format!("mf: hardness {s:.3} -> fallback (+{distance} tier)")
                };
                ScoredGroup::new(&g.name, score).with_reason(reason)
            })
            .collect();

        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_routing::group::{CapabilityTier, ModelGroup};

    fn catalog() -> GroupCatalog {
        GroupCatalog::new(vec![
            ModelGroup::new("cheap", CapabilityTier::Small),
            ModelGroup::new("balanced", CapabilityTier::Medium),
            ModelGroup::new("premium", CapabilityTier::Frontier),
        ])
    }

    #[test]
    fn low_score_picks_lowest_tier() {
        let p = TierPolicy::new(vec![0.33, 0.66]);
        let ranked = p.rank(0.1, &catalog());
        assert_eq!(ranked[0].group, "cheap");
    }

    #[test]
    fn high_score_picks_top_tier_then_neighbors() {
        let p = TierPolicy::new(vec![0.33, 0.66]);
        let ranked = p.rank(0.9, &catalog());
        assert_eq!(ranked[0].group, "premium");
        // Next-closest tier should outrank the farthest.
        assert_eq!(ranked[1].group, "balanced");
        assert_eq!(ranked[2].group, "cheap");
    }

    #[test]
    fn mid_score_picks_middle_tier() {
        let p = TierPolicy::new(vec![0.33, 0.66]);
        let ranked = p.rank(0.5, &catalog());
        assert_eq!(ranked[0].group, "balanced");
    }

    #[test]
    fn lower_is_harder_inverts() {
        let p = TierPolicy {
            thresholds: vec![0.33, 0.66],
            direction: TierDirection::LowerIsHarder,
        };
        let ranked = p.rank(0.9, &catalog());
        assert_eq!(ranked[0].group, "cheap");
    }

    #[test]
    fn empty_thresholds_splits_evenly() {
        let p = TierPolicy::default();
        assert_eq!(p.rank(0.1, &catalog())[0].group, "cheap");
        assert_eq!(p.rank(0.9, &catalog())[0].group, "premium");
    }
}
