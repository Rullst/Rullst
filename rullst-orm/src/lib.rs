pub use sqlx;

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
pub use sqlx::AnyPool as RullstPool;

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
pub use sqlx::any::AnyPoolOptions as RullstPoolOptions;

#[cfg(feature = "strict-postgres")]
pub use sqlx::PgPool as RullstPool;

#[cfg(feature = "strict-postgres")]
pub use sqlx::postgres::PgPoolOptions as RullstPoolOptions;

#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub use sqlx::MySqlPool as RullstPool;

#[cfg(all(feature = "strict-mysql", not(feature = "strict-postgres")))]
pub use sqlx::mysql::MySqlPoolOptions as RullstPoolOptions;

#[cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
pub use sqlx::SqlitePool as RullstPool;

#[cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
pub use sqlx::sqlite::SqlitePoolOptions as RullstPoolOptions;

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
use sqlx::any::install_default_drivers;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

// Hide underlying libraries for macro usage while keeping the public API clean
#[doc(hidden)]
pub use async_stream as _async_stream;
#[doc(hidden)]
pub use futures as _futures;
#[doc(hidden)]
pub use serde as _serde;
#[doc(hidden)]
pub use serde_json as _serde_json;
#[doc(hidden)]
pub use sqlx as _sqlx;
#[doc(hidden)]
pub use tracing as _tracing;

#[cfg(feature = "redis")]
#[doc(hidden)]
pub use redis as _redis;

/// Helper to convert `?` placeholders to `$1`, `$2` etc. for Postgres.
#[doc(hidden)]
pub fn replace_placeholders(sql: &str) -> String {
    let mut replaced = String::with_capacity(sql.len() + 10);
    let mut last_idx = 0;
    for (counter, (idx, _)) in (1..).zip(sql.match_indices('?')) {
        replaced.push_str(&sql[last_idx..idx]);
        use std::fmt::Write;
        let _ = write!(replaced, "${}", counter);
        last_idx = idx + 1;
    }
    replaced.push_str(&sql[last_idx..]);
    replaced
}

static PREVENT_LAZY_LOADING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Prevents relationships from being lazily loaded when accessed without being eager loaded.
/// When enabled, attempting to lazily load a relation will throw a panic in development.
pub fn prevent_lazy_loading(prevent: bool) {
    PREVENT_LAZY_LOADING.store(prevent, std::sync::atomic::Ordering::Relaxed);
}

#[doc(hidden)]
pub fn is_lazy_loading_prevented() -> bool {
    PREVENT_LAZY_LOADING.load(std::sync::atomic::Ordering::Relaxed)
}

pub mod admin;
pub mod audit;
pub mod collection;
pub mod database;
pub mod db;
pub mod policy;

tokio::task_local! {
    pub static CURRENT_TX: std::sync::Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>;
}

#[macro_export]
macro_rules! execute_query {
    ($query:expr, $method:ident, $pool_fn:ident) => {
        if let Ok(tx_arc) = $crate::CURRENT_TX.try_with(|tx| tx.clone()) {
            let mut tx_guard = tx_arc.lock().await;
            if let Some(tx) = tx_guard.as_mut() {
                $query.$method(&mut **tx).await
            } else {
                let pool = $crate::Orm::$pool_fn();
                $query.$method(pool).await
            }
        } else {
            let pool = $crate::Orm::$pool_fn();
            $query.$method(pool).await
        }
    };
}

#[macro_export]
macro_rules! dispatch_executor {
    ($pool_fn:ident, |$executor:ident| $e:expr) => {
        if let Ok(tx_arc) = $crate::CURRENT_TX.try_with(|tx| tx.clone()) {
            let mut tx_guard = tx_arc.lock().await;
            if let Some(tx) = tx_guard.as_mut() {
                let $executor = &mut **tx;
                $e
            } else {
                let $executor = $crate::Orm::$pool_fn();
                $e
            }
        } else {
            let $executor = $crate::Orm::$pool_fn();
            $e
        }
    };
}

