//! Global database connection pool manager, replica load-balancer, and ORM facade.

use async_trait::async_trait;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{RullstPool, RullstPoolOptions};

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
use sqlx::any::install_default_drivers;

/// The global connection pool
pub(crate) static DB_POOL: OnceLock<RullstPool> = OnceLock::new();

/// The driver identifier (postgres, mysql, sqlite) to help macro syntax formatting
pub(crate) static DB_DRIVER: OnceLock<String> = OnceLock::new();

/// The replica connection pools for read operations
static REPLICA_POOLS: OnceLock<Vec<RullstPool>> = OnceLock::new();

/// Atomic index for replica round-robin selection
static REPLICA_INDEX: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "redis")]
static REDIS_CLIENT: OnceLock<crate::_redis::Client> = OnceLock::new();

#[cfg(feature = "redis")]
static REDIS_MANAGER: OnceLock<crate::_redis::aio::ConnectionManager> = OnceLock::new();

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

/// Trait implementada automaticamente pelas macros para os modelos que usam `#[orm(rag_context)]`
pub trait RagContext {
    fn get_context(&self) -> String;
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

        let pool = RullstPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
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
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
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
    pub(crate) fn validate_dsn(database_url: &str) {
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

        let pool = RullstPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .connect(primary_url)
            .await?;

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

    /// Retrieve the active driver string (panics if DB is not initialized yet).
    #[allow(clippy::expect_used)]
    pub fn driver() -> &'static str {
        DB_DRIVER
            .get()
            .map(|s| s.as_str())
            .expect("Orm must be initialized before querying")
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
    pub fn raw(sql: &str) -> crate::raw::RawQueryBuilder {
        crate::raw::RawQueryBuilder::new(sql)
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
        let result = crate::CURRENT_TX
            .scope(tx_arc.clone(), f(tx_arc.clone()))
            .await;

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
        let client = crate::_redis::Client::open(redis_url)?;
        let manager = crate::_redis::aio::ConnectionManager::new(client.clone()).await?;
        let _ = REDIS_CLIENT.set(client);
        let _ = REDIS_MANAGER.set(manager);
        Ok(())
    }

    /// Get reference to the global Redis client
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_client() -> Result<&'static crate::_redis::Client, crate::Error> {
        REDIS_CLIENT.get().ok_or_else(|| {
            crate::Error::Internal(
                "Orm::init_redis() must be called before using cache features".to_string(),
            )
        })
    }

    /// Get clone of the thread-safe connection manager for async Redis queries
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_manager() -> Result<crate::_redis::aio::ConnectionManager, crate::Error> {
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
