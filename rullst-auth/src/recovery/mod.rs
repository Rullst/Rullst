//! Opt-in authoritative credentials, password recovery and revocable sessions.
//!
//! Adopt this store explicitly for both password verification and session checks.
//! Legacy encrypted-only cookies and a separate application's password table do
//! not automatically participate in this transaction. Migrate before enabling it.
//! Mount request/consume endpoints behind independent ingress abuse controls,
//! secure headers, CSRF and no-store/referrer-policy protections.

mod crypto;
mod outbox;
mod store;
mod transactions;

pub use crypto::{
    RecoveryLocale, RecoveryNotice, RecoveryNoticeKind, RecoverySecrets, SecretToken,
};
pub use outbox::{ClaimedRecoveryNotice, RecoveryDeliveryFailure, RecoveryOutboxSnapshot};
pub use store::{AuthenticatedRecoveryAccount, SqlRecoveryStore};

/// Failures contain no recipient, credential, database URL or provider response.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryError {
    #[error("invalid recovery configuration")]
    Configuration,
    #[error("invalid account input")]
    InvalidInput,
    #[error("action is invalid, expired, replaced or already consumed")]
    InvalidAction,
    #[error("recovery storage is unavailable")]
    Storage,
    #[error("recovery cryptography failed")]
    Crypto,
    #[error("recovery capacity or request limit reached")]
    Limited,
}

impl From<sqlx::Error> for RecoveryError {
    fn from(_: sqlx::Error) -> Self {
        Self::Storage
    }
}

/// Identical public acknowledgement for known, unknown and throttled accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetRequestAccepted;

fn valid_subject(subject: &str) -> bool {
    (1..=128).contains(&subject.len())
        && subject
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn timestamp(now: u64) -> Result<i64, RecoveryError> {
    i64::try_from(now)
        .ok()
        .filter(|t| *t <= i64::MAX - 31 * 86400)
        .ok_or(RecoveryError::InvalidInput)
}

fn normalized_email(email: &str) -> Result<String, RecoveryError> {
    // Deliberately bounded ASCII account identifiers; no provider-specific dot,
    // plus-tag or Unicode equivalence transformations are inferred.
    let email = email.trim().to_ascii_lowercase();
    if email.len() > 254
        || !email.is_ascii()
        || email
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err(RecoveryError::InvalidInput);
    }
    let Some((local, domain)) = email.split_once('@') else {
        return Err(RecoveryError::InvalidInput);
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') || !domain.contains('.') {
        return Err(RecoveryError::InvalidInput);
    }
    Ok(email)
}