pub mod error;
pub mod privacy;
pub mod raw;
pub mod resource;
pub mod schema;
pub mod schema_diff;
pub mod scout;
pub mod tenant;
pub mod types;

// Export the custom Error enum to the root
pub use error::RullstError as Error;

// Re-exports
pub use _sqlx::FromRow;
pub use admin::dashboard_html;
pub use collection::RullstCollection;
pub use database::RullstDatabase;
pub use policy::Policy;
pub use privacy::{ComplianceModel, PrivacyReport, SecretString};
pub use resource::{ApiResource, JsonResource, ResourceCollection};
pub use rullst_orm_macros::{Enum, Orm, PersonalData, test};
pub use scout::{SearchEngine, get_search_engine, set_search_engine};
pub use tenant::{get_tenant_id, with_tenant};
pub use types::Json;

// Re-export async_trait so the macro can use it implicitly
pub use async_trait::async_trait;

// Re-export sqlx and FromRow for database mapping
pub use schema::{JoinClause, SubqueryBuilder};

/// Trait implementada automaticamente pelas macros para os modelos que usam `#[orm(rag_context)]`
pub trait RagContext {
    fn get_context(&self) -> String;
}

/// The global connection pool
static DB_POOL: OnceLock<RullstPool> = OnceLock::new();

/// The driver identifier (postgres, mysql, sqlite) to help macro syntax formatting
static DB_DRIVER: OnceLock<String> = OnceLock::new();

/// The replica connection pools for read operations
static REPLICA_POOLS: OnceLock<Vec<RullstPool>> = OnceLock::new();

/// Atomic index for replica round-robin selection
static REPLICA_INDEX: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "redis")]
static REDIS_CLIENT: OnceLock<_redis::Client> = OnceLock::new();

#[cfg(feature = "redis")]
static REDIS_MANAGER: OnceLock<_redis::aio::ConnectionManager> = OnceLock::new();

/// Enum dinÃ¢mico para encapsular qualquer tipo que possa ser associado ao banco de dados pelo Macro
#[derive(Clone, Debug)]
pub enum RullstValue {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
}

impl From<&str> for RullstValue {
    fn from(s: &str) -> Self {
        RullstValue::String(s.to_string())
    }
}
impl From<String> for RullstValue {
    fn from(s: String) -> Self {
        RullstValue::String(s)
    }
}
impl From<i32> for RullstValue {
    fn from(i: i32) -> Self {
        RullstValue::Int(i)
    }
}
impl From<f64> for RullstValue {
    fn from(f: f64) -> Self {
        RullstValue::Float(f)
    }
}
impl From<bool> for RullstValue {
    fn from(b: bool) -> Self {
        RullstValue::Bool(b)
    }
}

impl TryFrom<RullstValue> for String {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::String(s) => Ok(s),
            _ => Err("Not a string"),
        }
    }
}
impl TryFrom<RullstValue> for i32 {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Int(i) => Ok(i),
            _ => Err("Not an i32"),
        }
    }
}
impl TryFrom<RullstValue> for f64 {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Float(f) => Ok(f),
            _ => Err("Not an f64"),
        }
    }
}
impl TryFrom<RullstValue> for bool {
    type Error = &'static str;
    fn try_from(val: RullstValue) -> Result<Self, Self::Error> {
        match val {
            RullstValue::Bool(b) => Ok(b),
            _ => Err("Not a bool"),
        }
    }
}

/// Orm configuration structure
pub struct Orm;

