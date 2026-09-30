//! When an insertion would exceed a bound, expired entries are dropped first
//! and then the oldest insertions are evicted until the store is back at three
//! quarters of both limits, which amortizes eviction work. A memoized call
//! whose entry is not cached simply runs uncached.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

mod exact_json;
#[doc(hidden)]
pub use exact_json::is_exact_json;

/// Maximum number of memoized entries kept by the process.
const MAX_ENTRIES: usize = 4_096;
/// Maximum key plus value bytes of one memoized entry.
const MAX_ENTRY_BYTES: usize = 256 * 1024;
/// Maximum key plus value bytes of all memoized entries.
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
/// Lifetime of a memoized entry.
const TTL: Duration = Duration::from_secs(3600);

static GLOBAL_MEMO_CACHE: OnceLock<MemoStore> = OnceLock::new();

fn get_cache() -> &'static MemoStore {
    GLOBAL_MEMO_CACHE.get_or_init(|| {
        MemoStore::new(Limits {
            max_entries: MAX_ENTRIES,
            max_entry_bytes: MAX_ENTRY_BYTES,
            max_total_bytes: MAX_TOTAL_BYTES,
            ttl: TTL,
        })
    })
}

/// Retrieve a value from the global memoize cache.
pub fn get(key: &str) -> Option<String> {
    get_cache().get(key).map(|value| value.to_string())
}

/// Store a value in the global memoize cache for one hour.
///
/// The entry is skipped when its key plus value exceeds 256 KiB; the oldest
/// entries are evicted when the store would exceed 4,096 entries or 32 MiB.
pub fn set(key: &str, value: &str) {
    get_cache().set(key, value);
}

#[derive(Clone, Copy)]
struct Limits {
    max_entries: usize,
    max_entry_bytes: usize,
    max_total_bytes: usize,
    ttl: Duration,
}

struct Entry {
    value: Arc<str>,
    expires_at: Instant,
    inserted: u64,
    bytes: usize,
}

#[derive(Default)]
struct State {
    entries: HashMap<Box<str>, Entry>,
    total_bytes: usize,
    next_insertion: u64,
}

impl State {
    fn remove(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.bytes);
        }
    }

    fn retain(&mut self, keep: impl Fn(&Entry) -> bool) {
        self.entries.retain(|_, entry| keep(entry));
        self.total_bytes = self.entries.values().map(|entry| entry.bytes).sum();
    }

    fn fits(&self, limits: &Limits, bytes: usize) -> bool {
        self.entries.len() < limits.max_entries
            && self.total_bytes.saturating_add(bytes) <= limits.max_total_bytes
    }

    /// Evicts the oldest insertions until the store and the new entry fit
    /// within three quarters of both limits.
    fn evict_oldest(&mut self, limits: &Limits, bytes: usize) {
        let entry_target = limits.max_entries - limits.max_entries / 4;
        let byte_target = limits.max_total_bytes - limits.max_total_bytes / 4;
        let mut order: Vec<(u64, usize)> = self
            .entries
            .values()
            .map(|entry| (entry.inserted, entry.bytes))
            .collect();
        order.sort_unstable();

        let mut remaining_entries = self.entries.len();
        let mut remaining_bytes = self.total_bytes;
        let mut newest_evicted = None;
        for (inserted, entry_bytes) in order {
            if remaining_entries < entry_target
                && remaining_bytes.saturating_add(bytes) <= byte_target
            {
                break;
            }
            remaining_entries -= 1;
            remaining_bytes = remaining_bytes.saturating_sub(entry_bytes);
            newest_evicted = Some(inserted);
        }
        if let Some(newest_evicted) = newest_evicted {
            self.retain(|entry| entry.inserted > newest_evicted);
        }
    }
}

struct MemoStore {
    state: Mutex<State>,
    limits: Limits,
}

