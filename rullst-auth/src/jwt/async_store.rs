use super::{ApplicationJwtClaims, JwtError};

/// Whether revocation state is local to one process or shared by a backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtRevocationMode {
    ProcessLocal,
    Shared,
}

/// Async revocation contract for shared durable backends.
///
/// Implementations must make a completed revocation visible to every verifier
/// that shares the same authoritative backend. Reporting
/// [`JwtRevocationMode::Shared`] is a deployment assertion that production
/// policy enforces but cannot independently prove.
pub trait AsyncJwtRevocationStore: Send + Sync {
    /// Declares whether every verifier using this backend observes shared state.
    fn mode(&self) -> JwtRevocationMode;

    /// Returns whether the claims are revoked at the supplied Unix timestamp.
    fn is_revoked(
        &self,
        claims: &ApplicationJwtClaims,
        now: u64,
    ) -> impl std::future::Future<Output = Result<bool, JwtError>> + Send;
}
