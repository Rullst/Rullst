//! Bounded bucket retention and peer keying for [`RateLimiter`].

use super::{RateLimiter, refill_token_count};
use std::net::IpAddr;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Maximum number of keys a limiter tracks (roughly 10 MB of buckets).
pub(super) const MAX_RATE_LIMIT_BUCKETS: usize = 100_000;
/// New keys between opportunistic sweeps of refilled buckets.
const REFILLED_SWEEP_INTERVAL: usize = 4_096;

impl RateLimiter {
    /// Keeps the bucket map bounded before a new key is inserted.
    ///
    /// Every [`REFILLED_SWEEP_INTERVAL`] new keys, and whenever the map is full,
    /// refilled buckets are dropped. A full map is then trimmed to seven eighths
    /// of its capacity (at least one slot) by evicting the least recently used
    /// buckets, so the scan cost is amortized over many insertions.
    pub(super) fn make_room_for_new_key(&self, now: Instant) {
        let inserted = self
            .new_keys
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        let at_capacity = self.buckets.len() >= self.max_buckets;
        if !at_capacity && !inserted.is_multiple_of(REFILLED_SWEEP_INTERVAL) {
            return;
        }
        self.evict_refilled_buckets(now);
        if at_capacity {
            let headroom = (self.max_buckets / 8).max(1);
            self.evict_least_recently_used(self.max_buckets.saturating_sub(headroom));
        }
    }

    /// A refilled bucket is indistinguishable from a new one, so dropping it
    /// never changes a later decision.
    fn evict_refilled_buckets(&self, now: Instant) {
        self.buckets.retain(|_, bucket| {
            let elapsed = now.saturating_duration_since(bucket.last_refill);
            refill_token_count(bucket.tokens, elapsed.as_secs_f64(), &self.config)
                < self.config.max_tokens
        });
    }

    /// Evicts the least recently used buckets until at most `target` remain.
    /// An evicted client starts again with a full burst.
    fn evict_least_recently_used(&self, target: usize) {
        let mut last_used: Vec<Instant> = self
            .buckets
            .iter()
            .map(|entry| entry.value().last_refill)
            .collect();
        let Some(excess) = last_used
            .len()
            .checked_sub(target)
            .filter(|excess| *excess > 0)
        else {
            return;
        };
        // `excess - 1 < last_used.len()`, so selection cannot go out of bounds.
        let (_, cutoff, _) = last_used.select_nth_unstable(excess - 1);
        let cutoff = *cutoff;
        self.buckets.retain(|_, bucket| bucket.last_refill > cutoff);
    }

    #[cfg(test)]
    pub(super) fn with_bucket_limit(mut self, max_buckets: usize) -> Self {
        self.max_buckets = max_buckets.max(1);
        self
    }
}

/// Rate-limit key for a transport peer.
///
/// IPv4 peers are keyed per address. IPv6 peers are keyed per /64, the
/// smallest prefix normally delegated to one subscriber, so rotating source
/// addresses inside it does not yield fresh buckets. IPv4-mapped IPv6
/// addresses are keyed as IPv4.
pub(super) fn peer_rate_limit_key(ip: IpAddr) -> String {
    match ip.to_canonical() {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => {
            let [a, b, c, d, ..] = address.segments();
            format!("{a:x}:{b:x}:{c:x}:{d:x}::/64")
        }
    }
}
