use std::error::Error;
use std::fmt;

/// The standard error type for rullst-orm, shielding users from internal dependency errors.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RullstError {
    /// The global ORM state has not been initialized yet.
    NotInitialized,
    /// The global ORM state was already initialized by an earlier call.
    AlreadyInitialized,
    /// A record was not found in the database.
    RecordNotFound,
    /// A general database or query error.
    DatabaseError(String),
    /// A unique or primary-key constraint rejected the statement, for example
    /// a second user with the same email. Match it as
    /// `UniqueViolation { .. }`: later versions may add fields. Added in v13.
    #[non_exhaustive]
    UniqueViolation {
        /// The violated constraint or index, when the database names it:
        /// PostgreSQL does, SQLite and MySQL/MariaDB do not.
        constraint: Option<String>,
    },
    /// A foreign-key constraint rejected the statement: the referenced row
    /// does not exist, or a row still references the one being deleted.
    /// Match it as `ForeignKeyViolation { .. }`. Added in v13.
    #[non_exhaustive]
    ForeignKeyViolation {
        /// The violated constraint, when the database names it: PostgreSQL
        /// does, SQLite and MySQL/MariaDB do not.
        constraint: Option<String>,
    },
    /// The connected database cannot make this schema change, for example
    /// adding a foreign key to an existing SQLite table. `Schema::table`
    /// returns it before running any statement. Match it as
    /// `UnsupportedSchemaChange { .. }`. Added in v13.
    #[non_exhaustive]
    UnsupportedSchemaChange {
        /// The SQL dialect: `sqlite`, `postgres` or `mysql`.
        driver: String,
        /// The change that cannot be made, and the usual alternative.
        change: String,
    },
    /// A serialization or deserialization error (e.g., JSON).
    SerializationError(String),
    /// A cache or event-related error.
    CacheError(String),
    /// A database commit succeeded, but one or more post-commit effects failed.
    PostCommit(String),
    /// A validation error, such as invalid SQL identifiers.
    Validation(String),
    /// Other internal errors.
    Internal(String),
}

impl fmt::Display for RullstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RullstError::NotInitialized => {
                write!(
                    f,
                    "Orm is not initialized. Call Orm::init() before querying."
                )
            }
            RullstError::AlreadyInitialized => write!(f, "Orm has already been initialized"),
            RullstError::RecordNotFound => write!(f, "Record not found"),
            RullstError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            RullstError::UniqueViolation { constraint } => {
                write_violation(f, "unique", constraint.as_deref())
            }
            RullstError::ForeignKeyViolation { constraint } => {
                write_violation(f, "foreign key", constraint.as_deref())
            }
            RullstError::UnsupportedSchemaChange { driver, change } => {
                write!(f, "Unsupported schema change on {driver}: {change}")
            }
            RullstError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            RullstError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            RullstError::PostCommit(msg) => {
                write!(
                    f,
                    "Database committed, but a post-commit effect failed: {}",
                    msg
                )
            }
            RullstError::Validation(msg) => write!(f, "Validation error: {}", msg),
            RullstError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Names the constraint only: the database's own message can quote the
/// rejected values (MySQL's "Duplicate entry 'alice@example.com'").
fn write_violation(
    f: &mut fmt::Formatter<'_>,
    kind: &str,
    constraint: Option<&str>,
) -> fmt::Result {
    match constraint {
        Some(name) => write!(f, "Database error: {kind} constraint `{name}` violated"),
        None => write!(f, "Database error: {kind} constraint violated"),
    }
}

impl Error for RullstError {}