impl MemoStore {
    fn new(limits: Limits) -> Self {
        Self {
            state: Mutex::new(State::default()),
            limits,
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // Every update leaves the map and byte count consistent, so a panic
        // in another thread cannot leave a state that is unsafe to reuse.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, key: &str) -> Option<Arc<str>> {
        let mut state = self.state();
        let entry = state.entries.get(key)?;
        if Instant::now() < entry.expires_at {
            return Some(Arc::clone(&entry.value));
        }
        state.remove(key);
        None
    }

    fn set(&self, key: &str, value: &str) {
        let bytes = key.len().saturating_add(value.len());
        let expires_at = Instant::now().checked_add(self.limits.ttl);
        let mut state = self.state();
        // A replaced or skipped key never keeps its previous value.
        state.remove(key);
        let Some(expires_at) = expires_at else {
            return;
        };
        if bytes > self.limits.max_entry_bytes || bytes > self.limits.max_total_bytes {
            return;
        }
        if !state.fits(&self.limits, bytes) {
            let now = Instant::now();
            state.retain(|entry| now < entry.expires_at);
        }
        if !state.fits(&self.limits, bytes) {
            state.evict_oldest(&self.limits, bytes);
        }
        if !state.fits(&self.limits, bytes) {
            return;
        }
        let inserted = state.next_insertion;
        state.next_insertion = inserted.wrapping_add(1);
        state.total_bytes = state.total_bytes.saturating_add(bytes);
        state.entries.insert(
            key.into(),
            Entry {
                value: value.into(),
                expires_at,
                inserted,
                bytes,
            },
        );
    }

    #[cfg(test)]
    fn usage(&self) -> (usize, usize) {
        let state = self.state();
        (state.entries.len(), state.total_bytes)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn store(max_entries: usize, max_total_bytes: usize) -> MemoStore {
        MemoStore::new(Limits {
            max_entries,
            max_entry_bytes: 64,
            max_total_bytes,
            ttl: TTL,
        })
    }

    #[test]
    fn entry_count_is_bounded_and_oldest_entries_are_evicted_first() {
        let store = store(8, 1_024);
        for index in 0..100 {
            store.set(&format!("key-{index}"), "value");
            let (entries, _) = store.usage();
            assert!(entries <= 8, "{entries} entries after insert {index}");
        }
        // The newest insertion is always kept; the oldest were evicted.
        assert_eq!(store.get("key-99").as_deref(), Some("value"));
        assert_eq!(store.get("key-0"), None);
    }

    #[test]
    fn total_bytes_are_bounded() {
        let store = store(1_000, 100);
        for index in 0..50 {
            store.set(&format!("k{index:02}"), "0123456789");
            let (_, bytes) = store.usage();
            assert!(bytes <= 100, "{bytes} bytes after insert {index}");
        }
        assert_eq!(store.get("k49").as_deref(), Some("0123456789"));
    }

    #[test]
    fn oversized_entries_are_skipped_and_replace_stale_values() {
        let store = store(8, 1_024);
        store.set("key", "small");
        store.set("key", &"x".repeat(100));
        assert_eq!(store.get("key"), None);
        store.set(&"k".repeat(100), "v");
        assert_eq!(store.usage(), (0, 0));
    }

    #[test]
    fn replacing_a_key_keeps_the_byte_count_exact() {
        let store = store(8, 1_024);
        store.set("key", "first");
        store.set("key", "second value");
        assert_eq!(store.usage(), (1, "key".len() + "second value".len()));
        assert_eq!(store.get("key").as_deref(), Some("second value"));
    }

    #[test]
    fn expired_entries_are_not_returned_and_are_reclaimed_before_eviction() {
        let store = MemoStore::new(Limits {
            max_entries: 2,
            max_entry_bytes: 64,
            max_total_bytes: 1_024,
            ttl: Duration::ZERO,
        });
        store.set("a", "1");
        store.set("b", "2");
        assert_eq!(store.usage(), (2, 4));
        // Both expired entries are reclaimed instead of evicting by age.
        store.set("c", "3");
        assert_eq!(store.usage(), (1, 2));
        assert_eq!(store.get("c"), None);
        assert_eq!(store.usage(), (0, 0));
    }
}
