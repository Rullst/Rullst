use std::fmt;

/// Errors returned by the explicit polyglot persistence boundary.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PolyglotError {
    /// A collection, database, namespace, or document identifier was rejected.
    #[error("invalid {kind}: {reason}")]
    InvalidIdentifier {
        /// Identifier category.
        kind: &'static str,
        /// Stable, credential-free failure detail.
        reason: &'static str,
    },
    /// An adapter configuration is unsafe or incomplete.
    #[error("invalid {backend} configuration: {reason}")]
    InvalidConfiguration {
        /// Backend being configured.
        backend: &'static str,
        /// Stable, credential-free failure detail.
        reason: &'static str,
    },
    /// A record with the requested identifier already exists.
    #[error("document already exists")]
    Conflict,
    /// A record required by a mutating operation does not exist.
    #[error("document not found")]
    NotFound,
    /// A model could not be serialized or decoded.
    #[error("serialization failed: {0}")]
    Serialization(String),
    /// A backend driver returned an error.
    #[error("{backend} operation failed: {message}")]
    Driver {
        /// Backend that returned the error.
        backend: &'static str,
        /// Error text with secrets excluded by the adapter.
        message: String,
    },
    /// A response exceeded the adapter's configured memory bound.
    #[error("{backend} response exceeded the {limit_bytes}-byte limit")]
    ResponseTooLarge {
        /// Backend that returned the response.
        backend: &'static str,
        /// Maximum accepted response size.
        limit_bytes: usize,
    },
    /// The backend cannot represent a value through the portable API.
    #[error("{backend} value type is unsupported: {kind}")]
    UnsupportedValue {
        /// Backend that produced the value.
        backend: &'static str,
        /// Backend type name.
        kind: String,
    },
    /// Work delegated to a blocking worker could not complete.
    #[error("{backend} worker failed: {message}")]
    Worker {
        /// Backend whose worker stopped.
        backend: &'static str,
        /// Join or lock failure detail.
        message: String,
    },
}

impl PolyglotError {
    #[cfg(any(
        feature = "duckdb",
        feature = "mongodb",
        feature = "surrealdb",
        feature = "turso"
    ))]
    pub(crate) fn driver(backend: &'static str, error: impl fmt::Display) -> Self {
        Self::Driver {
            backend,
            message: error.to_string(),
        }
    }

    pub(crate) fn serialization(error: impl fmt::Display) -> Self {
        Self::Serialization(error.to_string())
    }

    /// Converts a redacted model-codec failure into the public error contract.
    #[doc(hidden)]
    pub fn serialization_public(error: impl fmt::Display) -> Self {
        Self::serialization(error)
    }
}
