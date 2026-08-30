// src/drivers/failover.rs — Resilient multi-driver failover with lightweight circuit breaker.

use super::traits::MailDriver;
use crate::error::MailError;
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

#[async_trait]
impl MailDriver for FailoverDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        let tripped = self.is_tripped()?;

        let primary_error = if !tripped {
            match self.primary.send(message).await {
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

        for (idx, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.send(message).await {
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
                }
            }
        }

        Err(MailError::transport(
            "failover",
            "All mail drivers in failover chain failed",
        ))
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
}
