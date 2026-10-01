//! Native Real-Time Engine (Channels, Broadcast, Presence).

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

use crate::security::{TenantContext, tenant_namespaced_name};

const MAX_CHANNEL_BYTES: usize = 128;
const MAX_EVENT_BYTES: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
/// Registry size that triggers the first sweep of idle channels.
const MIN_CHANNEL_SWEEP_THRESHOLD: usize = 64;

/// Payload model for realtime broadcast events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeMessage {
    /// Target channel name.
    pub channel: String,
    /// Event name or topic.
    pub event: String,
    /// Stringified JSON or HTML payload.
    pub payload: String,
}

/// Represents a named realtime channel with broadcast capabilities.
pub struct Channel {
    /// Channel identifier name.
    pub name: String,
    sender: broadcast::Sender<RealtimeMessage>,
}

/// Strongly-typed error domain for Rullst Realtime and Broadcast operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RealtimeError {
    /// Broadcast transmission failed.
    #[error("Broadcast send error: {0}")]
    BroadcastError(String),
    /// A tenant-scoped channel name is empty, oversized, or ambiguous.
    #[error("invalid realtime channel: {0}")]
    InvalidChannel(String),
    /// An event name is empty, oversized, or ambiguous.
    #[error("invalid realtime event: {0}")]
    InvalidEvent(String),
    /// A payload exceeds the bounded in-process realtime envelope.
    #[error("realtime payload is too large: {actual} bytes exceeds {maximum}")]
    PayloadTooLarge {
        /// Observed UTF-8 payload length in bytes.
        actual: usize,
        /// Maximum accepted UTF-8 payload length in bytes.
        maximum: usize,
    },
    /// A presence identity is empty, oversized, or ambiguous.
    #[error("invalid realtime presence identity: {0}")]
    InvalidPresenceIdentity(String),
}

impl Channel {
    /// Creates a new realtime channel with specified message queue capacity.
    pub fn new(name: impl Into<String>, capacity: usize) -> Self {
        // Tokio rejects zero-capacity broadcast channels. Keep this infallible
        // compatibility constructor panic-free while preserving a bounded queue.
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self {
            name: name.into(),
            sender,
        }
    }

    /// Broadcasts an event and payload to all subscribed clients.
    pub fn broadcast(&self, event: &str, payload: &str) -> Result<usize, RealtimeError> {
        let msg = RealtimeMessage {
            channel: self.name.clone(),
            event: event.to_string(),
            payload: payload.to_string(),
        };
        self.sender
            .send(msg)
            .map_err(|e| RealtimeError::BroadcastError(e.to_string()))
    }

    /// Subscribes to messages broadcasted on this channel.
    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeMessage> {
        self.sender.subscribe()
    }
}

/// In-process realtime facade permanently bound to one authenticated tenant.
///
/// Logical channel names are mapped to an immutable tenant namespace before a
/// channel is opened or published. Authentication and room-level authorization
/// remain application responsibilities; this wrapper prevents accidental
/// cross-tenant reuse of the same logical room name.
#[derive(Clone)]
#[non_exhaustive]
pub struct TenantRealtime {
    manager: Arc<BroadcastManager>,
    tenant_id: String,
}

impl TenantRealtime {
    /// Binds a shared broadcast manager to an authenticated tenant context.
    pub fn from_context(manager: Arc<BroadcastManager>, context: &TenantContext) -> Self {
        Self {
            manager,
            tenant_id: context.tenant_id.clone(),
        }
    }

    /// Returns the authenticated tenant identifier bound to this instance.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Returns the canonical backend channel inside this tenant namespace.
    ///
    /// The channel is `tenants:<tenant>:<logical_channel>`. In the tenant
    /// segment `%` is written as `%25` and `:` as `%3A`, so a logical channel
    /// containing `:` can never alias another tenant's channel.
    pub fn namespaced_channel(&self, logical_channel: &str) -> Result<String, RealtimeError> {
        validate_room(logical_channel)?;
        Ok(tenant_namespaced_name(&self.tenant_id, logical_channel))
    }

