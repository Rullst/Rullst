//! Standalone Redis Streams with Rullst-owned fenced delivery indexes.

mod config;
mod operations;
mod transport;

use crate::{InMemoryBroker, Result};
pub use config::RedisBrokerConfig;
use std::{fmt, sync::Arc};
use transport::Remote;

/// Optional remote broker; clones share bounded connection admission.
///
/// Provision once, then use `connect` on application startup. The caller owns
/// Redis AOF/no-eviction configuration, ACLs, restore operations and topic access.
/// An operation error may follow a committed write: reconcile via idempotency or
/// redelivery. No write is automatically retried after an ambiguous response.
#[derive(Clone)]
pub struct RedisBroker {
    backend: Backend,
}

#[derive(Clone)]
enum Backend {
    Mock(InMemoryBroker),
    Remote(Arc<Remote>),
}

impl RedisBroker {
    /// Explicitly provisions an empty namespace, or checks an identical existing one.
    /// Never use this as a startup fallback after a missing-state error.
    pub async fn provision(config: RedisBrokerConfig) -> Result<Self> {
        Self::open(config, true).await
    }

    /// Opens only an already provisioned live namespace, validating generation/limits.
    pub async fn connect(config: RedisBrokerConfig) -> Result<Self> {
        Self::open(config, false).await
    }

    async fn open(config: RedisBrokerConfig, provision: bool) -> Result<Self> {
        if config.is_mock() {
            return Ok(Self {
                backend: Backend::Mock(InMemoryBroker::new(config.broker)),
            });
        }
        let remote = Remote::open(config, provision).await?;
        Ok(Self {
            backend: Backend::Remote(Arc::new(remote)),
        })
    }

    /// Returns whether this instance is explicitly using process-local fixture state.
    pub fn is_mock(&self) -> bool {
        matches!(self.backend, Backend::Mock(_))
    }
}

impl fmt::Debug for RedisBroker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisBroker")
            .field("is_mock", &self.is_mock())
            .finish_non_exhaustive()
    }
}