impl Orm {
    /// Initialize the global database connection pool using an agnostic URI
    pub async fn init(database_url: &str) -> Result<(), crate::Error> {
        // Reject unconfigured placeholder URLs before they reach the driver
        // (e.g. Turso template `libsql://[your-database-id].turso.io` whose
        //  brackets are misinterpreted as an IPv6 literal by URL parsers).
        if database_url.contains('[') && database_url.contains(']') {
            return Err(crate::Error::Internal(
                "DATABASE_URL contains placeholder brackets like [your-database-id]. \
                Update your .env file with a real connection string."
                    .to_string(),
            ));
        }

        Self::validate_dsn(database_url);

        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = RullstPool::connect(database_url).await?;

        if DB_POOL.set(pool).is_err() {
            return Err(crate::Error::Internal(
                "Orm has already been initialized".to_string(),
            ));
        }

        let driver = if database_url.starts_with("postgres") {
            "postgres"
        } else if database_url.starts_with("mysql") {
            "mysql"
        } else {
            "sqlite"
        };

        let _ = DB_DRIVER.set(driver.to_string());
        let _ = REPLICA_POOLS.set(vec![]);

        Ok(())
    }

    /// Initialize the global database connection pool with specific pool options
    pub async fn init_with_options(
        database_url: &str,
        max_connections: u32,
        acquire_timeout_secs: u64,
    ) -> Result<(), crate::Error> {
        Self::validate_dsn(database_url);

        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = RullstPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(std::time::Duration::from_secs(acquire_timeout_secs))
            .connect(database_url)
            .await?;

        if DB_POOL.set(pool).is_err() {
            return Err(crate::Error::Internal(
                "Orm has already been initialized".to_string(),
            ));
        }

        let driver = if database_url.starts_with("postgres") {
            "postgres"
        } else if database_url.starts_with("mysql") {
            "mysql"
        } else {
            "sqlite"
        };

        let _ = DB_DRIVER.set(driver.to_string());
        let _ = REPLICA_POOLS.set(vec![]);

        Ok(())
    }

    #[cfg_attr(test, mutants::skip)]
    fn validate_dsn(database_url: &str) {
        // Detect unconfigured placeholder URLs (e.g. the Turso template uses
        // [your-database-id] which the URL parser misreads as an IPv6 address).
        if database_url.contains('[') && database_url.contains(']') {
            eprintln!(
                "⚠️ [CONFIG] Rullst ORM: DATABASE_URL still contains placeholder brackets (e.g. [your-database-id]). \
                Update your .env file with a real connection string before using database features."
            );
            return;
        }

        if database_url.starts_with("sqlite") {
            let mut path_part = database_url
                .trim_start_matches("sqlite:")
                .trim_start_matches("//")
                .trim_start_matches("file:");
            if let Some(idx) = path_part.find('?') {
                path_part = &path_part[..idx];
            }
            if !path_part.is_empty()
                && path_part != ":memory:"
                && !path_part.contains("mode=memory")
            {
                let path = std::path::Path::new(path_part);
                // Ensure the parent directory exists
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                {
                    let _ = std::fs::create_dir_all(parent);
                }
                // Touch-create the SQLite file if it doesn't exist so that
                // drivers without implicit `mode=rwc` support (or without the
                // query-parameter) never hit SQLITE_CANTOPEN (error code 14).
                if !path.exists() {
                    let _ = std::fs::File::create(path);
                }
            }
        }

        if database_url.contains("sslmode=disable")
            && !database_url.contains("localhost")
            && !database_url.contains("127.0.0.1")
        {
            eprintln!(
                "⚠️ [SECURITY WARNING] Rullst ORM: TLS/SSL disabled on external database connection! This is highly discouraged in production environments."
            );
        }
    }

