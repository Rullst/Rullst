//! Secret-minimized delivery observations for metrics and tracing adapters.

use crate::drivers::MailDriver;
use crate::{DeliveryPipeline, MailError, MailFailureClass, Message};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_OBSERVATIONS: usize = 1_000_000;
const MAX_PROVIDER_BYTES: usize = 64;

/// Terminal outcome recorded by [`ObservedMailDriver`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MailDeliveryOutcome {
    /// The wrapped driver accepted the message.
    Delivered,
    /// Mandatory pre-flight rejected the message before transport.
    PreflightRejected,
    /// A permanent provider, policy or configuration failure occurred.
    PermanentFailure,
    /// A retryable transport/provider failure occurred.
    TransientFailure,
    /// The provider returned a rate-limit response.
    RateLimited,
}

/// Low-cardinality observation with no recipient, subject, body or filename.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailDeliveryObservation {
    provider: String,
    outcome: MailDeliveryOutcome,
    elapsed_microseconds: u64,
    attachment_count: usize,
    scheduled: bool,
    tenant_scoped: bool,
}

impl MailDeliveryObservation {
    /// Returns the validated provider label.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Returns the terminal delivery outcome.
    pub const fn outcome(&self) -> MailDeliveryOutcome {
        self.outcome
    }

    /// Returns elapsed time rounded down to microseconds.
    pub const fn elapsed_microseconds(&self) -> u64 {
        self.elapsed_microseconds
    }

    /// Returns the bounded attachment count observed before dispatch.
    pub const fn attachment_count(&self) -> usize {
        self.attachment_count
    }

    /// Returns whether the message carried a schedule timestamp.
    pub const fn scheduled(&self) -> bool {
        self.scheduled
    }

    /// Returns whether dispatch used an explicit tenant context.
    pub const fn tenant_scoped(&self) -> bool {
        self.tenant_scoped
    }
}

/// Non-failing sink contract suitable for local metrics/tracing adapters.
///
/// Sinks must not block or panic. Delivery results remain authoritative even if
/// an observer drops data after provider acceptance; returning a telemetry error
/// at that point could cause an unsafe duplicate retry.
pub trait MailDeliveryObserver: Send + Sync {
    /// Records one minimized terminal observation.
    fn observe(&self, observation: &MailDeliveryObservation);
}

/// Typed local observer failures.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MailObservationError {
    /// The configured capacity is outside the supported range.
    InvalidCapacity,
    /// The local observation state is unavailable.
    StateUnavailable,
}

impl std::fmt::Display for MailObservationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCapacity => formatter.write_str("invalid mail observation capacity"),
            Self::StateUnavailable => formatter.write_str("mail observation state unavailable"),
        }
    }
}

impl std::error::Error for MailObservationError {}

/// Snapshot from one bounded process-local observer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailObservationSnapshot {
    observations: Vec<MailDeliveryObservation>,
    evicted: u64,
    capacity: usize,
}

impl MailObservationSnapshot {
    /// Returns observations in oldest-to-newest order.
    pub fn observations(&self) -> &[MailDeliveryObservation] {
        &self.observations
    }

    /// Returns the number of oldest records evicted at capacity.
    pub const fn evicted(&self) -> u64 {
        self.evicted
    }

    /// Returns the configured record capacity.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Bounded deterministic process-local observation sink.
#[derive(Clone)]
pub struct BoundedMailObserver {
    state: Arc<Mutex<ObserverState>>,
    capacity: usize,
}

#[derive(Default)]
struct ObserverState {
    observations: VecDeque<MailDeliveryObservation>,
    evicted: u64,
}

impl BoundedMailObserver {
    /// Creates a bounded local sink.
    pub fn new(capacity: usize) -> Result<Self, MailObservationError> {
        if !(1..=MAX_OBSERVATIONS).contains(&capacity) {
            return Err(MailObservationError::InvalidCapacity);
        }
        Ok(Self {
            state: Arc::new(Mutex::new(ObserverState::default())),
            capacity,
        })
    }

