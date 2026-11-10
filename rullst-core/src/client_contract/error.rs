use thiserror::Error;

impl ClientContractError {
    pub(crate) fn invalid_json(error: serde_json::Error) -> Self {
        Self::InvalidJson(ContractJsonError::new(error))
    }
}

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
    InvalidJson(#[source] ContractJsonError),
    /// A typed response could not be serialized.
    #[error("client contract JSON encoding failed")]
    EncodeJson(#[source] ContractJsonError),
}

/// The JSON decoder's or encoder's own error behind
/// [`ClientContractError::InvalidJson`] and [`ClientContractError::EncodeJson`].
///
/// `Display` is that error's message, which names the line and column of a
/// syntax error. The JSON library is an implementation detail, so its error
/// type is not exposed.
#[derive(Debug)]
pub struct ContractJsonError(serde_json::Error);

impl ContractJsonError {
    pub(crate) fn new(error: serde_json::Error) -> Self {
        Self(error)
    }
}

impl std::fmt::Display for ContractJsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for ContractJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}