    /// Initialize the global database connection pool and its read replicas
    pub async fn init_with_replicas(
        primary_url: &str,
        replica_urls: Vec<&str>,
    ) -> Result<(), crate::Error> {
        Self::validate_dsn(primary_url);
        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = RullstPool::connect(primary_url).await?;

        if DB_POOL.set(pool).is_err() {
            return Err(crate::Error::Internal(
                "Orm has already been initialized".to_string(),
            ));
        }

        let driver = if primary_url.starts_with("postgres") {
            "postgres"
        } else if primary_url.starts_with("mysql") {
            "mysql"
        } else {
            "sqlite"
        };

        let _ = DB_DRIVER.set(driver.to_string());

        // Initialize all replica pools concurrently — each connect() is independent I/O.
        let replica_futures: Vec<_> = replica_urls.into_iter().map(RullstPool::connect).collect();
        let replicas = futures::future::try_join_all(replica_futures).await?;
        let _ = REPLICA_POOLS.set(replicas);

        Ok(())
    }

    /// Retrieve the global database connection pool (strictly for writes)
    #[allow(clippy::expect_used)]
    pub fn pool() -> &'static RullstPool {
        DB_POOL
            .get()
            .expect("Orm must be initialized before querying")
    }

    /// Fallible variant of [`Orm::pool`]: returns `Err` instead of panicking
    /// when the ORM has not been initialized yet.
    pub fn try_pool() -> Result<&'static RullstPool, crate::Error> {
        DB_POOL.get().ok_or_else(|| {
            crate::Error::Internal(
                "Orm is not initialized. Call Orm::init() before querying.".to_string(),
            )
        })
    }

    /// Retrieve the connection pool for read operations.
    /// Performs a round-robin load balancing over replicas if configured.
    #[cfg_attr(test, mutants::skip)]
    pub fn read_pool() -> &'static RullstPool {
        if let Some(replicas) = REPLICA_POOLS.get()
            && !replicas.is_empty()
        {
            let idx = REPLICA_INDEX.fetch_add(1, Ordering::Relaxed) % replicas.len();
            return &replicas[idx];
        }
        Self::pool()
    }

    /// Fallible variant of [`Orm::read_pool`].
    #[cfg_attr(test, mutants::skip)]
    pub fn try_read_pool() -> Result<&'static RullstPool, crate::Error> {
        if let Some(replicas) = REPLICA_POOLS.get()
            && !replicas.is_empty()
        {
            let idx = REPLICA_INDEX.fetch_add(1, Ordering::Relaxed) % replicas.len();
            return Ok(&replicas[idx]);
        }
        Self::try_pool()
    }

    /// Retrieve the active driver string (defaults to "sqlite" if DB is not initialized yet)
    pub fn driver() -> &'static str {
        DB_DRIVER.get().map(|s| s.as_str()).unwrap_or("sqlite")
    }

    /// Fallible variant of [`Orm::driver`].
    pub fn try_driver() -> Result<&'static str, crate::Error> {
        DB_DRIVER.get().map(|s| s.as_str()).ok_or_else(|| {
            crate::Error::Internal(
                "Orm is not initialized. Call Orm::init() before querying.".to_string(),
            )
        })
    }

    /// Create a raw SQL query builder.
    pub fn raw(sql: &str) -> raw::RawQueryBuilder {
        raw::RawQueryBuilder::new(sql)
    }

    pub async fn begin_transaction() -> Result<crate::db::Transaction<'static>, crate::Error> {
        let pool = Self::pool();
        pool.begin().await.map_err(Into::into)
    }

    /// Executes a closure inside an isolated database transaction, automatically committing on Ok and rolling back on Err.
    pub async fn transaction<F, R, E>(f: F) -> Result<R, crate::Error>
    where
        F: FnOnce(
                std::sync::Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, E>> + Send>>
            + Send,
        E: std::fmt::Display,
    {
        let tx = Self::begin_transaction().await?;
        let tx_arc = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));
        let result = CURRENT_TX.scope(tx_arc.clone(), f(tx_arc.clone())).await;

        match result {
            Ok(val) => {
                if let Some(tx) = tx_arc.lock().await.take() {
                    tx.commit().await?;
                }
                Ok(val)
            }
            Err(err) => {
                if let Some(tx) = tx_arc.lock().await.take() {
                    let _ = tx.rollback().await;
                }
                Err(crate::Error::DatabaseError(format!(
                    "Transaction failed: {}",
                    err
                )))
            }
        }
    }

    /// Run an array of seeders sequentially
    #[cfg_attr(test, mutants::skip)]
    pub async fn seed(seeders: Vec<Box<dyn Seeder>>) -> Result<(), crate::Error> {
        for seeder in seeders {
            seeder.run().await?;
        }
        Ok(())
    }

    /// Enable query logging to print all queries to the terminal
    pub fn enable_query_log() {
        crate::schema::enable_query_log();
    }

    /// Disable query logging
    pub fn disable_query_log() {
        crate::schema::disable_query_log();
    }

    /// Set a global maximum limit for all queries without an explicit limit override
    pub fn set_max_query_limit(limit: usize) {
        crate::schema::set_max_query_limit(limit);
    }

    /// Set a global maximum execution timeout for all queries
    pub fn set_query_timeout(secs: u64) {
        crate::schema::set_query_timeout(secs);
    }

    /// Initialize Redis connection and connection manager for caching and events
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub async fn init_redis(redis_url: &str) -> Result<(), crate::Error> {
        let client = _redis::Client::open(redis_url)?;
        let manager = _redis::aio::ConnectionManager::new(client.clone()).await?;
        let _ = REDIS_CLIENT.set(client);
        let _ = REDIS_MANAGER.set(manager);
        Ok(())
    }

    /// Get reference to the global Redis client
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_client() -> Result<&'static _redis::Client, crate::Error> {
        REDIS_CLIENT.get().ok_or_else(|| {
            crate::Error::Internal(
                "Orm::init_redis() must be called before using cache features".to_string(),
            )
        })
    }

    /// Get clone of the thread-safe connection manager for async Redis queries
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_manager() -> Result<_redis::aio::ConnectionManager, crate::Error> {
        REDIS_MANAGER.get().cloned().ok_or_else(|| {
            crate::Error::Internal(
                "Orm::init_redis() must be called before using cache features".to_string(),
            )
        })
    }
}

