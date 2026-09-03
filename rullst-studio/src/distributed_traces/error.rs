use std::fmt;

/// Errors returned by the bounded distributed trace contract.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceIngestionError {
    /// The HMAC key is shorter than 32 bytes or exceeds the supported bound.
    InvalidKey,
    /// The store capacity is zero or exceeds the supported bound.
    InvalidCapacity,
    /// A source, nonce, signature, timestamp, or MAC is invalid.
    AuthenticationFailed,
    /// The signed timestamp is outside the accepted clock window.
    TimestampOutsideWindow,
    /// A valid signed nonce has already been consumed.
    ReplayDetected,
    /// The replay cache or trace store is unavailable or full.
    StoreUnavailable,
    /// JSON encoding or decoding failed.
    InvalidEncoding,
    /// The batch or one of its spans violates the v1 bounds.
    InvalidBatch,
    /// The system clock cannot produce a Unix timestamp.
    ClockUnavailable,
    /// The operating system could not produce a fresh nonce.
    RandomnessUnavailable,
}

impl fmt::Display for TraceIngestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidKey => "trace ingestion HMAC key must contain 32 to 128 bytes",
            Self::InvalidCapacity => "trace store capacity is outside the supported range",
            Self::AuthenticationFailed => "trace ingestion authentication failed",
            Self::TimestampOutsideWindow => {
                "trace ingestion timestamp is outside the accepted window"
            }
            Self::ReplayDetected => "trace ingestion request was already consumed",
            Self::StoreUnavailable => "trace ingestion state is unavailable or at capacity",
            Self::InvalidEncoding => "trace ingestion payload encoding is invalid",
            Self::InvalidBatch => "trace ingestion batch violates the v1 contract",
            Self::ClockUnavailable => "system clock cannot produce a Unix timestamp",
            Self::RandomnessUnavailable => {
                "operating-system randomness is unavailable for trace ingestion"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TraceIngestionError {}
