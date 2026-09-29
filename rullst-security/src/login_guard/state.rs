//! Bounded, time-ordered identity indexes used by [`super::LoginGuard`].
//!
//! Each index keeps a hash map for exact lookups and a B-tree ordered by one
//! instant (last failed attempt or jail expiry). Eviction and pruning therefore
//! touch only the oldest entries instead of scanning the whole map.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

/// SHA-256 digest of a trimmed identity; raw identities are never retained.
pub(super) type IdentityKey = [u8; 32];

pub(super) struct TimeOrderedIndex<V> {
    entries: HashMap<IdentityKey, (Instant, u64, V)>,
    order: BTreeMap<(Instant, u64), IdentityKey>,
    next_sequence: u64,
}

impl<V> TimeOrderedIndex<V> {
    pub(super) fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: BTreeMap::new(),
            next_sequence: 0,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn contains(&self, key: &IdentityKey) -> bool {
        self.entries.contains_key(key)
    }

    pub(super) fn get(&self, key: &IdentityKey) -> Option<(Instant, &V)> {
        self.entries.get(key).map(|(at, _, value)| (*at, value))
    }

    /// Inserts or replaces an entry and moves it to its new time position.
    pub(super) fn upsert(&mut self, key: IdentityKey, at: Instant, value: V) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if let Some((old_at, old_sequence, _)) = self.entries.insert(key, (at, sequence, value)) {
            self.order.remove(&(old_at, old_sequence));
        }
        self.order.insert((at, sequence), key);
    }

    pub(super) fn remove(&mut self, key: &IdentityKey) -> Option<V> {
        let (at, sequence, value) = self.entries.remove(key)?;
        self.order.remove(&(at, sequence));
        Some(value)
    }

    /// Removes the entry with the earliest instant.
    pub(super) fn pop_earliest(&mut self) -> Option<IdentityKey> {
        let (_, key) = self.order.pop_first()?;
        self.entries.remove(&key);
        Some(key)
    }

    /// Removes entries from the earliest end while `expired` holds.
    pub(super) fn prune_while(&mut self, mut expired: impl FnMut(Instant) -> bool) {
        while let Some((&(at, _), _)) = self.order.first_key_value() {
            if !expired(at) {
                break;
            }
            self.pop_earliest();
        }
    }

    /// Evicts earliest entries until one more entry fits within `capacity`.
    pub(super) fn make_room(&mut self, capacity: usize) {
        while self.len() >= capacity && self.pop_earliest().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn upsert_moves_the_entry_and_eviction_follows_time_order() {
        let base = Instant::now();
        let mut index = TimeOrderedIndex::new();
        index.upsert([1; 32], base + Duration::from_secs(2), 1_u32);
        index.upsert([2; 32], base + Duration::from_secs(1), 2);
        index.upsert([1; 32], base + Duration::from_secs(3), 3);
        assert_eq!(index.len(), 2);
        assert_eq!(index.order.len(), 2);

        index.make_room(2);
        assert!(!index.contains(&[2; 32]));
        assert_eq!(index.get(&[1; 32]).map(|(_, value)| *value), Some(3));

        index.prune_while(|at| at <= base + Duration::from_secs(3));
        assert_eq!(index.len(), 0);
        assert!(index.order.is_empty());
        assert_eq!(index.remove(&[1; 32]), None);
    }
}
