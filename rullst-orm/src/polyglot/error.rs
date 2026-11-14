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
}

impl PolyglotError {
    #[cfg(any(feature = "surrealdb", feature = "turso"))]
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
    // Used by `#[derive(TursoModel)]`.
    #[doc(hidden)]
    pub fn serialization_public(error: impl fmt::Display) -> Self {
        Self::serialization(error)
    }
}

/// Most characters of server-provided text a driver error keeps.
#[cfg(any(feature = "surrealdb", feature = "redis"))]
const MAX_SERVER_DETAIL_CHARS: usize = 240;

/// A server's own error text made safe to carry in [`PolyglotError::Driver`]:
/// one line without control characters, every URL (which can embed
/// credentials) replaced, and at most 240 characters. `None` when no text is
/// left.
#[cfg(any(feature = "surrealdb", feature = "redis"))]
pub(crate) fn server_detail(text: &str) -> Option<String> {
    let words = text
        .split(|character: char| character.is_whitespace() || character.is_control())
        .filter(|word| !word.is_empty())
        .map(|word| {
            if word.contains("://") {
                "[redacted URL]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    let detail = words.join(" ");
    if detail.chars().count() <= MAX_SERVER_DETAIL_CHARS {
        return Some(detail);
    }
    let mut bounded = detail
        .chars()
        .take(MAX_SERVER_DETAIL_CHARS - 1)
        .collect::<String>();
    bounded.push('…');
    Some(bounded)
}

#[cfg(all(test, any(feature = "surrealdb", feature = "redis")))]
mod tests {
    use super::server_detail;

    #[test]
    fn server_detail_is_one_bounded_line_without_urls() {
        assert_eq!(server_detail(""), None);
        assert_eq!(server_detail(" \n\t\u{1b} "), None);
        assert_eq!(
            server_detail("  The database 'app'\n  does not exist\r\n"),
            Some("The database 'app' does not exist".to_owned())
        );
        // A control character that is not whitespace also splits words.
        assert_eq!(
            server_detail("bad\u{1b}[31mcolour"),
            Some("bad [31mcolour".to_owned())
        );
        assert_eq!(
            server_detail("cannot reach https://user:secret@db.example:8000/sql now"),
            Some("cannot reach [redacted URL] now".to_owned())
        );
        let exact = "é".repeat(240);
        assert_eq!(server_detail(&exact), Some(exact.clone()));
        let long = server_detail(&format!("{exact}x")).unwrap();
        assert_eq!(long.chars().count(), 240);
        assert_eq!(long, format!("{}…", "é".repeat(239)));
    }
}