/// A database seeder trait for populating tables
#[async_trait]
pub trait Seeder: Send + Sync {
    async fn run(&self) -> Result<(), crate::Error>;
}

/// The core trait that all Orm models will implement via #[derive(Orm)]
#[async_trait]
pub trait RullstModel {
    fn table_name() -> &'static str;
}

/// Represents a paginated result set
#[derive(Debug, Clone)]
pub struct PaginationResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub per_page: usize,
    pub current_page: usize,
    pub last_page: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn test_pagination_result() {
        let mut pr = PaginationResult {
            data: vec![1, 2, 3],
            total: 3,
            per_page: 10,
            current_page: 1,
            last_page: 1,
        };
        assert_eq!(pr.data.len(), 3);
        assert_eq!(pr.total, 3);
        pr.data.push(4);
        assert_eq!(pr.data.len(), 4);
    }

    #[test]
    fn test_replace_placeholders() {
        assert_eq!(
            super::replace_placeholders("SELECT * FROM users WHERE id = ? AND name = ?"),
            "SELECT * FROM users WHERE id = $1 AND name = $2"
        );
        assert_eq!(
            super::replace_placeholders("INSERT INTO users (name) VALUES (?)"),
            "INSERT INTO users (name) VALUES ($1)"
        );
        assert_eq!(
            super::replace_placeholders("SELECT * FROM users"),
            "SELECT * FROM users"
        );
        assert_eq!(super::replace_placeholders("? ? ?"), "$1 $2 $3");
    }

    #[test]
    fn test_rullst_value_conversions() {
        // From
        let v: RullstValue = "test".into();
        assert!(matches!(v, RullstValue::String(_)));
        let v_string: RullstValue = "test".to_string().into();
        assert!(matches!(v_string, RullstValue::String(_)));
        let v_int: RullstValue = 100.into();
        assert!(matches!(v_int, RullstValue::Int(100)));
        let v_bool: RullstValue = false.into();
        assert!(matches!(v_bool, RullstValue::Bool(false)));
        let v_float: RullstValue = std::f64::consts::PI.into();
        assert!(matches!(v_float, RullstValue::Float(_)));

        // TryFrom String
        let v_str_conv = RullstValue::String("hello".to_string());
        assert_eq!(String::try_from(v_str_conv).unwrap(), "hello");
        assert!(String::try_from(RullstValue::Int(10)).is_err());

        // TryFrom i32
        let v_int_conv = RullstValue::Int(42);
        assert_eq!(i32::try_from(v_int_conv).unwrap(), 42);
        assert!(i32::try_from(RullstValue::Bool(true)).is_err());

        // TryFrom f64
        let v_float_conv = RullstValue::Float(2.71);
        assert_eq!(f64::try_from(v_float_conv).unwrap(), 2.71);
        assert!(f64::try_from(RullstValue::Int(10)).is_err());

        // TryFrom bool
        let v_bool_conv = RullstValue::Bool(true);
        assert!(bool::try_from(v_bool_conv).unwrap());
        assert!(bool::try_from(RullstValue::Int(10)).is_err());
    }

    #[test]
    fn test_enable_query_log_wrapper() {
        // Orm::enable/disable_query_log delegate to schema — verify the delegation works.
        Orm::disable_query_log();
        assert!(!crate::schema::is_query_log_enabled());
        Orm::enable_query_log();
        assert!(crate::schema::is_query_log_enabled());
        Orm::disable_query_log();
        assert!(!crate::schema::is_query_log_enabled());
    }

    #[test]
    fn test_disable_query_log_wrapper() {
        Orm::enable_query_log();
        Orm::disable_query_log();
        assert!(!crate::schema::is_query_log_enabled());
    }

    #[cfg(feature = "redis")]
    #[test]
    fn test_redis_client_uninitialized() {
        let err = Orm::redis_client().unwrap_err();
        assert!(matches!(err, crate::Error::Internal(_)));
    }

    #[cfg(feature = "redis")]
    #[test]
    fn test_redis_manager_uninitialized() {
        let err = Orm::redis_manager().unwrap_err();
        assert!(matches!(err, crate::Error::Internal(_)));
    }

    #[test]
    #[should_panic(expected = "Orm must be initialized before querying")]
    fn test_pool_uninitialized() {
        let _ = Orm::pool();
    }

    #[test]
    #[should_panic(expected = "Orm must be initialized before querying")]
    fn test_driver_uninitialized() {
        let _ = Orm::driver();
    }

    #[test]
    #[should_panic(expected = "Orm must be initialized before querying")]
    fn test_read_pool_uninitialized() {
        let _ = Orm::read_pool();
    }

    #[test]
    fn test_validate_dsn() {
        // Safe case
        Orm::validate_dsn("sqlite::memory:");
        // Security warning case (printed to stderr, shouldn't panic)
        Orm::validate_dsn("postgres://external-db.com/mydb?sslmode=disable");
    }

    #[cfg(feature = "redis")]
    #[tokio::test]
    async fn test_init_redis_failure() {
        let err = Orm::init_redis("redis://127.0.0.1:0").await.unwrap_err();
        assert!(matches!(err, crate::Error::CacheError(_)));
    }

    #[test]
    fn test_orm_max_query_limit_and_timeout() {
        Orm::set_max_query_limit(15);
        assert_eq!(crate::schema::get_max_query_limit(), Some(15));
        Orm::set_max_query_limit(0);
        assert_eq!(crate::schema::get_max_query_limit(), None);

        Orm::set_query_timeout(5);
        assert_eq!(
            crate::schema::get_query_timeout(),
            Some(std::time::Duration::from_secs(5))
        );
        Orm::set_query_timeout(0);
        assert_eq!(crate::schema::get_query_timeout(), None);
    }
}
