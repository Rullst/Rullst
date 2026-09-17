/// Errors intentionally exclude provider payloads and subject identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AgeError {
    #[error("invalid age-assurance configuration")]
    InvalidConfiguration,
    #[error("the method does not satisfy the configured risk policy")]
    MethodNotAllowed,
    #[error("invalid age-assurance challenge or clock")]
    InvalidChallenge,
    #[error("age-assurance challenge expired")]
    Expired,
    #[error("age evidence does not match the current policy or authenticated context")]
    BindingMismatch,
    #[error("invalid or oversized age attestation")]
    InvalidAttestation,
    #[error("age attestation signature is invalid")]
    InvalidSignature,
    #[error("age evidence was already consumed")]
    Replay,
    #[error("shared durable replay protection is required in production")]
    DurableReplayRequired,
    #[error("replay protection is unavailable")]
    StoreUnavailable,
    #[error("replay protection capacity reached")]
    StoreCapacity,
    #[error("offline age evidence cannot authorize production access")]
    MockInProduction,
    #[error("operating-system randomness is unavailable")]
    EntropyUnavailable,
}
