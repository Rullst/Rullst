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
/// A room is removed when its last user leaves.
#[derive(Default)]
pub struct PresenceTracker {
    online_users: DashMap<String, DashMap<String, u64>>,
}

impl PresenceTracker {
    /// Creates a new PresenceTracker instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a user as online in a specific room.
    pub fn user_joined(&self, room: &str, user_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let room_map = self.online_users.entry(room.to_string()).or_default();
        room_map.insert(user_id.to_string(), now);
    }

    /// Removes a user from a specific room upon disconnect, and the room once
    /// it is empty.
    pub fn user_left(&self, room: &str, user_id: &str) {
        let now_empty = self.online_users.get(room).is_some_and(|room_map| {
            room_map.remove(user_id);
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

    /// Registers a bounded identity only in this tenant's logical room.
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

    /// Removes a bounded identity only from this tenant's logical room.
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::security::TenantMembership;

    #[tokio::test]
    // TM-TENANT-04
    async fn identical_channels_are_isolated_by_authenticated_tenant_context() {
        let membership = TenantMembership::try_new(["school-alpha", "school-beta"])
            .expect("valid tenant membership");
        let alpha_context = membership.select("school-alpha").expect("alpha membership");
        let beta_context = membership.select("school-beta").expect("beta membership");
        let manager = Arc::new(BroadcastManager::new());
        let alpha = TenantRealtime::from_context(Arc::clone(&manager), &alpha_context);
        let beta = TenantRealtime::from_context(manager, &beta_context);
        let mut alpha_receiver = alpha.subscribe("course/1").expect("alpha subscription");
        let mut beta_receiver = beta.subscribe("course/1").expect("beta subscription");

        alpha
            .publish("course/1", "lesson.completed", r#"{"lesson_id":7}"#)
            .expect("alpha publish");
        let alpha_message = alpha_receiver.recv().await.expect("alpha message");

        assert_eq!(alpha_message.channel, "tenants:school-alpha:course/1");
        assert_eq!(alpha_message.event, "lesson.completed");
        assert!(matches!(
            beta_receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            beta.publish("../school-alpha/course/1", "lesson.completed", "{}"),
            Err(RealtimeError::InvalidChannel(_))
        ));
        assert!(matches!(
            beta.publish("course/1", "lesson completed", "{}"),
            Err(RealtimeError::InvalidEvent(_))
        ));
        assert!(matches!(
            beta.publish("course/1", "lesson.completed", &"x".repeat(65_537)),
            Err(RealtimeError::PayloadTooLarge { .. })
        ));

        let presence = Arc::new(PresenceTracker::new());
        let alpha_presence = TenantPresence::from_context(Arc::clone(&presence), &alpha_context);
        let beta_presence = TenantPresence::from_context(presence, &beta_context);
        alpha_presence
            .user_joined("course/1", "learner-7")
            .expect("alpha presence");
        assert_eq!(
            alpha_presence
                .count_online("course/1")
                .expect("alpha presence count"),
            1
        );
        assert_eq!(
            beta_presence
                .count_online("course/1")
                .expect("beta presence count"),
            0
        );
        assert!(matches!(
            beta_presence.user_joined("course/1", "learner 7"),
            Err(RealtimeError::InvalidPresenceIdentity(_))
        ));
    }

    #[tokio::test]
    // TM-TENANT-04
    async fn colon_in_tenant_or_channel_cannot_alias_another_namespace() {
        let membership =
            TenantMembership::try_new(["a", "a:b", "acme"]).expect("valid tenant membership");
        let a_context = membership.select("a").expect("a membership");
        let a_b_context = membership.select("a:b").expect("a:b membership");
        let acme_context = membership.select("acme").expect("acme membership");
        let manager = Arc::new(BroadcastManager::new());
        let a = TenantRealtime::from_context(Arc::clone(&manager), &a_context);
        let a_b = TenantRealtime::from_context(Arc::clone(&manager), &a_b_context);
        let acme = TenantRealtime::from_context(manager, &acme_context);

        assert_eq!(a.namespaced_channel("b:c").expect("a"), "tenants:a:b:c");
        assert_eq!(a_b.namespaced_channel("c").expect("a:b"), "tenants:a%3Ab:c");
        assert_eq!(
            acme.namespaced_channel("course:1").expect("acme"),
            "tenants:acme:course:1"
        );

        let mut a_receiver = a.subscribe("b:c").expect("a subscription");
        let mut a_b_receiver = a_b.subscribe("c").expect("a:b subscription");
        assert_eq!(
            a_b.publish("c", "lesson.completed", "{}")
                .expect("a:b publish"),
            1
        );
        assert_eq!(
            a_b_receiver.recv().await.expect("a:b message").channel,
            "tenants:a%3Ab:c"
        );
        assert!(matches!(
            a_receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));

        let presence = Arc::new(PresenceTracker::new());
        let a_presence = TenantPresence::from_context(Arc::clone(&presence), &a_context);
        let a_b_presence = TenantPresence::from_context(presence, &a_b_context);
        assert_eq!(
            a_b_presence.namespaced_room("c").expect("a:b room"),
            "tenants:a%3Ab:c"
        );
        a_presence
            .user_joined("b:c", "learner-7")
            .expect("a presence");
        assert_eq!(a_presence.count_online("b:c").expect("a count"), 1);
        assert_eq!(a_b_presence.count_online("c").expect("a:b count"), 0);
    }

    #[test]
    fn zero_capacity_channel_is_panic_free() {
        let channel = Channel::new("bounded", 0);
        let _receiver = channel.subscribe();
        assert_eq!(channel.name, "bounded");
    }
}
