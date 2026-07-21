//! Decision cache.
//!
//! Selectors fetch an embedding (or call a classifier) per request, which adds a
//! network round-trip on the hot path. The cache stores the resulting ranked
//! groups keyed by a hash of (selector name, prompt, catalog signature), so
//! repeated/identical prompts skip the network call. A small hand-rolled
//! bounded LRU with optional TTL keeps the dependency footprint at zero.

use crate::routing::ScoredGroup;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// FNV-1a 64-bit hash over the parts that make a decision unique.
pub fn decision_key(selector: &str, prompt: &str, catalog_sig: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for part in [
        selector.as_bytes(),
        b"\x1f",
        prompt.as_bytes(),
        b"\x1f",
        catalog_sig.as_bytes(),
    ] {
        for &b in part {
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
        }
    }
    h
}

struct Entry {
    groups: Vec<ScoredGroup>,
    inserted: Instant,
}

struct Inner {
    map: HashMap<u64, Entry>,
    order: VecDeque<u64>,
}

/// Bounded LRU cache of routing decisions.
pub struct DecisionCache {
    inner: Mutex<Inner>,
    capacity: usize,
    ttl: Option<Duration>,
}

impl DecisionCache {
    /// Create a cache with the given capacity and optional TTL.
    pub fn new(capacity: usize, ttl: Option<Duration>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
            capacity: capacity.max(1),
            ttl,
        }
    }

    /// Look up a cached decision; returns a clone if present and not expired.
    pub fn get(&self, key: u64) -> Option<Vec<ScoredGroup>> {
        let mut inner = self.inner.lock().ok()?;
        let expired = match (&self.ttl, inner.map.get(&key)) {
            (Some(ttl), Some(e)) => e.inserted.elapsed() > *ttl,
            (_, Some(_)) => false,
            (_, None) => return None,
        };
        if expired {
            inner.map.remove(&key);
            inner.order.retain(|k| *k != key);
            return None;
        }
        // Move to most-recently-used.
        inner.order.retain(|k| *k != key);
        inner.order.push_back(key);
        inner.map.get(&key).map(|e| e.groups.clone())
    }

    /// Insert a decision, evicting the least-recently-used entry if at capacity.
    pub fn put(&self, key: u64, groups: Vec<ScoredGroup>) {
        let mut inner = if let Ok(g) = self.inner.lock() {
            g
        } else {
            return;
        };
        if inner.map.contains_key(&key) {
            inner.order.retain(|k| *k != key);
        } else if inner.map.len() >= self.capacity {
            if let Some(old) = inner.order.pop_front() {
                inner.map.remove(&old);
            }
        }
        inner.map.insert(
            key,
            Entry {
                groups,
                inserted: Instant::now(),
            },
        );
        inner.order.push_back(key);
    }

    /// Remove all entries.
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.map.clear();
            inner.order.clear();
        }
    }

    /// Current number of cached entries.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|i| i.map.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for DecisionCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecisionCache")
            .field("capacity", &self.capacity)
            .field("ttl", &self.ttl)
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(name: &str) -> Vec<ScoredGroup> {
        vec![ScoredGroup::new(name, 1.0)]
    }

    #[test]
    fn key_is_stable_and_sensitive() {
        let a = decision_key("mf", "hello", "sig");
        let b = decision_key("mf", "hello", "sig");
        assert_eq!(a, b);
        assert_ne!(a, decision_key("mf", "world", "sig"));
        assert_ne!(a, decision_key("semantic", "hello", "sig"));
        assert_ne!(a, decision_key("mf", "hello", "sig2"));
    }

    #[test]
    fn lru_evicts_oldest() {
        let c = DecisionCache::new(2, None);
        c.put(1, g("a"));
        c.put(2, g("b"));
        assert!(c.get(1).is_some());
        // Access 1 so 2 becomes LRU, then insert 3 -> evict 2.
        c.put(3, g("c"));
        assert!(c.get(2).is_none());
        assert!(c.get(1).is_some());
        assert!(c.get(3).is_some());
    }

    #[test]
    fn ttl_expires_entries() {
        let c = DecisionCache::new(4, Some(Duration::from_millis(1)));
        c.put(1, g("a"));
        std::thread::sleep(Duration::from_millis(5));
        assert!(c.get(1).is_none());
    }
}
