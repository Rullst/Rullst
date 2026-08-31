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
    /// An internal invariant failed closed instead of losing or acknowledging a message.
    #[error("messaging state invariant failed at {context}")]
    InternalState {
        /// Static diagnostic location without message data.
        context: &'static str,
    },
}

/// Convenient result alias for messaging operations.
pub type Result<T> = std::result::Result<T, MessagingError>;
