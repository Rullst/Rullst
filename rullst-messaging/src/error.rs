//! Typed messaging errors that avoid embedding payloads, headers, or credentials.

/// Errors emitted by the broker-neutral messaging boundary.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessagingError {
    /// A public value failed its documented bounded grammar.
    #[error("invalid {field}: {reason}")]
    Invalid {
        /// Stable field name.
        field: &'static str,
        /// Stable reason without the rejected value.
        reason: &'static str,
    },
    /// A configured in-memory or transport resource limit was reached.
    #[error("{resource} capacity limit of {limit} was reached")]
    CapacityExceeded {
        /// Stable resource class.
        resource: &'static str,
        /// Configured upper bound.
        limit: usize,
    },
    /// An idempotency key was reused with different message content.
    #[error("idempotency key was reused with different message content")]
    IdempotencyConflict,
    /// The requested topic/group subscription has not been registered.
    #[error("message subscription was not found")]
    SubscriptionNotFound,
    /// The acknowledgement capability is unknown or was already consumed.
    #[error("acknowledgement lease was not found or was already consumed")]
    LeaseNotFound,
    /// The acknowledgement capability was presented after its lease expired.
    #[error("acknowledgement lease has expired")]
    LeaseExpired,
    /// The system clock could not produce a supported Unix millisecond value.
    #[error("system clock is outside the supported Unix millisecond range")]
    ClockOutOfRange,
    /// The durable adapter could not complete a storage operation.
    #[error("durable messaging storage failed during {operation}")]
    StorageUnavailable {
        /// Static operation label without paths, SQL, payloads, or credentials.
        operation: &'static str,
    },
    /// Persisted state violated the versioned durable-adapter contract.
    #[error("durable messaging storage is corrupt at {context}")]
    CorruptStorage {
        /// Static diagnostic location without persisted values.
        context: &'static str,
    },
    /// A namespace was reopened with different operational limits.
    #[error("durable messaging namespace configuration does not match persisted state")]
    ConfigurationConflict,
    /// An encrypted durable record references a rotation key not in the supplied keyring.
    #[error("durable messaging storage requires an unavailable rotation key")]
    StorageKeyUnavailable,
    /// The operating system could not generate a fresh authenticated-encryption nonce.
    #[error("durable messaging storage could not obtain secure randomness")]
    StorageRandomnessUnavailable,
    /// Authenticated encryption failed without exposing stored message data.
    #[error("durable messaging storage encryption failed")]
    StorageEncryptionFailed,
    /// A durable encrypted record, its key, or its bound metadata failed authentication.
    #[error("durable messaging encrypted storage authentication failed")]
    StorageAuthenticationFailed,
    /// A remote frame uses a schema or codec version this release does not implement.
    #[error("messaging wire version is unsupported")]
    UnsupportedWireVersion,
    /// A remote frame is malformed, non-canonical, oversized, or violates envelope bounds.
    #[error("messaging wire envelope is invalid")]
    InvalidWireEnvelope,
    /// An internal invariant failed closed instead of losing or acknowledging a message.
    #[error("messaging state invariant failed at {context}")]
    InternalState {
        /// Static diagnostic location without message data.
        context: &'static str,
    },
}

/// Convenient result alias for messaging operations.
pub type Result<T> = std::result::Result<T, MessagingError>;
