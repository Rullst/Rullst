//! Deterministic bounded broker used for tests, local development, and contract proofs.

mod helpers;
mod operations;
mod state;

use crate::{
    AckToken, BrokerConfig, Clock, DeadLetter, DeadLetterQuery, Delivery, FailureCode,
    MessageAdmin, MessageBroker, PublishReceipt, PublishRequest, PurgeReceipt, PurgeRequest,
    ReceiveRequest, Result, RetryDisposition, SubscriptionReceipt, SubscriptionRequest,
    SystemClock,
};
use state::State;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Process-local bounded broker with fan-out consumer groups and at-least-once leases.
///
/// This is a deterministic offline implementation, not a distributed broker. State disappears
/// with the process and a hard capacity failure requires explicit terminal-message purging.
#[derive(Clone)]
pub struct InMemoryBroker<C = SystemClock> {
    pub(in crate::memory) config: BrokerConfig,
    pub(in crate::memory) clock: C,
    pub(in crate::memory) state: Arc<Mutex<State>>,
}

impl InMemoryBroker<SystemClock> {
    /// Creates a broker using the system clock.
    pub fn new(config: BrokerConfig) -> Self {
        Self::with_clock(config, SystemClock)
    }
}

impl<C: Clock> InMemoryBroker<C> {
    /// Creates a broker with an injectable trusted clock.
    pub fn with_clock(config: BrokerConfig, clock: C) -> Self {
        Self {
            config,
            clock,
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    /// Returns the immutable broker configuration.
    pub fn config(&self) -> &BrokerConfig {
        &self.config
    }
}

impl<C: Clock> MessageBroker for InMemoryBroker<C> {
    async fn publish(&self, request: PublishRequest) -> Result<PublishReceipt> {
        self.publish_inner(request).await
    }

    async fn subscribe(&self, request: SubscriptionRequest) -> Result<SubscriptionReceipt> {
        self.subscribe_inner(request).await
    }

    async fn receive(&self, request: ReceiveRequest) -> Result<Vec<Delivery>> {
        self.receive_inner(request).await
    }

    async fn ack(&self, token: &AckToken) -> Result<()> {
        self.ack_inner(token).await
    }

    async fn retry(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> Result<RetryDisposition> {
        self.retry_inner(token, delay, failure_code).await
    }

    async fn dead_letter(&self, token: &AckToken, failure_code: FailureCode) -> Result<()> {
        self.dead_letter_inner(token, failure_code).await
    }
}

impl<C: Clock> MessageAdmin for InMemoryBroker<C> {
    async fn dead_letters(&self, query: DeadLetterQuery) -> Result<Vec<DeadLetter>> {
        self.dead_letters_inner(query).await
    }

    async fn purge_terminal(&self, request: PurgeRequest) -> Result<PurgeReceipt> {
        self.purge_terminal_inner(request).await
    }
}
