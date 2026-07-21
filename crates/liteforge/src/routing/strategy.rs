//! Deployment selection strategies.
//!
//! A strategy picks one candidate from an already-filtered, non-empty list
//! (cooled-down and zero-weight deployments are removed before the strategy
//! runs). Strategies are `Send + Sync` and may carry internal atomic state
//! (e.g. the round-robin cursor), so the router holds them behind `Arc`.

use super::deployment::Deployment;
use super::health::HealthSnapshot;
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A candidate handed to a [`SelectionStrategy`]: deployment metadata plus a
/// live health snapshot.
#[derive(Debug, Clone, Copy)]
pub struct Candidate<'a> {
    /// The deployment under consideration.
    pub deployment: &'a Deployment,
    /// Its health snapshot at selection time.
    pub health: HealthSnapshot,
}

/// Picks one index from a non-empty candidate slice.
pub trait SelectionStrategy: Send + Sync {
    /// Return the index (into `candidates`) of the chosen deployment.
    fn select(&self, candidates: &[Candidate<'_>]) -> usize;
    /// Stable name for logging / introspection.
    fn name(&self) -> &'static str;
}

/// The load-balancing strategy for a model group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum RoutingStrategy {
    /// Weighted random selection (the LiteLLM default).
    #[default]
    #[serde(rename = "simple-shuffle", alias = "simple_shuffle")]
    SimpleShuffle,
    /// Cycle through live candidates in order.
    #[serde(rename = "round-robin", alias = "round_robin")]
    RoundRobin,
    /// Pick the candidate with the fewest in-flight requests.
    #[serde(rename = "least-busy", alias = "least_busy")]
    LeastBusy,
    /// Pick the candidate with the lowest smoothed latency (unmeasured first).
    #[serde(
        rename = "latency-based-routing",
        alias = "latency-based",
        alias = "latency_based"
    )]
    LatencyBased,
}

impl RoutingStrategy {
    /// Construct the concrete strategy implementation. `RoundRobin` gets its
    /// own cursor instance.
    pub fn build(&self) -> Arc<dyn SelectionStrategy> {
        match self {
            RoutingStrategy::SimpleShuffle => Arc::new(SimpleShuffle::new()),
            RoutingStrategy::RoundRobin => Arc::new(RoundRobin::new()),
            RoutingStrategy::LeastBusy => Arc::new(LeastBusy),
            RoutingStrategy::LatencyBased => Arc::new(LatencyBased),
        }
    }
}

/// SplitMix64 PRNG seeded from the clock plus a per-instance counter.
///
/// Non-cryptographic; suitable only for load balancing. Each `next()` call
/// advances the counter so concurrent callers see different streams.
#[derive(Debug)]
pub struct SimpleShuffle {
    state: AtomicU64,
}

impl Default for SimpleShuffle {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleShuffle {
    /// Create a new weighted-random strategy with a clock-derived seed.
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            ^ 0xD1B5_4A32_D192_ED03;
        Self {
            state: AtomicU64::new(seed | 1),
        }
    }

