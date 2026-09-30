//! Global database connection pool manager, replica load-balancer, and ORM facade.

use async_trait::async_trait;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{RullstPool, RullstPoolOptions};

const POOL_SLOW_ACQUIRE_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(500);
/// Upper bound of `Orm::init_with_options`'s acquire timeout. SQLx adds the
/// timeout to `Instant::now()` unchecked, so an unbounded value would panic.
const MAX_ACQUIRE_TIMEOUT_SECS: u64 = 86_400;

mod dsn;
mod placeholders;
mod savepoint;
mod telemetry;
#[doc(hidden)]
pub use placeholders::portable_subquery;
pub use placeholders::raw_fragment;
pub use placeholders::replace_placeholders;

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
use sqlx::any::install_default_drivers;

/// Coherent global state published only after the primary and every configured
/// replica have connected successfully.
struct OrmState {
    primary: RullstPool,
    driver: &'static str,
    replicas: Vec<RullstPool>,
    replica_index: AtomicUsize,
}

impl OrmState {
    fn new(primary: RullstPool, driver: &'static str, replicas: Vec<RullstPool>) -> Self {
        Self {
            primary,
            driver,
            replicas,
            replica_index: AtomicUsize::new(0),
        }
    }

    fn read_pool(&self) -> &RullstPool {
        if self.replicas.is_empty() {
            return &self.primary;
        }

        let index = self.replica_index.fetch_add(1, Ordering::Relaxed) % self.replicas.len();
        &self.replicas[index]
    }
}

static ORM_STATE: OnceLock<OrmState> = OnceLock::new();

#[cfg(feature = "redis")]
struct RedisState {
    client: crate::_redis::Client,
    manager: crate::_redis::aio::ConnectionManager,
    cache_namespace: String,
}

#[cfg(feature = "redis")]
static REDIS_STATE: OnceLock<RedisState> = OnceLock::new();

static PREVENT_LAZY_LOADING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Prevents relationships from being lazily loaded when accessed without being eager loaded.
/// When enabled, attempting to lazily load a relation returns a validation error.
pub fn prevent_lazy_loading(prevent: bool) {
    PREVENT_LAZY_LOADING.store(prevent, std::sync::atomic::Ordering::Relaxed);
}

#[doc(hidden)]
pub fn is_lazy_loading_prevented() -> bool {
    PREVENT_LAZY_LOADING.load(std::sync::atomic::Ordering::Relaxed)
}

/// Trait implementada automaticamente pelas macros para os modelos que usam `#[orm(rag_context)]`
pub trait RagContext {
    fn get_context(&self) -> String;
}

/// Orm configuration structure
pub struct Orm;

impl Orm {
    fn pool_options() -> RullstPoolOptions {
        RullstPoolOptions::new()
            .acquire_time_level(tracing::log::LevelFilter::Info)
            .acquire_slow_level(tracing::log::LevelFilter::Warn)
            .acquire_slow_threshold(POOL_SLOW_ACQUIRE_THRESHOLD)
            .after_release(savepoint::release_outside_transaction)
    }

    fn ensure_uninitialized() -> Result<(), crate::Error> {
        if ORM_STATE.get().is_some() {
            Err(crate::Error::AlreadyInitialized)
        } else {
            Ok(())
        }
    }

    fn publish(state: OrmState) -> Result<(), crate::Error> {
        ORM_STATE
            .set(state)
            .map_err(|_| crate::Error::AlreadyInitialized)
    }

