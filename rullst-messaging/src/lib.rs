//! Bounded brokered-messaging contracts for Rullst applications.
//!
//! This crate is deliberately separate from [`rullst-connect`](https://docs.rs/rullst-connect):
//! identity federation and event brokers have different security, availability, and delivery
//! semantics. The crate provides deterministic memory, optional durable SQLite and an opt-in
//! standalone Redis Streams profile with Rullst-owned fenced delivery indexes. Other remote
//! brokers remain roadmap work; no cross-system exactly-once effects are claimed.
//!
//! [`MessageBroker`] is the broker contract and [`InMemoryBroker`] its deterministic,
//! process-local implementation. Delivery is at least once: make external effects idempotent
//! with the envelope ID.
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
//!
//! No Cargo feature is enabled by default. `sqlite` adds `SqliteBroker` (durable local state
//! with optional AES-256-GCM content encryption), `redis-streams` adds `RedisBroker`,
//! `orm-outbox` adds `OrmOutboxRelay` for the `rullst-orm` outbox, `schedules-postgres` adds the
//! `schedules` module (recurring publications) and `webhooks` adds the `webhooks` module
//! (outgoing webhooks; it enables `sqlite`). In a Rullst application, enable the `messaging`
//! feature of `rullst`.
//!
//! Guide: <https://rullst.github.io/Rullst/book/crates/messaging.html>.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod admin;
mod clock;
mod error;
mod memory;
mod model;
#[cfg(feature = "orm-outbox")]
mod outbox_relay;
#[cfg(feature = "redis-streams")]
mod redis_streams;
#[cfg(feature = "schedules-postgres")]
pub mod schedules;
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(any(feature = "sqlite", feature = "schedules-postgres"))]
mod storage_keys;
mod trace;
mod traits;
mod types;
mod validation;
#[cfg(feature = "webhooks")]
pub mod webhooks;
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
pub use sqlite::SqliteBroker;
#[cfg(any(feature = "sqlite", feature = "schedules-postgres"))]
pub use storage_keys::{MessagingKeyring, MessagingStorageKey};
pub use trace::TraceContext;
pub use traits::{MessageAdmin, MessageBroker};
pub use types::{
    AckToken, BrokerConfig, ConsumerGroup, ConsumerName, ContentType, EventKind, FailureCode,
    IdempotencyKey, MessageHeaders, MessageId, Namespace, StartPosition, TopicName,
};
pub use wire::WireEnvelopeCodec;
