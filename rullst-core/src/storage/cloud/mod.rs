//! Optional, explicit private AWS S3 and Cloudflare R2 storage.

mod client;
mod config;
mod mock;
mod signing;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

pub(crate) use client::CloudClient;
pub use config::{CloudCredentials, CloudStorageConfig};
pub(crate) use signing::validate_key;
use std::{fmt, time::SystemTime};

/// Cloud failures never contain credentials, signed URLs, object keys or response bodies.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CloudError {
    /// Invalid credentials, endpoint, provider parameters or resource limits.
    #[error("invalid cloud storage configuration")]
    Configuration,
    /// The key is empty, ambiguous, too long or contains unsafe components.
    #[error("invalid cloud object key")]
    InvalidKey,
    /// An upload or response exceeds the configured object size.
    #[error("cloud object size limit exceeded")]
    SizeLimit,
    /// Explicit temporary credentials have expired or cannot cover the requested grant.
    #[error("cloud credentials expired or expire before this grant")]
    CredentialsExpired,
    /// The supplied presigned URL lifetime is outside the supported 1–900 seconds.
    #[error("signed download lifetime must be between 1 and 900 whole seconds")]
    InvalidLifetime,
    /// Provider response did not match the supported operation contract.
    #[error("invalid cloud storage response")]
    InvalidResponse,
    /// A remote request was rejected. Only the HTTP status is retained.
    #[error("cloud storage rejected the request (HTTP {0})")]
    Rejected(u16),
    /// No object exists at this key.
    #[error("cloud object not found")]
    NotFound,
    /// HTTP transport failed; the underlying URL-bearing error is deliberately omitted.
    #[error("cloud storage transport failed")]
    Transport,
    /// The complete operation exceeded its deadline.
    #[error("cloud storage operation timed out")]
    Timeout,
    /// Signing failed without exposing its request.
    #[error("cloud request signing failed")]
    Signing,
    /// Offline storage has reached its bounded capacity or its lock failed.
    #[error("offline cloud storage unavailable or full")]
    MockUnavailable,
    /// Offline mode cannot issue authentic provider download grants.
    #[error("offline cloud storage cannot issue a provider URL")]
    MockGrantUnsupported,
}

pub use super::ObjectMetadata;

/// A short-lived bearer grant. Debug is redacted; URL access is deliberately explicit.
#[derive(Clone)]
pub struct SignedDownload {
    pub(super) url: String,
    pub(super) expires_at: SystemTime,
}

impl SignedDownload {
    /// Returns the bearer URL. Never put this value in logs or analytics.
    pub fn expose_url(&self) -> &str {
        &self.url
    }
    /// Maximum requested expiry; provider credentials/policy can invalidate it earlier.
    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
}

impl fmt::Debug for SignedDownload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedDownload")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}
