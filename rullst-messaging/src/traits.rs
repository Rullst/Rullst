//! Static-dispatch broker and administration boundaries.

use crate::{
    AckToken, DeadLetter, DeadLetterQuery, Delivery, FailureCode, PublishReceipt, PublishRequest,
    PurgeReceipt, PurgeRequest, ReceiveRequest, Result, RetryDisposition, SubscriptionReceipt,
    SubscriptionRequest,
};
use std::future::Future;
use std::time::Duration;

/// Broker operations shared by deterministic fixtures and future remote adapters.
///
/// Delivery is at least once. A handler can finish its external effect and lose its lease before
/// acknowledgement, so consumers must use the envelope ID or a stable domain key carried in the
/// application payload at their own side-effect boundary.
pub trait MessageBroker: Send + Sync {
    /// Publishes exactly one bounded message, deduplicated by topic and idempotency key.
    fn publish(
        &self,
        request: PublishRequest,
    ) -> impl Future<Output = Result<PublishReceipt>> + Send;

    /// Registers an idempotent consumer-group view of one topic.
    fn subscribe(
        &self,
        request: SubscriptionRequest,
    ) -> impl Future<Output = Result<SubscriptionReceipt>> + Send;

    /// Claims a bounded batch for one consumer using expiring single-use acknowledgement leases.
    fn receive(
        &self,
        request: ReceiveRequest,
    ) -> impl Future<Output = Result<Vec<Delivery>>> + Send;

    /// Acknowledges one currently valid lease.
    fn ack(&self, token: &AckToken) -> impl Future<Output = Result<()>> + Send;

    /// Returns a valid lease for bounded delayed retry or automatic dead-lettering.
    fn retry(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> impl Future<Output = Result<RetryDisposition>> + Send;

    /// Moves one valid lease directly to the consumer group's dead-letter view.
    fn dead_letter(
        &self,
        token: &AckToken,
        failure_code: FailureCode,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Explicit operational surfaces that are intentionally separate from the hot delivery path.
pub trait MessageAdmin: MessageBroker {
    /// Lists bounded dead letters without exposing acknowledgement capabilities.
    fn dead_letters(
        &self,
        query: DeadLetterQuery,
    ) -> impl Future<Output = Result<Vec<DeadLetter>>> + Send;

    /// Removes retained messages terminal for every currently registered group.
    fn purge_terminal(
        &self,
        request: PurgeRequest,
    ) -> impl Future<Output = Result<PurgeReceipt>> + Send;
}
