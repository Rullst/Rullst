// src/drivers/failover.rs — Resilient multi-driver failover with lightweight circuit breaker.

use super::traits::MailDriver;
use crate::error::{MailError, MailFailureClass};
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct CircuitState {
    consecutive_failures: usize,
    last_failure_at: Option<Instant>,
}

/// Resilient multi-driver failover dispatcher with an integrated circuit breaker.
///
/// Dispatches outgoing emails via a designated primary driver (e.g. `ResendDriver`).
/// If the primary driver fails or is tripped by repeated consecutive failures,
/// the failover engine automatically routes messages across configured fallback drivers
/// (e.g. `SendGridDriver`, `SmtpDriver`) with structured telemetry warnings.
pub struct FailoverDriver {
    primary: Arc<dyn MailDriver>,
    fallbacks: Vec<Arc<dyn MailDriver>>,
    failure_threshold: usize,
    cooldown: Duration,
    circuit: Mutex<CircuitState>,
}

impl FailoverDriver {
    /// Creates a new `FailoverDriver` with the specified primary driver.
    pub fn new(primary: impl MailDriver + 'static) -> Self {
        Self {
            primary: Arc::new(primary),
            fallbacks: Vec::new(),
            failure_threshold: 3,
            cooldown: Duration::from_secs(60),
            circuit: Mutex::new(CircuitState {
                consecutive_failures: 0,
                last_failure_at: None,
            }),
        }
    }

    /// Creates a new `FailoverDriver` with an `Arc`-wrapped primary driver.
    pub fn new_arc(primary: Arc<dyn MailDriver>) -> Self {
        Self {
            primary,
            fallbacks: Vec::new(),
            failure_threshold: 3,
            cooldown: Duration::from_secs(60),
            circuit: Mutex::new(CircuitState {
                consecutive_failures: 0,
                last_failure_at: None,
            }),
        }
    }

    /// Appends a fallback mail driver to the contingency chain.
    pub fn with_fallback(mut self, fallback: impl MailDriver + 'static) -> Self {
        self.fallbacks.push(Arc::new(fallback));
        self
    }

    /// Appends an `Arc`-wrapped fallback mail driver to the contingency chain.
    pub fn with_fallback_arc(mut self, fallback: Arc<dyn MailDriver>) -> Self {
        self.fallbacks.push(fallback);
        self
    }

    /// Configures the consecutive failure threshold before the circuit breaker trips.
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.failure_threshold = threshold.max(1);
        self
    }

    /// Configures the circuit breaker cooldown duration.
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Checks if the primary driver circuit breaker is currently in the tripped state.
    pub fn is_tripped(&self) -> Result<bool, MailError> {
        let circuit = self.circuit.lock().map_err(|_| circuit_unavailable())?;
        Ok(circuit.consecutive_failures >= self.failure_threshold
            && circuit
                .last_failure_at
                .is_some_and(|last_failure| last_failure.elapsed() < self.cooldown))
    }

    /// Manually resets the circuit breaker failure counter and cooldown timer.
    pub fn reset_circuit(&self) -> Result<(), MailError> {
        let mut circuit = self.circuit.lock().map_err(|_| circuit_unavailable())?;
        circuit.consecutive_failures = 0;
        circuit.last_failure_at = None;
        Ok(())
    }

    /// Returns the current number of recorded consecutive failures on the primary driver.
    pub fn failure_count(&self) -> Result<usize, MailError> {
        Ok(self
            .circuit
            .lock()
            .map_err(|_| circuit_unavailable())?
            .consecutive_failures)
    }

    /// Returns the number of configured fallback drivers.
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.len()
    }

    fn record_primary_failure(&self) -> Result<usize, MailError> {
        let mut circuit = self.circuit.lock().map_err(|_| circuit_unavailable())?;
        circuit.consecutive_failures = circuit.consecutive_failures.saturating_add(1);
        circuit.last_failure_at = Some(Instant::now());
        Ok(circuit.consecutive_failures)
    }
}

fn circuit_unavailable() -> MailError {
    MailError::ConfigError("mail failover circuit state is unavailable".to_string())
}

