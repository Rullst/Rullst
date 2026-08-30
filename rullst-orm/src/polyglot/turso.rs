use std::{collections::BTreeMap, fmt, net::IpAddr, path::PathBuf};

use super::{Backend, BackendCapabilities, Capability, PolyglotError};

const MAX_QUERY_ROWS: u32 = 10_000;
const MAX_STATEMENT_BYTES: usize = 1024 * 1024;
const MAX_PARAMETERS: usize = 1_024;
const MAX_PARAMETER_BYTES: usize = 8 * 1024 * 1024;
const MAX_TRANSACTION_STATEMENTS: usize = 1_024;
const MAX_TRANSACTION_BYTES: usize = 16 * 1024 * 1024;

mod codec;
use codec::{execute_sqlite, query_sqlite, transaction_sqlite};
mod hrana;
use hrana::HranaClient;
mod migration;
pub use migration::{TursoMigration, TursoMigrationReport, TursoRollbackReport};
mod model;
pub use model::{TursoCodec, TursoModel, TursoPrimaryKey};
mod primary;
pub use primary::{TursoActiveRecord, TursoOrm};
mod repository;
pub use repository::{TursoOrder, TursoQuery, TursoRepository};

/// Scalar values supported by the Turso/libSQL SQL boundary.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TursoValue {
    /// SQL `NULL`.
    Null,
    /// Signed 64-bit integer.
    Integer(i64),
    /// Double-precision floating point value.
    Real(f64),
    /// UTF-8 text.
    Text(String),
    /// Binary data.
    Blob(Vec<u8>),
}

/// A parameterized statement for the Turso/libSQL adapter.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TursoStatement {
    sql: String,
    parameters: Vec<TursoValue>,
}

impl TursoStatement {
    /// Creates a non-empty statement with a one-MiB structural SQL bound.
    pub fn new(sql: impl Into<String>, parameters: Vec<TursoValue>) -> Result<Self, PolyglotError> {
        let sql = sql.into();
        if sql.trim().is_empty() || sql.len() > MAX_STATEMENT_BYTES {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso SQL statement",
                reason: "SQL must contain between 1 byte and 1 MiB",
            });
        }
        if parameters.len() > MAX_PARAMETERS {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso SQL parameters",
                reason: "a statement accepts at most 1024 parameters",
            });
        }
        let parameter_bytes = parameters.iter().try_fold(0usize, |total, value| {
            total.checked_add(match value {
                TursoValue::Null => 0,
                TursoValue::Integer(_) | TursoValue::Real(_) => 8,
                TursoValue::Text(value) => value.len(),
                TursoValue::Blob(value) => value.len(),
            })
        });
        if parameter_bytes.is_none_or(|bytes| bytes > MAX_PARAMETER_BYTES) {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso SQL parameters",
                reason: "parameter payload must not exceed 8 MiB",
            });
        }
        Ok(Self { sql, parameters })
    }

    fn payload_bytes(&self) -> Option<usize> {
        self.parameters
            .iter()
            .try_fold(self.sql.len(), |total, value| {
                total.checked_add(match value {
                    TursoValue::Null => 0,
                    TursoValue::Integer(_) | TursoValue::Real(_) => 8,
                    TursoValue::Text(value) => value.len(),
                    TursoValue::Blob(value) => value.len(),
                })
            })
    }
}

/// One Turso/libSQL result row keyed by column name.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TursoRow {
    columns: BTreeMap<String, TursoValue>,
}

impl TursoRow {
    /// Returns a value by its result column name.
    pub fn get(&self, name: &str) -> Option<&TursoValue> {
        self.columns.get(name)
    }

    /// Consumes the row and returns all columns.
    pub fn into_columns(self) -> BTreeMap<String, TursoValue> {
        self.columns
    }
}

/// A mandatory bound for materialized Turso/libSQL query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TursoQueryLimit(u32);

impl TursoQueryLimit {
    /// Creates a result limit between 1 and 10,000 rows.
    pub fn new(limit: u32) -> Result<Self, PolyglotError> {
        if limit == 0 || limit > MAX_QUERY_ROWS {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso query limit",
                reason: "limit must be between 1 and 10000",
            });
        }
        Ok(Self(limit))
    }

    /// Returns the validated row limit.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Safe Turso endpoint and credential configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct TursoConfig {
    url: String,
    auth_token: String,
    allow_insecure_loopback: bool,
    offline_path: Option<PathBuf>,
}