    /// Create with a fixed seed (deterministic; used in tests).
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: AtomicU64::new(seed | 1),
        }
    }

    fn next_u64(&self) -> u64 {
        // Advance the shared state, then run the SplitMix64 finaliser.
        let z = self
            .state
            .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        let mut z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl SelectionStrategy for SimpleShuffle {
    fn select(&self, candidates: &[Candidate<'_>]) -> usize {
        let total: u64 = candidates.iter().map(|c| c.deployment.weight as u64).sum();
        if total == 0 {
            // All weights zero: fall back to uniform selection.
            return (self.next_u64() % candidates.len() as u64) as usize;
        }
        let mut pick = self.next_u64() % total;
        for (i, c) in candidates.iter().enumerate() {
            let w = c.deployment.weight as u64;
            if pick < w {
                return i;
            }
            pick -= w;
        }
        candidates.len() - 1
    }

    fn name(&self) -> &'static str {
        "simple-shuffle"
    }
}

/// Round-robin over the (post-filter) candidate list.
#[derive(Debug, Default)]
pub struct RoundRobin {
    cursor: AtomicU64,
}

impl RoundRobin {
    /// Create a new round-robin strategy.
    pub fn new() -> Self {
        Self {
            cursor: AtomicU64::new(0),
        }
    }
}

impl SelectionStrategy for RoundRobin {
    fn select(&self, candidates: &[Candidate<'_>]) -> usize {
        let n = self.cursor.fetch_add(1, Ordering::Relaxed);
        (n % candidates.len() as u64) as usize
    }

    fn name(&self) -> &'static str {
        "round-robin"
    }
}

/// Pick the candidate with the fewest in-flight requests (ties: lowest index).
#[derive(Debug, Default)]
pub struct LeastBusy;

impl SelectionStrategy for LeastBusy {
    fn select(&self, candidates: &[Candidate<'_>]) -> usize {
        candidates
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| c.health.in_flight)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn name(&self) -> &'static str {
        "least-busy"
    }
}

/// Pick the candidate with the lowest smoothed latency. Unmeasured
/// deployments sort first so they get a probing request (cold-start).
#[derive(Debug, Default)]
pub struct LatencyBased;

impl SelectionStrategy for LatencyBased {
    fn select(&self, candidates: &[Candidate<'_>]) -> usize {
        candidates
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| c.health.ewma_latency_us.unwrap_or(0))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn name(&self) -> &'static str {
        "latency-based-routing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::deployment::DeploymentId;

    fn snap(in_flight: u32, ewma: Option<u64>) -> HealthSnapshot {
        HealthSnapshot {
            in_flight,
            ewma_latency_us: ewma,
            cooled_down: false,
        }
    }

    fn dep(weight: u32) -> Deployment {
        let mut d = Deployment::new(DeploymentId(0), "g", "m", "http://x");
        d.weight = weight;
        d
    }

    #[test]
    fn round_robin_cycles() {
        let rr = RoundRobin::new();
        let deps = [dep(1), dep(1), dep(1)];
        let cands: Vec<Candidate> = deps
            .iter()
            .map(|d| Candidate {
                deployment: d,
                health: snap(0, None),
            })
            .collect();
        let picks: Vec<usize> = (0..6).map(|_| rr.select(&cands)).collect();
        assert_eq!(picks, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn least_busy_picks_min_in_flight() {
        let deps = [dep(1), dep(1), dep(1)];
        let cands = vec![
            Candidate {
                deployment: &deps[0],
                health: snap(5, None),
            },
            Candidate {
                deployment: &deps[1],
                health: snap(1, None),
            },
            Candidate {
                deployment: &deps[2],
                health: snap(3, None),
            },
        ];
        assert_eq!(LeastBusy.select(&cands), 1);
    }

    #[test]
    fn latency_based_prefers_unmeasured_then_lowest() {
        let deps = [dep(1), dep(1)];
        // Both measured: lowest wins.
        let cands = vec![
            Candidate {
                deployment: &deps[0],
                health: snap(0, Some(900)),
            },
            Candidate {
                deployment: &deps[1],
                health: snap(0, Some(200)),
            },
        ];
        assert_eq!(LatencyBased.select(&cands), 1);
    }

    #[test]
    fn weighted_shuffle_respects_weights() {
        let deps = [dep(0), dep(10)];
        let cands = vec![
            Candidate {
                deployment: &deps[0],
                health: snap(0, None),
            },
            Candidate {
                deployment: &deps[1],
                health: snap(0, None),
            },
        ];
        let ss = SimpleShuffle::with_seed(42);
        // Index 0 has weight 0 and must never be picked.
        for _ in 0..1000 {
            assert_eq!(ss.select(&cands), 1);
        }
    }

    #[test]
    fn weighted_shuffle_distribution_is_roughly_proportional() {
        let deps = [dep(1), dep(3)];
        let cands = vec![
            Candidate {
                deployment: &deps[0],
                health: snap(0, None),
            },
            Candidate {
                deployment: &deps[1],
                health: snap(0, None),
            },
        ];
        let ss = SimpleShuffle::with_seed(12345);
        let mut counts = [0usize; 2];
        for _ in 0..4000 {
            counts[ss.select(&cands)] += 1;
        }
        // Expect ~25% / ~75%. Allow generous tolerance for the tiny PRNG.
        let ratio = counts[1] as f64 / 4000.0;
        assert!(ratio > 0.68 && ratio < 0.82, "ratio was {ratio}");
    }
}