/// How each driver in the chain is invoked, so every attempt keeps the
/// caller's delivery identity or tenant context.
#[derive(Clone, Copy)]
enum Route<'a> {
    Plain,
    DeliveryId(&'a str),
    Tenant(&'a str),
}

impl FailoverDriver {
    async fn dispatch(&self, message: &Message, route: Route<'_>) -> Result<(), MailError> {
        let prepared = match route {
            Route::Tenant(tenant_id) => DeliveryPipeline::prepare_for_tenant(tenant_id, message)?,
            Route::Plain | Route::DeliveryId(_) => DeliveryPipeline::prepare(message)?,
        };
        let message = prepared.message();
        let tripped = self.is_tripped()?;

        let primary_error = if !tripped {
            match dispatch_to(self.primary.as_ref(), message, route).await {
                Ok(()) => {
                    if self.failure_count()? > 0 {
                        tracing::info!(
                            event = "mail.failover.primary_recovered",
                            "Primary mail driver recovered and dispatched successfully. Resetting circuit breaker."
                        );
                        self.reset_circuit()?;
                    }
                    return Ok(());
                }
                Err(err) => {
                    let class = err.failure_class();
                    if !err.is_failover_eligible() {
                        tracing::warn!(
                            event = "mail.failover.primary_permanent_failure",
                            failure.class = class.as_str(),
                            "Primary mail driver returned a permanent failure; fallback is suppressed"
                        );
                        return Err(err);
                    }
                    let failures = self.record_primary_failure()?;
                    tracing::warn!(
                        event = "mail.failover.primary_retryable_failure",
                        failure.class = class.as_str(),
                        failure.count = failures,
                        retry_after_seconds = err.retry_after().map(|delay| delay.as_secs()),
                        fallback.count = self.fallbacks.len(),
                        "Primary mail driver failed transiently; attempting configured fallbacks"
                    );
                    err
                }
            }
        } else {
            tracing::warn!(
                event = "mail.failover.circuit_open",
                cooldown_seconds = self.cooldown.as_secs(),
                fallback.count = self.fallbacks.len(),
                "Primary mail driver circuit is open; routing directly to fallbacks"
            );
            MailError::transport("failover", "primary driver circuit is open")
        };

        if self.fallbacks.is_empty() {
            return Err(primary_error);
        }

        let mut rate_limit = None;
        for (idx, fallback) in self.fallbacks.iter().enumerate() {
            match dispatch_to(fallback.as_ref(), message, route).await {
                Ok(()) => {
                    tracing::info!(
                        event = "mail.failover.fallback_succeeded",
                        fallback.index = idx,
                        "Mail dispatched via fallback driver"
                    );
                    return Ok(());
                }
                Err(err) => {
                    tracing::warn!(
                        event = "mail.failover.fallback_failed",
                        fallback.index = idx,
                        failure.class = err.failure_class().as_str(),
                        "Fallback mail driver failed"
                    );
                    // A permanent rejection (suppressed recipient, rejected
                    // attachment, invalid message) or a fail-closed guard
                    // outage ends the chain with its typed error, as it
                    // does for the primary.
                    if !err.is_failover_eligible() {
                        return Err(err);
                    }
                    rate_limit = (err.failure_class() == MailFailureClass::RateLimited)
                        .then(|| err.retry_after());
                }
            }
        }

        const EXHAUSTED: &str = "All mail drivers in failover chain failed";
        Err(match rate_limit {
            // The last driver asked to slow down; keep its bounded delay.
            Some(retry_after) => MailError::RateLimited {
                provider: "failover",
                message: EXHAUSTED.to_string(),
                retry_after,
            },
            None => MailError::transport("failover", EXHAUSTED),
        })
    }
}

async fn dispatch_to(
    driver: &dyn MailDriver,
    message: &Message,
    route: Route<'_>,
) -> Result<(), MailError> {
    match route {
        Route::Plain => driver.send(message).await,
        Route::DeliveryId(id) => driver.send_with_delivery_id(message, id).await,
        Route::Tenant(tenant_id) => driver.send_for_tenant(tenant_id, message).await,
    }
}

#[async_trait]
impl MailDriver for FailoverDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        self.dispatch(message, Route::Plain).await
    }

    // Identities are scoped by each provider. Switching providers after an
    // ambiguous acceptance still has at-least-once semantics.
    async fn send_with_delivery_id(
        &self,
        message: &Message,
        delivery_id: &str,
    ) -> Result<(), MailError> {
        super::traits::validate_delivery_id(delivery_id)?;
        self.dispatch(message, Route::DeliveryId(delivery_id)).await
    }

    // Tenant-aware primaries and fallbacks (e.g. `TenantMailResolver`) must
    // receive the tenant rather than their default no-tenant route.
    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        self.dispatch(message, Route::Tenant(tenant_id)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::MemoryDriver;

    #[tokio::test]
    async fn poisoned_circuit_state_fails_before_primary_delivery() {
        let (primary, primary_store) = MemoryDriver::isolated();
        let failover = Arc::new(FailoverDriver::new(primary));
        let poison_target = Arc::clone(&failover);
        let poisoner = std::thread::spawn(move || {
            let _guard = poison_target
                .circuit
                .lock()
                .expect("circuit lock before intentional poison");
            panic!("intentional test-only circuit poison");
        });
        assert!(poisoner.join().is_err());

        let result = failover
            .send(&Message::new().to("user@example.com").subject("Fail closed"))
            .await;
        assert!(matches!(result, Err(MailError::ConfigError(_))));
        assert!(primary_store.lock().expect("primary store").is_empty());
    }

    struct TransientFailure;

    #[async_trait]
    impl MailDriver for TransientFailure {
        async fn send(&self, _message: &Message) -> Result<(), MailError> {
            Err(MailError::transport("fixture", "unavailable"))
        }
    }

    struct Failing(MailError);

    #[async_trait]
    impl MailDriver for Failing {
        async fn send(&self, _message: &Message) -> Result<(), MailError> {
            Err(self.0.clone())
        }
    }

    #[tokio::test]
    async fn fallback_chain_keeps_typed_permanent_and_rate_limit_errors() {
        let message = Message::new().to("user@example.com").subject("Chain");
        let suppressed = MailError::SuppressedRecipient { reason: "manual" };
        let (unguarded, unguarded_store) = MemoryDriver::isolated();
        let failover = FailoverDriver::new(TransientFailure)
            .with_fallback(Failing(suppressed.clone()))
            .with_fallback(unguarded);
        assert_eq!(failover.send(&message).await, Err(suppressed));
        assert!(unguarded_store.lock().expect("store").is_empty());

        let slow = MailError::from_provider_response(
            "fixture",
            429,
            "slow down",
            Some(Duration::from_secs(30)),
        );
        let failover = FailoverDriver::new(TransientFailure)
            .with_fallback(TransientFailure)
            .with_fallback(Failing(slow));
        let error = failover.send(&message).await.expect_err("chain exhausted");
        assert_eq!(error.failure_class(), MailFailureClass::RateLimited);
        assert_eq!(error.retry_after(), Some(Duration::from_secs(30)));
        assert!(
            error
                .to_string()
                .contains("All mail drivers in failover chain failed")
        );
    }

    #[tokio::test]
    async fn tenant_context_reaches_the_primary_and_fallbacks() {
        use crate::TenantMailResolver;
        let message = Message::new()
            .to("user@example.com")
            .subject("Tenant route");

        let (tenant, tenant_store) = MemoryDriver::isolated();
        let (global, global_store) = MemoryDriver::isolated();
        let primary = TenantMailResolver::with_default(global);
        primary
            .register("tenant_acme", tenant)
            .expect("register tenant");
        let failover = FailoverDriver::new(primary).with_fallback(TransientFailure);
        failover
            .send_for_tenant("tenant_acme", &message)
            .await
            .expect("tenant delivery");
        assert_eq!(tenant_store.lock().expect("tenant store").len(), 1);
        assert!(global_store.lock().expect("global store").is_empty());

        let (tenant, tenant_store) = MemoryDriver::isolated();
        let fallback = TenantMailResolver::new();
        fallback
            .register("tenant_acme", tenant)
            .expect("register tenant");
        let failover = FailoverDriver::new(TransientFailure).with_fallback(fallback);
        failover
            .send_for_tenant("tenant_acme", &message)
            .await
            .expect("fallback tenant delivery");
        assert_eq!(tenant_store.lock().expect("tenant store").len(), 1);
        assert!(failover.send_for_tenant("../acme", &message).await.is_err());
    }
}