impl From<sqlx::Error> for RullstError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => RullstError::RecordNotFound,
            sqlx::Error::Database(ref error) if error.is_unique_violation() => {
                RullstError::UniqueViolation {
                    constraint: error.constraint().map(str::to_owned),
                }
            }
            sqlx::Error::Database(ref error) if error.is_foreign_key_violation() => {
                RullstError::ForeignKeyViolation {
                    constraint: error.constraint().map(str::to_owned),
                }
            }
            _ => RullstError::DatabaseError(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for RullstError {
    fn from(err: serde_json::Error) -> Self {
        RullstError::SerializationError(err.to_string())
    }
}

impl From<crate::privacy::PrivacyError> for RullstError {
    fn from(err: crate::privacy::PrivacyError) -> Self {
        RullstError::Internal(err.to_string())
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
        assert_eq!(
            RullstError::NotInitialized.to_string(),
            "Orm is not initialized. Call Orm::init() before querying."
        );
        assert_eq!(
            RullstError::AlreadyInitialized.to_string(),
            "Orm has already been initialized"
        );
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
            RullstError::PostCommit("msg".to_string()).to_string(),
            "Database committed, but a post-commit effect failed: msg"
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
    fn constraint_violations_name_only_their_constraint() {
        let named = RullstError::UniqueViolation {
            constraint: Some("users_email_key".to_string()),
        };
        assert_eq!(
            named.to_string(),
            "Database error: unique constraint `users_email_key` violated"
        );
        let unnamed = RullstError::UniqueViolation { constraint: None };
        assert_eq!(
            unnamed.to_string(),
            "Database error: unique constraint violated"
        );
        let foreign = RullstError::ForeignKeyViolation {
            constraint: Some("posts_user_id_fkey".to_string()),
        };
        assert_eq!(
            foreign.to_string(),
            "Database error: foreign key constraint `posts_user_id_fkey` violated"
        );
        let unnamed = RullstError::ForeignKeyViolation { constraint: None };
        assert_eq!(
            unnamed.to_string(),
            "Database error: foreign key constraint violated"
        );
    }

    #[test]
    fn unsupported_schema_changes_name_the_driver_and_change() {
        let error = RullstError::UnsupportedSchemaChange {
            driver: "sqlite".to_string(),
            change: "adding a foreign key to an existing table".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Unsupported schema change on sqlite: adding a foreign key to an existing table"
        );
    }

    /// A database error that names its constraint, as PostgreSQL's does.
    #[derive(Debug)]
    struct NamedConstraintError {
        unique: bool,
    }

    impl fmt::Display for NamedConstraintError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Key (email)=(alice@example.test) already exists.")
        }
    }

    impl Error for NamedConstraintError {}

    impl sqlx::error::DatabaseError for NamedConstraintError {
        fn message(&self) -> &str {
            "Key (email)=(alice@example.test) already exists."
        }

        fn as_error(&self) -> &(dyn Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn Error + Send + Sync + 'static> {
            self
        }

        fn constraint(&self) -> Option<&str> {
            Some(if self.unique {
                "users_email_key"
            } else {
                "posts_user_id_fkey"
            })
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            if self.unique {
                sqlx::error::ErrorKind::UniqueViolation
            } else {
                sqlx::error::ErrorKind::ForeignKeyViolation
            }
        }
    }

    #[test]
    fn database_errors_keep_the_constraint_name_but_not_the_message() {
        let unique: RullstError =
            sqlx::Error::Database(Box::new(NamedConstraintError { unique: true })).into();
        assert!(
            matches!(&unique, RullstError::UniqueViolation { constraint: Some(name) } if name == "users_email_key"),
            "{unique:?}"
        );
        let foreign: RullstError =
            sqlx::Error::Database(Box::new(NamedConstraintError { unique: false })).into();
        assert!(
            matches!(&foreign, RullstError::ForeignKeyViolation { constraint: Some(name) } if name == "posts_user_id_fkey"),
            "{foreign:?}"
        );
        assert!(!format!("{unique} {unique:?} {foreign}").contains("alice@example.test"));
    }

    #[cfg(any(feature = "drivers-all", feature = "strict-sqlite"))]
    #[tokio::test]
    async fn sqlite_constraint_errors_become_typed_variants() {
        use sqlx::Connection;

        // SQLx enables SQLite foreign keys on every connection it opens.
        let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE);
             CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id));
             INSERT INTO users (id, email) VALUES (1, 'alice@example.test');",
        )
        .execute(&mut connection)
        .await
        .unwrap();

        let duplicate = sqlx::query("INSERT INTO users (email) VALUES ('alice@example.test')")
            .execute(&mut connection)
            .await
            .unwrap_err();
        let duplicate = RullstError::from(duplicate);
        assert!(
            matches!(duplicate, RullstError::UniqueViolation { constraint: None }),
            "{duplicate:?}"
        );
        assert_eq!(
            duplicate.to_string(),
            "Database error: unique constraint violated"
        );

        let orphan = sqlx::query("INSERT INTO posts (user_id) VALUES (99)")
            .execute(&mut connection)
            .await
            .unwrap_err();
        let orphan = RullstError::from(orphan);
        assert!(
            matches!(
                orphan,
                RullstError::ForeignKeyViolation { constraint: None }
            ),
            "{orphan:?}"
        );

        let missing = sqlx::query("SELECT * FROM missing_table")
            .execute(&mut connection)
            .await
            .unwrap_err();
        let missing = RullstError::from(missing);
        assert!(
            matches!(&missing, RullstError::DatabaseError(message) if message.contains("no such table: missing_table")),
            "{missing:?}"
        );
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
