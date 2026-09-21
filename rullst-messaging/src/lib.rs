//! Bounded brokered-messaging contracts for Rullst applications.
//!
//! This crate is deliberately separate from [`rullst-connect`](https://docs.rs/rullst-connect):
//! identity federation and event brokers have different security, availability, and delivery
//! semantics. The crate provides deterministic memory, optional durable SQLite and an opt-in
//! standalone Redis Streams profile with Rullst-owned fenced delivery indexes. Other remote
//! brokers remain roadmap work; no cross-system exactly-once effects are claimed.

mod admin;
mod clock;
mod error;
mod memory;
mod model;
#[cfg(feature = "orm-outbox")]
mod outbox_relay;
#[cfg(feature = "redis-streams")]
mod redis_streams;
#[cfg(feature = "sqlite")]
mod sqlite;
mod trace;
mod traits;
mod types;
mod validation;
mod wire;

pub use admin::{DeadLetter, DeadLetterQuery, PurgeReceipt, PurgeRequest};
pub use clock::{Clock, SystemClock};
pub use error::{MessagingError, Result};
pub use memory::InMemoryBroker;
pub use model::{
    Delivery, MessageEnvelope, PublishReceipt, PublishRequest, ReceiveRequest, RetryDisposition,
    SubscriptionReceipt, SubscriptionRequest,
};
#[cfg(feature = "orm-outbox")]
pub use outbox_relay::{OrmOutboxRelay, OrmOutboxRelayError, OutboxRelayReceipt};
#[cfg(feature = "redis-streams")]
pub use redis_streams::{RedisBroker, RedisBrokerConfig};
#[cfg(feature = "sqlite")]
pub use sqlite::{MessagingKeyring, MessagingStorageKey, SqliteBroker};
pub use trace::TraceContext;
pub use traits::{MessageAdmin, MessageBroker};
pub use types::{
    AckToken, BrokerConfig, ConsumerGroup, ConsumerName, ContentType, EventKind, FailureCode,
    IdempotencyKey, MessageHeaders, MessageId, Namespace, StartPosition, TopicName,
};
pub use wire::WireEnvelopeCodec;
