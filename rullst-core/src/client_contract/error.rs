use thiserror::Error;

/// Fail-closed client-contract validation or codec error.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClientContractError {
    /// Wire versions must be positive.
    #[error("client contract version must be positive")]
    InvalidVersion,
    /// Request identifiers must satisfy the bounded token grammar.
    #[error("client request id must be a 1-128 byte ASCII token")]
    InvalidRequestId,
    /// Mutation replay keys must satisfy the bounded token grammar.
    #[error("client idempotency key must be an 8-128 byte ASCII token")]
    InvalidIdempotencyKey,
    /// Failure codes must use the stable dotted grammar.
    #[error("client failure code must be a bounded lowercase dotted identifier")]
    InvalidFailureCode,
    /// A negotiation offer contained no version.
    #[error("client version offer must not be empty")]
    EmptyVersionOffer,
    /// A negotiation offer exceeded its structural bound.
    #[error("client version offer exceeds 16 entries")]
    TooManyOfferedVersions,
    /// Server policy range or body limit is invalid.
    #[error("client contract policy range or body limit is invalid")]
    InvalidPolicy,
    /// Client and server share no supported version.
    #[error("client and server have no mutually supported contract version")]
    NoMutualVersion,
    /// A request or response selected a version outside server policy.
    #[error(
        "client contract version {received} is outside the supported range {minimum}..={current}"
    )]
    UnsupportedVersion {
        /// Received version.
        received: u16,
        /// Minimum supported version.
        minimum: u16,
        /// Latest supported version.
        current: u16,
    },
    /// A state-changing request omitted its replay key.
    #[error("client mutation requires an idempotency key")]
    MissingIdempotencyKey,
    /// Encoded content exceeded the policy ceiling.
    #[error("client contract body exceeds the {maximum}-byte maximum")]
    BodyTooLarge {
        /// Configured maximum size.
        maximum: usize,
    },
    /// Incoming JSON did not satisfy the typed envelope.
    #[error("client contract JSON is invalid")]
    InvalidJson(#[source] serde_json::Error),
    /// A typed response could not be serialized.
    #[error("client contract JSON encoding failed")]
    EncodeJson(#[source] serde_json::Error),
}
