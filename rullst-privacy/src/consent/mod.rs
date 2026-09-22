//! Explicit, versioned choices for optional processing. This is not a legal
//! basis selector, an age/guardian check or an application authorization layer.

mod gate;
mod memory;
#[cfg(feature = "consent-postgres")]
mod postgres;
#[cfg(feature = "consent-sqlite")]
mod sqlite;
mod state;

pub use gate::{ConsentClock, ConsentGate, SystemConsentClock};
pub use memory::MemoryConsentStore;
#[cfg(feature = "consent-postgres")]
pub use postgres::PostgresConsentStore;
#[cfg(feature = "consent-sqlite")]
pub use sqlite::SqliteConsentStore;
pub use state::{
    ConsentChoice, ConsentPurpose, ConsentRecord, ConsentSubject, ConsentSubmission, ConsentUpdate,
};

use std::future::Future;

/// Errors contain no subject references, storage details or application data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConsentError {
    #[error("invalid optional-processing purpose or choice")]
    InvalidConfiguration,
    #[error("consent state differs from the authenticated scope")]
    BindingMismatch,
    #[error("the consent revision changed; display the current choice again")]
    RevisionConflict,
    #[error("the consent revision cannot advance")]
    RevisionExhausted,
    #[error("shared durable consent state is required in production")]
    DurableStateRequired,
    #[error("consent storage is unavailable")]
    StoreUnavailable,
    #[error("consent storage has reached its configured capacity")]
    StoreCapacity,
    #[error("consent state or storage configuration is invalid")]
    StoreConfiguration,
    #[error("trusted server time is invalid or moved backwards")]
    ClockRollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentDurability {
    ProcessLocal,
    SharedDurable,
}

/// Trusted server integration. Each read and update must serialize against one
/// durable, global clock high-water mark and reject rollback before returning.
/// Updates must apply `ConsentUpdate::apply_to` atomically to the current record,
/// not a snapshot read outside that transaction. Persist completed withdrawals
/// across all application hosts and restarts; never evict records for capacity.
/// An uncertain/cancelled commit returns no success. Backup rollback and domain
/// effect atomicity require application/deployment controls outside this trait.
pub trait ConsentStore: Send + Sync {
    fn durability(&self) -> ConsentDurability;

    fn read(
        &self,
        subject: &ConsentSubject,
        purpose_id: &str,
        now: i64,
    ) -> impl Future<Output = Result<ConsentRecord, ConsentError>> + Send;

    fn update(
        &self,
        update: &ConsentUpdate,
    ) -> impl Future<Output = Result<ConsentRecord, ConsentError>> + Send;
}

impl<S: ConsentStore> ConsentStore for std::sync::Arc<S> {
    fn durability(&self) -> ConsentDurability {
        (**self).durability()
    }

    async fn read(
        &self,
        subject: &ConsentSubject,
        purpose_id: &str,
        now: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        (**self).read(subject, purpose_id, now).await
    }

    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        (**self).update(update).await
    }
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[cfg(any(feature = "consent-sqlite", feature = "consent-postgres"))]
fn scope_digest(subject: &ConsentSubject, purpose: &str) -> ring::digest::Digest {
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    digest.update(b"rullst.optional-consent-scope.v1\0");
    for value in [subject.tenant_ref(), subject.subject_ref(), purpose] {
        digest.update(&(value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    digest.finish()
}

#[cfg(any(feature = "consent-sqlite", feature = "consent-postgres"))]
fn validate_capacity(capacity: usize) -> Result<(), ConsentError> {
    if !(1..=100_000).contains(&capacity) {
        return Err(ConsentError::InvalidConfiguration);
    }
    Ok(())
}
