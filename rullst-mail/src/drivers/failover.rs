// src/drivers/failover.rs — Resilient multi-driver failover with lightweight circuit breaker.

use super::traits::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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
    consecutive_failures: AtomicUsize,
    last_failure_at: RwLock<Option<Instant>>,
}

impl FailoverDriver {
    /// Creates a new `FailoverDriver` with the specified primary driver.
    pub fn new(primary: impl MailDriver + 'static) -> Self {
        Self {
            primary: Arc::new(primary),
            fallbacks: Vec::new(),
            failure_threshold: 3,
            cooldown: Duration::from_secs(60),
            consecutive_failures: AtomicUsize::new(0),
            last_failure_at: RwLock::new(None),
        }
    }

    /// Creates a new `FailoverDriver` with an `Arc`-wrapped primary driver.
    pub fn new_arc(primary: Arc<dyn MailDriver>) -> Self {
        Self {
            primary,
            fallbacks: Vec::new(),
            failure_threshold: 3,
            cooldown: Duration::from_secs(60),
            consecutive_failures: AtomicUsize::new(0),
            last_failure_at: RwLock::new(None),
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
    pub fn is_tripped(&self) -> bool {
        let failures = self.consecutive_failures.load(Ordering::SeqCst);
        if failures < self.failure_threshold {
            return false;
        }

        if let Ok(lock) = self.last_failure_at.read()
            && let Some(last_fail) = *lock
        {
            last_fail.elapsed() < self.cooldown
        } else {
            false
        }
    }

    /// Manually resets the circuit breaker failure counter and cooldown timer.
    pub fn reset_circuit(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        if let Ok(mut lock) = self.last_failure_at.write() {
            *lock = None;
        }
    }

    /// Returns the current number of recorded consecutive failures on the primary driver.
    pub fn failure_count(&self) -> usize {
        self.consecutive_failures.load(Ordering::SeqCst)
    }

    /// Returns the number of configured fallback drivers.
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.len()
    }
}

#[async_trait]
impl MailDriver for FailoverDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        let tripped = self.is_tripped();

        let primary_err_msg = if !tripped {
            match self.primary.send(message).await {
                Ok(()) => {
                    if self.consecutive_failures.load(Ordering::SeqCst) > 0 {
                        tracing::info!(
                            "Primary mail driver recovered and dispatched successfully. Resetting circuit breaker."
                        );
                        self.reset_circuit();
                    }
                    return Ok(());
                }
                Err(err) => {
                    self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
                    if let Ok(mut lock) = self.last_failure_at.write() {
                        *lock = Some(Instant::now());
                    }
                    tracing::warn!(
                        "Primary mail driver failed (error: {}). Triggering failover across {} fallback driver(s)...",
                        err,
                        self.fallbacks.len()
                    );
                    Some(err.to_string())
                }
            }
        } else {
            tracing::warn!(
                "Primary mail driver is tripped in cooldown ({:?}). Routing directly to fallback drivers.",
                self.cooldown
            );
            Some("Primary driver circuit breaker is tripped".to_string())
        };

        if self.fallbacks.is_empty() {
            return Err(MailError::SendError(format!(
                "Primary mail driver failed and no fallback drivers are configured: {}",
                primary_err_msg.unwrap_or_else(|| "Unknown primary error".to_string())
            )));
        }

        let mut fallback_errors = Vec::new();
        for (idx, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.send(message).await {
                Ok(()) => {
                    tracing::info!(
                        "Mail successfully dispatched via fallback driver index {}.",
                        idx
                    );
                    return Ok(());
                }
                Err(err) => {
                    tracing::warn!("Fallback mail driver index {} failed: {}", idx, err);
                    fallback_errors.push(format!("[Fallback {}]: {}", idx, err));
                }
            }
        }

        Err(MailError::SendError(format!(
            "All mail drivers in failover chain failed. Primary: {}. Fallbacks: [{}]",
            primary_err_msg.unwrap_or_default(),
            fallback_errors.join(", ")
        )))
    }
}