impl TursoConfig {
    /// Creates a remote or deterministic mock configuration.
    ///
    /// Empty and `mock_*` URLs select the SQLite-compatible in-memory fallback.
    pub fn new(url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            url: url.into().trim().to_owned(),
            auth_token: auth_token.into(),
            allow_insecure_loopback: false,
            offline_path: None,
        }
    }

    /// Persists the deterministic SQLite-compatible fallback at `path`.
    ///
    /// This setting is used only when the URL is empty or begins with
    /// `mock_`/`mock://`; remote Turso connections ignore it.
    pub fn with_offline_path(mut self, path: impl Into<PathBuf>) -> Result<Self, PolyglotError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso offline",
                reason: "offline database path must not be empty",
            });
        }
        self.offline_path = Some(path);
        Ok(self)
    }

    /// Allows plain HTTP only for a loopback test or development server.
    pub fn allow_insecure_loopback(mut self) -> Self {
        self.allow_insecure_loopback = true;
        self
    }

    fn is_mock(&self) -> bool {
        self.url.is_empty() || self.url.starts_with("mock_") || self.url.starts_with("mock://")
    }

    fn validate(&self) -> Result<(), PolyglotError> {
        if self.is_mock() {
            return Ok(());
        }
        let parsed =
            url::Url::parse(&self.url).map_err(|_| PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "endpoint is not a valid URL",
            })?;
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "endpoint must not contain credentials, query parameters, or a fragment",
            });
        }
        if parsed.host_str().is_none() {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "endpoint must contain a host",
            });
        }
        let secure = matches!(parsed.scheme(), "libsql" | "https");
        let loopback = parsed.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .parse::<IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
        let permitted_loopback =
            parsed.scheme() == "http" && loopback && self.allow_insecure_loopback;
        if !secure && !permitted_loopback {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "use libsql:// or HTTPS; plain HTTP is limited to explicitly enabled loopback",
            });
        }
        if self.auth_token.trim().is_empty() && !loopback {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "a remote authentication token is required",
            });
        }
        Ok(())
    }
}

impl fmt::Debug for TursoConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TursoConfig")
            .field("url", &"[CONFIGURED]")
            .field("auth_token", &"[REDACTED]")
            .field("allow_insecure_loopback", &self.allow_insecure_loopback)
            .field(
                "offline_path",
                &self.offline_path.as_ref().map(|_| "[CONFIGURED]"),
            )
            .finish()
    }
}

enum TursoInner {
    Remote(HranaClient),
    Offline(sqlx::SqlitePool),
}

/// Turso/libSQL remote adapter with a deterministic SQLite-compatible fallback.
pub struct TursoStore {
    inner: TursoInner,
}

impl TursoStore {
    /// Connects to Turso/libSQL or selects the offline fallback for mock credentials.
    pub async fn connect(config: TursoConfig) -> Result<Self, PolyglotError> {
        config.validate()?;
        if config.is_mock() {
            let options = match config.offline_path {
                Some(path) => sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
                None => sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(":memory:")
                    .in_memory(true),
            };
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .map_err(|error| PolyglotError::driver("Turso offline", error))?;
            return Ok(Self {
                inner: TursoInner::Offline(pool),
            });
        }

        let client = HranaClient::new(&config.url, config.auth_token)?;
        Ok(Self {
            inner: TursoInner::Remote(client),
        })
    }

    /// Reports whether the deterministic offline backend was selected.
    pub fn is_offline(&self) -> bool {
        matches!(&self.inner, TursoInner::Offline(_))
    }

    /// Declares the adapter's portable behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(
            Backend::Turso,
            &[Capability::EdgeSql, Capability::RelationalModels],
        )
    }

    /// Executes one parameterized statement and returns affected rows.
    pub async fn execute(&self, statement: TursoStatement) -> Result<u64, PolyglotError> {
        match &self.inner {
            TursoInner::Remote(client) => client.execute(statement).await,
            TursoInner::Offline(pool) => execute_sqlite(pool, statement).await,
        }
    }

    /// Executes all statements atomically and returns each affected-row count.
    pub async fn transaction(
        &self,
        statements: Vec<TursoStatement>,
    ) -> Result<Vec<u64>, PolyglotError> {
        if statements.is_empty() {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso transaction",
                reason: "at least one statement is required",
            });
        }
        if statements.len() > MAX_TRANSACTION_STATEMENTS {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso transaction",
                reason: "a transaction accepts at most 1024 statements",
            });
        }
        let payload_bytes = statements.iter().try_fold(0usize, |total, statement| {
            total.checked_add(statement.payload_bytes()?)
        });
        if payload_bytes.is_none_or(|bytes| bytes > MAX_TRANSACTION_BYTES) {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso transaction",
                reason: "transaction SQL and parameters must not exceed 16 MiB",
            });
        }
        match &self.inner {
            TursoInner::Remote(client) => client.transaction(statements).await,
            TursoInner::Offline(pool) => transaction_sqlite(pool, statements).await,
        }
    }

    /// Executes a parameterized query with mandatory result materialization bound.
    pub async fn query(
        &self,
        statement: TursoStatement,
        limit: TursoQueryLimit,
    ) -> Result<Vec<TursoRow>, PolyglotError> {
        match &self.inner {
            TursoInner::Remote(client) => client.query(statement, limit).await,
            TursoInner::Offline(pool) => query_sqlite(pool, statement, limit).await,
        }
    }
}

#[cfg(test)]
#[path = "turso/tests.rs"]
mod tests;
