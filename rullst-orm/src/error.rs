use std::error::Error;
use std::fmt;

/// The standard error type for rullst-orm, shielding users from internal dependency errors.
#[derive(Debug, Clone)]
pub enum RullstError {
    /// A record was not found in the database.
    RecordNotFound,
    /// A general database or query error.
    DatabaseError(String),
    /// A serialization or deserialization error (e.g., JSON).
    SerializationError(String),
    /// A cache or event-related error.
    CacheError(String),
    /// A validation error, such as invalid SQL identifiers.
    Validation(String),
    /// Other internal errors.
    Internal(String),
}

impl fmt::Display for RullstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RullstError::RecordNotFound => write!(f, "Record not found"),
            RullstError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            RullstError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            RullstError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            RullstError::Validation(msg) => write!(f, "Validation error: {}", msg),
            RullstError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl Error for RullstError {}

impl From<sqlx::Error> for RullstError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => RullstError::RecordNotFound,
            _ => RullstError::DatabaseError(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for RullstError {
    fn from(err: serde_json::Error) -> Self {
        RullstError::SerializationError(err.to_string())
    }
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for RullstError {
    #[cfg_attr(test, mutants::skip)]
    fn from(err: redis::RedisError) -> Self {
        RullstError::CacheError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rullst_error_display() {
        assert_eq!(RullstError::RecordNotFound.to_string(), "Record not found");
        assert_eq!(
            RullstError::DatabaseError("msg".to_string()).to_string(),
            "Database error: msg"
        );
        assert_eq!(
            RullstError::SerializationError("msg".to_string()).to_string(),
            "Serialization error: msg"
        );
        assert_eq!(
            RullstError::Validation("msg".to_string()).to_string(),
            "Validation error: msg"
        );
        assert_eq!(
            RullstError::Internal("msg".to_string()).to_string(),
            "Internal error: msg"
        );
    }

    #[test]
    fn test_rullst_error_from() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let err: RullstError = sqlx_err.into();
        assert!(matches!(err, RullstError::RecordNotFound));

        let sqlx_other_err = sqlx::Error::PoolClosed;
        let err2: RullstError = sqlx_other_err.into();
        assert!(matches!(err2, RullstError::DatabaseError(_)));
    }

    #[test]
    fn test_serde_json_error_from() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err: RullstError = json_err.into();
        assert!(matches!(err, RullstError::SerializationError(_)));
        assert!(err.to_string().contains("Serialization error:"));
    }

    #[cfg(feature = "redis")]
    #[test]
    fn test_redis_error_from() {
        let redis_err = redis::RedisError::from((redis::ErrorKind::Io, "redis io error"));
        let err: RullstError = redis_err.into();
        assert!(matches!(err, RullstError::CacheError(_)));
        assert_eq!(err.to_string(), "Cache error: redis io error - Io");
    }
}