    fn state() -> Result<&'static OrmState, crate::Error> {
        ORM_STATE.get().ok_or(crate::Error::NotInitialized)
    }

    /// Initialize the global database connection pool using an agnostic URI
    pub async fn init(database_url: &str) -> Result<(), crate::Error> {
        Self::ensure_uninitialized()?;
        dsn::ensure_configured_dsn(database_url)?;

        Self::validate_dsn(database_url);

        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = Self::pool_options()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .connect(database_url)
            .await?;

        Self::publish(OrmState::new(
            pool,
            dsn::driver_for_url(database_url),
            Vec::new(),
        ))
    }

    /// Initialize the global database connection pool with specific pool options.
    ///
    /// `max_connections` must be at least 1 and `acquire_timeout_secs` within
    /// 1–86,400 seconds; other values return `Error::Validation`.
    pub async fn init_with_options(
        database_url: &str,
        max_connections: u32,
        acquire_timeout_secs: u64,
    ) -> Result<(), crate::Error> {
        Self::ensure_uninitialized()?;
        if max_connections == 0 || !(1..=MAX_ACQUIRE_TIMEOUT_SECS).contains(&acquire_timeout_secs) {
            return Err(crate::Error::Validation(format!(
                "pool max_connections must be at least 1 and acquire_timeout_secs 1-{MAX_ACQUIRE_TIMEOUT_SECS}"
            )));
        }
        dsn::ensure_configured_dsn(database_url)?;
        Self::validate_dsn(database_url);

        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = Self::pool_options()
            .max_connections(max_connections)
            .acquire_timeout(std::time::Duration::from_secs(acquire_timeout_secs))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .connect(database_url)
            .await?;

        Self::publish(OrmState::new(
            pool,
            dsn::driver_for_url(database_url),
            Vec::new(),
        ))
    }

    #[cfg_attr(test, mutants::skip)]
    pub(crate) fn validate_dsn(database_url: &str) {
        // Detect unconfigured placeholder URLs (e.g. the Turso template uses
        // [your-database-id] which the URL parser misreads as an IPv6 address).
        if dsn::has_placeholder_brackets(database_url) {
            eprintln!(
                "⚠️ [CONFIG] Rullst ORM: DATABASE_URL still contains placeholder brackets (e.g. [your-database-id]). \
                Update your .env file with a real connection string before using database features."
            );
            return;
        }

        dsn::prepare_sqlite_file(database_url);

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
        Self::ensure_uninitialized()?;
        dsn::ensure_configured_dsn(primary_url)?;
        for replica_url in &replica_urls {
            dsn::ensure_configured_dsn(replica_url)?;
        }
        Self::validate_dsn(primary_url);
        for replica_url in &replica_urls {
            Self::validate_dsn(replica_url);
        }
        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        install_default_drivers();

        let pool = Self::pool_options()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .connect(primary_url)
            .await?;

        // Prepare every replica before publishing any global state. If one
        // connection fails, `pool` and the successfully prepared replicas are
        // dropped locally and initialization remains retryable.
        let replica_futures: Vec<_> = replica_urls
            .into_iter()
            .map(|replica_url| Self::pool_options().connect(replica_url))
            .collect();
        let replicas = futures::future::try_join_all(replica_futures).await?;

        Self::publish(OrmState::new(
            pool,
            dsn::driver_for_url(primary_url),
            replicas,
        ))
    }

    /// Retrieve the global database connection pool (strictly for writes).
    pub fn pool() -> Result<&'static RullstPool, crate::Error> {
        Self::try_pool()
    }

    /// Fallible variant of [`Orm::pool`]: returns `Err` instead of panicking
    /// when the ORM has not been initialized yet.
    pub fn try_pool() -> Result<&'static RullstPool, crate::Error> {
        crate::__transaction_access::ensure_allowed()?;
        Ok(&Self::state()?.primary)
    }

    /// Retrieve the connection pool for read operations.
    /// Performs a round-robin load balancing over replicas if configured.
    #[cfg_attr(test, mutants::skip)]
    pub fn read_pool() -> Result<&'static RullstPool, crate::Error> {
        Self::try_read_pool()
    }

    /// Fallible variant of [`Orm::read_pool`].
    #[cfg_attr(test, mutants::skip)]
    pub fn try_read_pool() -> Result<&'static RullstPool, crate::Error> {
        crate::__transaction_access::ensure_allowed()?;
        Ok(Self::state()?.read_pool())
    }

    /// Retrieve the active driver string.
    pub fn driver() -> Result<&'static str, crate::Error> {
        Self::try_driver()
    }

    /// Fallible variant of [`Orm::driver`].
    pub fn try_driver() -> Result<&'static str, crate::Error> {
        Ok(Self::state()?.driver)
    }

    /// Create a raw SQL query builder.
    pub fn raw(sql: impl Into<String>) -> crate::raw::RawQueryBuilder {
        crate::raw::RawQueryBuilder::new(sql)
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

    /// Initializes Redis with the compatibility cache namespace `default`.
    ///
    /// Applications sharing one Redis database should prefer
    /// [`Orm::init_redis_with_namespace`] and choose a unique stable namespace.
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub async fn init_redis(redis_url: &str) -> Result<(), crate::Error> {
        Self::init_redis_with_namespace(redis_url, "default").await
    }

    /// Initializes Redis and isolates generated `.remember(...)` query keys
    /// under an application-specific namespace.
    ///
    /// The namespace must contain 1-64 ASCII letters, digits, dots, dashes or
    /// underscores. Redis hash model helpers keep their existing key contract.
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub async fn init_redis_with_namespace(
        redis_url: &str,
        cache_namespace: impl Into<String>,
    ) -> Result<(), crate::Error> {
        if REDIS_STATE.get().is_some() {
            return Err(crate::Error::AlreadyInitialized);
        }
        let cache_namespace = cache_namespace.into();
        crate::query_cache::validate_namespace(&cache_namespace)?;
        let client = crate::_redis::Client::open(redis_url)?;
        let manager = crate::_redis::aio::ConnectionManager::new(client.clone()).await?;
        REDIS_STATE
            .set(RedisState {
                client,
                manager,
                cache_namespace,
            })
            .map_err(|_| crate::Error::AlreadyInitialized)
    }

    /// Get reference to the global Redis client
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_client() -> Result<&'static crate::_redis::Client, crate::Error> {
        Ok(&Self::redis_state()?.client)
    }

    /// Get clone of the thread-safe connection manager for async Redis queries
    #[cfg(feature = "redis")]
    #[cfg_attr(test, mutants::skip)]
    pub fn redis_manager() -> Result<crate::_redis::aio::ConnectionManager, crate::Error> {
        Ok(Self::redis_state()?.manager.clone())
    }

    #[cfg(feature = "redis")]
    fn redis_state() -> Result<&'static RedisState, crate::Error> {
        REDIS_STATE.get().ok_or_else(|| {
            crate::Error::Internal(
                "Orm::init_redis() or Orm::init_redis_with_namespace() must be called before using Redis features".to_string(),
            )
        })
    }

    /// Returns the application namespace used by generated query caches.
    #[cfg(feature = "redis")]
    #[doc(hidden)]
    pub fn redis_cache_namespace() -> Result<&'static str, crate::Error> {
        Ok(Self::redis_state()?.cache_namespace.as_str())
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
mod tests;