    /// Subscribes only to the tenant-scoped version of a logical channel.
    pub fn subscribe(
        &self,
        logical_channel: &str,
    ) -> Result<broadcast::Receiver<RealtimeMessage>, RealtimeError> {
        let channel = self.namespaced_channel(logical_channel)?;
        Ok(self.manager.get_or_create(&channel).subscribe())
    }

    /// Publishes a bounded event only to this tenant's logical channel.
    pub fn publish(
        &self,
        logical_channel: &str,
        event: &str,
        payload: &str,
    ) -> Result<usize, RealtimeError> {
        let channel = self.namespaced_channel(logical_channel)?;
        validate_name(event, MAX_EVENT_BYTES, RealtimeError::InvalidEvent)?;
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RealtimeError::PayloadTooLarge {
                actual: payload.len(),
                maximum: MAX_PAYLOAD_BYTES,
            });
        }
        self.manager.publish(&channel, event, payload)
    }
}

fn validate_name(
    value: &str,
    maximum: usize,
    error: impl FnOnce(String) -> RealtimeError,
) -> Result<(), RealtimeError> {
    if value.is_empty()
        || value.len() > maximum
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
    {
        return Err(error(value.to_string()));
    }
    Ok(())
}

fn validate_room(room: &str) -> Result<(), RealtimeError> {
    validate_name(room, MAX_CHANNEL_BYTES, RealtimeError::InvalidChannel)?;
    if room
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(RealtimeError::InvalidChannel(room.to_string()));
    }
    Ok(())
}

/// Thread-safe in-memory pub/sub manager for realtime channels.
///
/// A channel is retained only while it has a subscriber or a caller holds the
/// `Arc` from [`Self::get_or_create`]. Publishing never creates a channel, and
/// idle channels are released on a failed publish or by an amortized sweep.
#[derive(Default)]
pub struct BroadcastManager {
    channels: DashMap<String, Arc<Channel>>,
    sweep_threshold: AtomicUsize,
}

impl BroadcastManager {
    /// Creates a new BroadcastManager instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves an existing channel or creates a new one if it does not exist.
    /// A channel with no subscriber and no outstanding `Arc` may be released.
    pub fn get_or_create(&self, channel_name: &str) -> Arc<Channel> {
        let mut created = false;
        let channel = self
            .channels
            .entry(channel_name.to_string())
            .or_insert_with(|| {
                created = true;
                Arc::new(Channel::new(channel_name, 100))
            })
            .value()
            .clone();
        if created {
            self.sweep_idle_channels_if_due();
        }
        channel
    }

    /// Publishes a message directly to a channel by name. Without subscribers it
    /// returns [`RealtimeError::BroadcastError`] and retains no channel.
    pub fn publish(
        &self,
        channel_name: &str,
        event: &str,
        payload: &str,
    ) -> Result<usize, RealtimeError> {
        let Some(channel) = self
            .channels
            .get(channel_name)
            .map(|entry| Arc::clone(entry.value()))
        else {
            return Err(RealtimeError::BroadcastError(
                broadcast::error::SendError(()).to_string(),
            ));
        };
        let result = channel.broadcast(event, payload);
        if result.is_err() {
            drop(channel);
            self.channels
                .remove_if(channel_name, |_, channel| is_idle_channel(channel));
        }
        result
    }

    /// Sweeps once the registry has doubled since the previous sweep, keeping
    /// the amortized cost per created channel constant.
    fn sweep_idle_channels_if_due(&self) {
        let threshold = self.sweep_threshold.load(Ordering::Relaxed);
        if self.channels.len() >= threshold.max(MIN_CHANNEL_SWEEP_THRESHOLD) {
            self.remove_idle_channels();
            let next = self.channels.len().saturating_mul(2);
            self.sweep_threshold.store(next, Ordering::Relaxed);
        }
    }

    fn remove_idle_channels(&self) {
        self.channels.retain(|_, channel| !is_idle_channel(channel));
    }
}

/// Checked under the shard write lock: if the registry holds the only `Arc`,
/// nobody can clone it or subscribe through it until the lock is released, so
/// a concurrent subscriber is never detached from the channel it joined.
fn is_idle_channel(channel: &Arc<Channel>) -> bool {
    Arc::strong_count(channel) == 1 && channel.sender.receiver_count() == 0
}

