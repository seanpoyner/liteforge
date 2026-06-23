//! Per-deployment health tracking.
//!
//! The set of deployments is fixed at router-build time, so health is stored
//! as a vector of plain atomics indexed by [`DeploymentId`](super::DeploymentId).
//! This avoids any lock or concurrent map on the hot path: selection reads
//! atomics, request completion updates atomics. EWMA latency is stored as the
//! bit pattern of an `f64` in an `AtomicU64`, updated with a CAS loop (a lost
//! race merely drops one latency sample, which is fine for a smoothing average).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Current unix time in milliseconds (0 if the clock is before the epoch).
pub(crate) fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Live health counters for a single deployment.
#[derive(Debug)]
pub struct DeploymentHealth {
    in_flight: AtomicU32,
    consecutive_failures: AtomicU32,
    ewma_latency_us: AtomicU64,
    ewma_initialized: AtomicBool,
    /// Cooldown expiry as unix millis; `0` means not cooled down.
    cooldown_until_ms: AtomicU64,
}

impl Default for DeploymentHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl DeploymentHealth {
    /// Create a fresh, healthy deployment.
    pub fn new() -> Self {
        Self {
            in_flight: AtomicU32::new(0),
            consecutive_failures: AtomicU32::new(0),
            ewma_latency_us: AtomicU64::new(0),
            ewma_initialized: AtomicBool::new(false),
            cooldown_until_ms: AtomicU64::new(0),
        }
    }

    /// Increment the in-flight counter and return an RAII guard that
    /// decrements it on drop (so cancellation / early return cannot leak).
    pub fn on_request_start(&self) -> InFlightGuard<'_> {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        InFlightGuard { health: self }
    }

    /// Like [`on_request_start`](Self::on_request_start) but returns an owned
    /// guard that can outlive the borrow (used by the streaming path, where the
    /// guard must travel with the returned `'static` stream).
    pub fn start_owned(self: &Arc<Self>) -> OwnedInFlightGuard {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        OwnedInFlightGuard {
            health: Arc::clone(self),
        }
    }

    /// Record a successful request: reset failures and update EWMA latency.
    pub fn record_success(&self, latency: Duration, alpha: f64) {
        self.consecutive_failures.store(0, Ordering::Release);
        self.cooldown_until_ms.store(0, Ordering::Release);

        let sample = latency.as_micros() as f64;
        if !self.ewma_initialized.swap(true, Ordering::AcqRel) {
            self.ewma_latency_us.store(sample.to_bits(), Ordering::Release);
            return;
        }
        // CAS loop blending the new sample into the EWMA.
        loop {
            let cur_bits = self.ewma_latency_us.load(Ordering::Acquire);
            let cur = f64::from_bits(cur_bits);
            let next = alpha * sample + (1.0 - alpha) * cur;
            if self
                .ewma_latency_us
                .compare_exchange_weak(
                    cur_bits,
                    next.to_bits(),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                break;
            }
        }
    }

    /// Record a failure. When consecutive failures reach `allowed_fails`,
    /// the deployment is cooled down for `cooldown` starting at `now_ms`.
    pub fn record_failure(&self, allowed_fails: u32, cooldown: Duration, now_ms: u64) {
        let fails = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        if allowed_fails > 0 && fails >= allowed_fails {
            let until = now_ms.saturating_add(cooldown.as_millis() as u64);
            self.cooldown_until_ms.store(until, Ordering::Release);
        }
    }

    /// Whether the deployment is currently cooled down at `now_ms`.
    pub fn is_cooled_down(&self, now_ms: u64) -> bool {
        let until = self.cooldown_until_ms.load(Ordering::Acquire);
        until != 0 && now_ms < until
    }

    /// Number of consecutive failures since the last success.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Acquire)
    }

    /// Take a cheap point-in-time snapshot for the selection strategy.
    pub fn snapshot(&self, now_ms: u64) -> HealthSnapshot {
        let ewma = if self.ewma_initialized.load(Ordering::Acquire) {
            Some(f64::from_bits(self.ewma_latency_us.load(Ordering::Acquire)) as u64)
        } else {
            None
        };
        HealthSnapshot {
            in_flight: self.in_flight.load(Ordering::Acquire),
            ewma_latency_us: ewma,
            cooled_down: self.is_cooled_down(now_ms),
        }
    }
}

/// Decrements the in-flight counter when dropped.
pub struct InFlightGuard<'a> {
    health: &'a DeploymentHealth,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.health.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Owned in-flight guard; decrements the counter when dropped.
pub struct OwnedInFlightGuard {
    health: Arc<DeploymentHealth>,
}

impl Drop for OwnedInFlightGuard {
    fn drop(&mut self) {
        self.health.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Cheap read-only view of a deployment's health used during selection.
#[derive(Debug, Clone, Copy)]
pub struct HealthSnapshot {
    /// Number of requests currently in flight to this deployment.
    pub in_flight: u32,
    /// Smoothed latency in microseconds, or `None` if not yet measured.
    pub ewma_latency_us: Option<u64>,
    /// Whether the deployment is cooled down right now.
    pub cooled_down: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_enters_after_allowed_fails_and_expires() {
        let h = DeploymentHealth::new();
        let now = 1_000_000u64;
        h.record_failure(3, Duration::from_secs(60), now);
        assert!(!h.is_cooled_down(now));
        h.record_failure(3, Duration::from_secs(60), now);
        assert!(!h.is_cooled_down(now));
        h.record_failure(3, Duration::from_secs(60), now);
        assert!(h.is_cooled_down(now));
        // Still cooled just before expiry, healthy after.
        assert!(h.is_cooled_down(now + 59_000));
        assert!(!h.is_cooled_down(now + 60_001));
    }

    #[test]
    fn success_resets_failures_and_cooldown() {
        let h = DeploymentHealth::new();
        let now = 500u64;
        h.record_failure(1, Duration::from_secs(30), now);
        assert!(h.is_cooled_down(now));
        h.record_success(Duration::from_millis(10), 0.3);
        assert!(!h.is_cooled_down(now));
        assert_eq!(h.consecutive_failures(), 0);
    }

    #[test]
    fn in_flight_guard_balances() {
        let h = DeploymentHealth::new();
        {
            let _g1 = h.on_request_start();
            let _g2 = h.on_request_start();
            assert_eq!(h.snapshot(0).in_flight, 2);
        }
        assert_eq!(h.snapshot(0).in_flight, 0);
    }

    #[test]
    fn ewma_initializes_then_blends() {
        let h = DeploymentHealth::new();
        h.record_success(Duration::from_micros(1000), 0.5);
        assert_eq!(h.snapshot(0).ewma_latency_us, Some(1000));
        h.record_success(Duration::from_micros(3000), 0.5);
        // 0.5*3000 + 0.5*1000 = 2000
        assert_eq!(h.snapshot(0).ewma_latency_us, Some(2000));
    }
}
