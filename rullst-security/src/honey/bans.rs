//! Bounded honeypot ban list with expiry-ordered eviction.
//!
//! Lookups touch only the requested peer. Expired bans are pruned from the
//! front of an expiry-ordered index when a ban is inserted or counted, so no
//! operation scans every retained ban.

use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;
use std::time::Instant;

#[derive(Debug, Default)]
pub(super) struct BanList {
    expires: HashMap<IpAddr, Instant>,
    /// `(expiry, peer)` pairs; the first entry expires soonest.
    order: BTreeSet<(Instant, IpAddr)>,
}

impl BanList {
    /// Returns whether `ip` has an active ban, removing only its own expired entry.
    pub(super) fn is_banned(&mut self, ip: IpAddr, now: Instant) -> bool {
        let Some(&expires_at) = self.expires.get(&ip) else {
            return false;
        };
        if expires_at > now {
            return true;
        }
        self.expires.remove(&ip);
        self.order.remove(&(expires_at, ip));
        false
    }

    /// Inserts or refreshes a ban, evicting the soonest-expiring bans at capacity.
    pub(super) fn insert(&mut self, ip: IpAddr, expires_at: Instant, capacity: usize) {
        if let Some(previous) = self.expires.insert(ip, expires_at) {
            self.order.remove(&(previous, ip));
        } else {
            while self.expires.len() > capacity {
                let Some((_, evicted)) = self.order.pop_first() else {
                    break;
                };
                self.expires.remove(&evicted);
            }
        }
        self.order.insert((expires_at, ip));
    }

    /// Removes every expired ban, starting with the soonest expiry.
    pub(super) fn prune_expired(&mut self, now: Instant) {
        while let Some(&(expires_at, ip)) = self.order.first() {
            if expires_at > now {
                break;
            }
            self.order.pop_first();
            self.expires.remove(&ip);
        }
    }

    pub(super) fn len(&self) -> usize {
        self.expires.len()
    }
}