    /// Returns a stable snapshot without exposing message content.
    pub fn snapshot(&self) -> Result<MailObservationSnapshot, MailObservationError> {
        let state = self
            .state
            .lock()
            .map_err(|_| MailObservationError::StateUnavailable)?;
        Ok(MailObservationSnapshot {
            observations: state.observations.iter().cloned().collect(),
            evicted: state.evicted,
            capacity: self.capacity,
        })
    }
}

impl MailDeliveryObserver for BoundedMailObserver {
    fn observe(&self, observation: &MailDeliveryObservation) {
        if let Ok(mut state) = self.state.lock() {
            if state.observations.len() == self.capacity {
                state.observations.pop_front();
                state.evicted = state.evicted.saturating_add(1);
            }
            state.observations.push_back(observation.clone());
        }
    }
}

/// Static-dispatch wrapper which records minimized terminal outcomes.
pub struct ObservedMailDriver<D, O> {
    provider: String,
    driver: D,
    observer: O,
}

impl<D, O> ObservedMailDriver<D, O> {
    /// Creates a wrapper after validating its low-cardinality provider label.
    pub fn try_new(provider: impl Into<String>, driver: D, observer: O) -> Result<Self, MailError> {
        let provider = provider.into();
        if provider.is_empty()
            || provider.len() > MAX_PROVIDER_BYTES
            || !provider
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        {
            return Err(MailError::ConfigError(
                "mail observation provider label is invalid".to_string(),
            ));
        }
        Ok(Self {
            provider,
            driver,
            observer,
        })
    }

    /// Returns the wrapped driver.
    pub const fn driver(&self) -> &D {
        &self.driver
    }

    /// Returns the configured observer.
    pub const fn observer(&self) -> &O {
        &self.observer
    }

    fn observe(
        &self,
        started: Instant,
        message: &Message,
        tenant_scoped: bool,
        outcome: MailDeliveryOutcome,
    ) where
        O: MailDeliveryObserver,
    {
        let micros = started.elapsed().as_micros();
        self.observer.observe(&MailDeliveryObservation {
            provider: self.provider.clone(),
            outcome,
            elapsed_microseconds: u64::try_from(micros).unwrap_or(u64::MAX),
            attachment_count: message.attachments.len(),
            scheduled: message.send_at.is_some(),
            tenant_scoped,
        });
    }

    fn outcome(error: &MailError) -> MailDeliveryOutcome {
        match error.failure_class() {
            MailFailureClass::Permanent => MailDeliveryOutcome::PermanentFailure,
            MailFailureClass::Transient => MailDeliveryOutcome::TransientFailure,
            MailFailureClass::RateLimited => MailDeliveryOutcome::RateLimited,
        }
    }
}

#[async_trait]
impl<D, O> MailDriver for ObservedMailDriver<D, O>
where
    D: MailDriver,
    O: MailDeliveryObserver,
{
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let started = Instant::now();
        let prepared = match DeliveryPipeline::prepare(message) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.observe(
                    started,
                    message,
                    false,
                    MailDeliveryOutcome::PreflightRejected,
                );
                return Err(error);
            }
        };
        let result = self.driver.send(prepared.message()).await;
        let outcome = result
            .as_ref()
            .map_or_else(Self::outcome, |_| MailDeliveryOutcome::Delivered);
        self.observe(started, prepared.message(), false, outcome);
        result
    }

    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        let started = Instant::now();
        let prepared = match DeliveryPipeline::prepare_for_tenant(tenant_id, message) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.observe(
                    started,
                    message,
                    true,
                    MailDeliveryOutcome::PreflightRejected,
                );
                return Err(error);
            }
        };
        let result = self
            .driver
            .send_for_tenant(tenant_id, prepared.message())
            .await;
        let outcome = result
            .as_ref()
            .map_or_else(Self::outcome, |_| MailDeliveryOutcome::Delivered);
        self.observe(started, prepared.message(), true, outcome);
        result
    }
}

#[cfg(test)]
mod tests;
