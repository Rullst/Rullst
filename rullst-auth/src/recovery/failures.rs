//! Bounded, process-local budgets for failed credential lookups.

use super::RecoveryError;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Per-key attempt budget shared by clones of one store. A unit is taken
/// before the lookup and kept only when the lookup fails, so attempts still
/// in flight count too. Memory is bounded by `capacity`; beyond it, expired
/// windows and then the oldest window are evicted.
#[derive(Clone)]
pub(super) struct FailureBudget {
    limit: u32,
    window: i64,
    capacity: usize,
    entries: Arc<Mutex<HashMap<String, (i64, u32)>>>,
}

impl FailureBudget {
    /// Password reset: 10 attempts per reset-token digest per 60 seconds,
    /// tracking at most 10,000 digests.
    pub(super) fn password_reset() -> Self {
        Self::new(10, 60, 10_000)
    }

    fn new(limit: u32, window: i64, capacity: usize) -> Self {
        Self {
            limit,
            window,
            capacity,
            entries: Arc::default(),
        }
    }

    /// Takes one unit for `key`; `false` means its window is already spent.
    pub(super) fn try_take(&self, key: &str, now: i64) -> Result<bool, RecoveryError> {
        let window = self.window;
        let active = |start: i64| (start..start.saturating_add(window)).contains(&now);
        let mut entries = self.entries.lock().map_err(|_| RecoveryError::Storage)?;
        if let Some((start, used)) = entries.get_mut(key) {
            if !active(*start) {
                (*start, *used) = (now, 0);
            }
            if *used >= self.limit {
                return Ok(false);
            }
            *used += 1;
            return Ok(true);
        }
        if entries.len() >= self.capacity {
            entries.retain(|_, (start, _)| active(*start));
            if entries.len() >= self.capacity
                && let Some(oldest) = entries
                    .iter()
                    .min_by_key(|(_, (start, _))| *start)
                    .map(|(key, _)| key.clone())
            {
                entries.remove(&oldest);
            }
        }
        entries.insert(key.to_owned(), (now, 1));
        Ok(true)
    }

    /// Returns the unit of an attempt whose lookup did not fail.
    pub(super) fn refund(&self, key: &str) -> Result<(), RecoveryError> {
        let mut entries = self.entries.lock().map_err(|_| RecoveryError::Storage)?;
        if let Some((_, used)) = entries.get_mut(key) {
            *used = used.saturating_sub(1);
            if *used == 0 {
                entries.remove(key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_per_key_windowed_refundable_and_bounded() {
        let budget = FailureBudget::new(3, 60, 4);
        for _ in 0..3 {
            assert!(budget.try_take("replayed", 1_000).unwrap());
        }
        assert!(!budget.try_take("replayed", 1_059).unwrap());
        assert!(budget.try_take("other", 1_000).unwrap());
        // A refunded (successful) attempt frees its unit and, at zero, its entry.
        budget.refund("other").unwrap();
        assert!(!budget.entries.lock().unwrap().contains_key("other"));
        // The window ends, and a clock regression starts a fresh window.
        assert!(budget.try_take("replayed", 1_060).unwrap());
        assert!(budget.try_take("replayed", 999).unwrap());

        for index in 0..10 {
            assert!(budget.try_take(&format!("rotated-{index}"), 2_000).unwrap());
        }
        let entries = budget.entries.lock().unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries.contains_key("rotated-9"));
    }
}
