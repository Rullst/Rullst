//! Bounded brokered-messaging contracts for Rullst applications.
//!
//! This crate is deliberately separate from [`rullst-connect`](https://docs.rs/rullst-connect):
//! identity federation and event brokers have different security, availability, and delivery
//! semantics. The initial implementation provides a deterministic in-memory broker and a shared
//! conformance boundary for future remote adapters. It does not claim Kafka, RabbitMQ, NATS,
//! Redis Streams, SQS/SNS, Pub/Sub, or Pulsar interoperability yet.

mod admin;
mod clock;
mod error;
mod memory;
mod model;
mod traits;
mod types;
mod validation;

pub use admin::{DeadLetter, DeadLetterQuery, PurgeReceipt, PurgeRequest};
pub use clock::{Clock, SystemClock};
pub use error::{MessagingError, Result};
pub use memory::InMemoryBroker;
pub use model::{
    Delivery, MessageEnvelope, PublishReceipt, PublishRequest, ReceiveRequest, RetryDisposition,
    SubscriptionReceipt, SubscriptionRequest,
};
pub use traits::{MessageAdmin, MessageBroker};
pub use types::{
    AckToken, BrokerConfig, ConsumerGroup, ConsumerName, ContentType, EventKind, FailureCode,
    IdempotencyKey, MessageHeaders, MessageId, Namespace, StartPosition, TopicName,
};