/// In-memory tracker for active user presence across channels/rooms.
///
/// Presence is counted per connection: call [`Self::user_left`] once for
/// every [`Self::user_joined`]. A user with several connections in a room (for
/// example two browser tabs) stays online until the last one leaves, and a
/// room is removed when its last user leaves.
#[derive(Default)]
pub struct PresenceTracker {
    /// Open connections of each online user, per room.
    online_users: DashMap<String, DashMap<String, usize>>,
}

impl PresenceTracker {
    /// Creates a new PresenceTracker instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one more connection of a user in a specific room.
    pub fn user_joined(&self, room: &str, user_id: &str) {
        let room_map = self.online_users.entry(room.to_string()).or_default();
        let mut connections = room_map.entry(user_id.to_string()).or_insert(0);
        *connections = connections.saturating_add(1);
    }

    /// Removes one of a user's connections from a room upon disconnect. The
    /// user goes offline when their last connection leaves, and the room is
    /// removed once it is empty. A leave without a matching join is ignored.
    pub fn user_left(&self, room: &str, user_id: &str) {
        let now_empty = self.online_users.get(room).is_some_and(|room_map| {
            if let dashmap::Entry::Occupied(mut connections) = room_map.entry(user_id.to_string()) {
                if *connections.get() <= 1 {
                    connections.remove();
                } else {
                    *connections.get_mut() -= 1;
                }
            }
            room_map.is_empty()
        });
        if now_empty {
            // `user_joined` inserts while holding this shard's write lock, so the
            // re-check cannot discard a user who joined in the meantime.
            self.online_users
                .remove_if(room, |_, room_map| room_map.is_empty());
        }
    }

    /// Returns the count of currently online users in a room.
    pub fn count_online(&self, room: &str) -> usize {
        self.online_users.get(room).map(|m| m.len()).unwrap_or(0)
    }
}

/// In-process presence facade permanently bound to one authenticated tenant.
#[derive(Clone)]
#[non_exhaustive]
pub struct TenantPresence {
    tracker: Arc<PresenceTracker>,
    tenant_id: String,
}

impl TenantPresence {
    /// Binds a shared presence tracker to an authenticated tenant context.
    pub fn from_context(tracker: Arc<PresenceTracker>, context: &TenantContext) -> Self {
        Self {
            tracker,
            tenant_id: context.tenant_id.clone(),
        }
    }

    /// Returns the authenticated tenant identifier bound to this instance.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Registers one connection of a bounded identity only in this tenant's
    /// logical room; see [`PresenceTracker::user_joined`].
    pub fn user_joined(&self, room: &str, user_id: &str) -> Result<(), RealtimeError> {
        let room = self.namespaced_room(room)?;
        validate_name(
            user_id,
            MAX_CHANNEL_BYTES,
            RealtimeError::InvalidPresenceIdentity,
        )?;
        self.tracker.user_joined(&room, user_id);
        Ok(())
    }

    /// Removes one connection of a bounded identity only from this tenant's
    /// logical room; see [`PresenceTracker::user_left`].
    pub fn user_left(&self, room: &str, user_id: &str) -> Result<(), RealtimeError> {
        let room = self.namespaced_room(room)?;
        validate_name(
            user_id,
            MAX_CHANNEL_BYTES,
            RealtimeError::InvalidPresenceIdentity,
        )?;
        self.tracker.user_left(&room, user_id);
        Ok(())
    }

    /// Returns the online count only for this tenant's logical room.
    pub fn count_online(&self, room: &str) -> Result<usize, RealtimeError> {
        Ok(self.tracker.count_online(&self.namespaced_room(room)?))
    }

    fn namespaced_room(&self, room: &str) -> Result<String, RealtimeError> {
        validate_room(room)?;
        Ok(tenant_namespaced_name(&self.tenant_id, room))
    }
}

#[cfg(test)]
#[path = "realtime_retention_tests.rs"]
mod retention_tests;

#[cfg(test)]
#[path = "realtime_tests.rs"]
mod tests;
