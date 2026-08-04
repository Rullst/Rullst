//! Native Real-Time Engine (Channels, Broadcast, Presence).

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

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

impl Channel {
    /// Creates a new realtime channel with specified message queue capacity.
    pub fn new(name: impl Into<String>, capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            name: name.into(),
            sender,
        }
    }

    /// Broadcasts an event and payload to all subscribed clients.
    pub fn broadcast(&self, event: &str, payload: &str) -> Result<usize, String> {
        let msg = RealtimeMessage {
            channel: self.name.clone(),
            event: event.to_string(),
            payload: payload.to_string(),
        };
        self.sender
            .send(msg)
            .map_err(|e| format!("Broadcast send error: {}", e))
    }

    /// Subscribes to messages broadcasted on this channel.
    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeMessage> {
        self.sender.subscribe()
    }
}

/// Thread-safe in-memory pub/sub manager for realtime channels.
#[derive(Default)]
pub struct BroadcastManager {
    channels: DashMap<String, Arc<Channel>>,
}

impl BroadcastManager {
    /// Creates a new BroadcastManager instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves an existing channel or creates a new one if it does not exist.
    pub fn get_or_create(&self, channel_name: &str) -> Arc<Channel> {
        self.channels
            .entry(channel_name.to_string())
            .or_insert_with(|| Arc::new(Channel::new(channel_name, 100)))
            .value()
            .clone()
    }

    /// Publishes a message directly to a channel by name.
    pub fn publish(&self, channel_name: &str, event: &str, payload: &str) -> Result<usize, String> {
        let ch = self.get_or_create(channel_name);
        ch.broadcast(event, payload)
    }
}

/// In-memory tracker for active user presence across channels/rooms.
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

    /// Removes a user from a specific room upon disconnect.
    pub fn user_left(&self, room: &str, user_id: &str) {
        if let Some(room_map) = self.online_users.get(room) {
            room_map.remove(user_id);
        }
    }

    /// Returns the count of currently online users in a room.
    pub fn count_online(&self, room: &str) -> usize {
        self.online_users.get(room).map(|m| m.len()).unwrap_or(0)
    }
}
