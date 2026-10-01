use crate::Router;
use crate::lifecycle::ApplicationLifecycle;
use crate::scheduler::Scheduler;

mod environment;
mod shutdown;
mod startup;

#[doc(hidden)]
pub use environment::read_optional_environment_variable;
pub(super) use environment::{development_console_enabled, parse_dotenv};
pub use shutdown::shutdown_signal;

/// Typed server startup and runtime failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// Application configuration is invalid or unreadable.
    #[error("server configuration error: {0}")]
    Configuration(String),

    /// A configured database could not be initialized.
    #[error("database initialization failed: {0}")]
    Database(String),

    /// A configured background scheduler could not be started or stopped.
    #[error("scheduler lifecycle failed: {0}")]
    Scheduler(#[from] crate::scheduler::SchedulerError),

    /// Traffic Shield monitoring could not be started safely.
    #[error("traffic shield lifecycle failed: {0}")]
    TrafficShield(#[from] crate::resilience::TrafficShieldError),

    /// The process readiness or graceful-drain lifecycle became invalid.
    #[error("application lifecycle failed: {0}")]
    Lifecycle(#[from] crate::lifecycle::ApplicationLifecycleError),

    /// The requested listen address is invalid.
    #[error("invalid server listen address `{host}:{port}`")]
    InvalidAddress {
        /// Configured host value.
        host: String,
        /// Configured TCP port.
        port: u16,
    },

    /// Hot reload was requested outside its supported local debug mode.
    #[error("hot reload is available only in local development debug builds")]
    HotReloadDisabled,

    /// Loading or invoking a hot-reload library failed.
    #[error("hot reload failed: {0}")]
    HotReload(String),

    /// The private development reload channel is not configured safely.
    #[error("hot reload configuration error: {0}")]
    HotReloadConfiguration(String),

    /// An operating-system I/O operation failed.
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[non_exhaustive]
/// The central application server builder for Rullst.
///
/// Configures and boots the Axum HTTP server, optional ORM connection pool,
/// task scheduler, hot-reload DLL watcher, traffic shield, and rate limiter in
/// a single fluent chain.
///
/// # Example
/// ```rust,no_run
/// use rullst_core::{Server, routes, routing::get};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     Server::new(routes![get("/" => || async { "OK" })])
///         .run(3000)
///         .await?;
///     Ok(())
/// }
/// ```
pub struct Server {
    pub(crate) router: Router,
    pub(crate) db_url: Option<String>,
    pub(crate) scheduler: Option<Scheduler>,
    pub(crate) hot_reload_lib: Option<String>,
    pub(crate) shield: Option<crate::resilience::TrafficShield>,
    pub(crate) limiter: Option<crate::resilience::RateLimiter>,
    pub(crate) lifecycle: Option<ApplicationLifecycle>,
    pub(crate) machine_endpoints: Option<crate::security::MachineEndpointPolicy>,
    pub(crate) trusted_proxy: Option<crate::security::TrustedProxyConfig>,
}

impl Server {
    /// Creates a new `Server` from an already-built [`Router`].
    /// Use [`Server::new_hot`] instead to enable hot-reload mode.
    pub fn new(router: Router) -> Self {
        Server {
            router,
            db_url: None,
            scheduler: None,
            hot_reload_lib: None,
            shield: None,
            limiter: None,
            lifecycle: None,
            machine_endpoints: None,
            trusted_proxy: None,
        }
    }

    /// Creates a `Server` in **hot-reload** mode that loads the application router from
    /// a compiled `cdylib` dynamic library at the given `lib_path`.
    /// The background file-watcher recompiles and hot-swaps the router on source changes.
    pub fn new_hot<S: Into<String>>(lib_path: S) -> Self {
        Server {
            router: Router::new(),
            db_url: None,
            scheduler: None,
            hot_reload_lib: Some(lib_path.into()),
            shield: None,
            limiter: None,
            lifecycle: None,
            machine_endpoints: None,
            trusted_proxy: None,
        }
    }

    /// Requires explicit machine authentication before CSRF exemptions on exact routes.
    pub fn with_machine_endpoints(
        mut self,
        policy: crate::security::MachineEndpointPolicy,
    ) -> Self {
        self.machine_endpoints = Some(policy);
        self
    }

    /// Sets a database URL to initialize the ORM connection pool at startup.
    ///
    /// This requires the `orm` feature. When Core is compiled without `orm`,
    /// configuring a database remains a valid builder operation but [`Self::run`]
    /// fails closed with [`ServerError::Database`].
    pub fn with_db<S: Into<String>>(mut self, db_url: S) -> Self {
        self.db_url = Some(db_url.into());
        self
    }

    /// Attach a task scheduler that runs alongside the HTTP server.
    ///
    /// The server owns the scheduler handle: every task failure is logged as
    /// a `tracing` error on the `rullst::scheduler` target when it is
    /// reported, and task failures never make a clean shutdown fail. Only a
    /// failed scheduler loop is returned as [`ServerError::Scheduler`].
    ///
    /// # Example
    /// ```rust,no_run
    /// use rullst_core::{Server, Scheduler, routes, routing::get};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let scheduler = Scheduler::new()
    ///         .task("0 0 * * *", || async { println!("daily cleanup"); })?;
    ///     let router = routes![get("/" => || async { "OK" })];
    ///
    ///     Server::new(router)
    ///         .schedule(scheduler)
    ///         .run(3000)
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn schedule(mut self, scheduler: Scheduler) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Attaches an adaptive TrafficShield to the server to protect against CPU/DB saturation.
    ///
    /// Exact `GET`/`HEAD /health` and `/ready` probes are never shed.
    pub fn shield(mut self, shield: crate::resilience::TrafficShield) -> Self {
        self.shield = Some(shield);
        self
    }

    /// Attaches a global RateLimiter to the server.
    ///
    /// Exact `GET`/`HEAD /health` and `/ready` probes do not consume tokens.
    pub fn rate_limit(mut self, limiter: crate::resilience::RateLimiter) -> Self {
        self.limiter = Some(limiter);
        self
    }

    /// Attaches a shared readiness and graceful-drain coordinator.
    ///
    /// Static and development hot-reload requests are admitted only while this
    /// lifecycle and all of its required components are ready. Exact health
    /// probes remain reachable. Mount
    /// [`crate::health::health_router_with_lifecycle`] with a clone to expose
    /// the same aggregate state to an orchestrator.
    pub fn with_lifecycle(mut self, lifecycle: ApplicationLifecycle) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    /// Resolves the client address behind the listed reverse proxies.
    ///
    /// Installs [`crate::security::TrustedProxyLayer`] outside every other
    /// framework layer (security baseline, lifecycle, Traffic Shield and rate
    /// limiting), matching [`crate::ProductionPreset::MIDDLEWARE_ORDER`], in both
    /// the static and the development hot-reload server. This policy replaces
    /// the `[security]` `trusted_proxies` settings of `Rullst.toml`; an empty
    /// policy disables forwarded-address resolution. List only the networks
    /// your own proxies connect from.
    ///
    /// ```rust,no_run
    /// use rullst_core::{Server, routes, routing::get, security::TrustedProxyConfig};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// Server::new(routes![get("/" => || async { "OK" })])
    ///     .trusted_proxies(TrustedProxyConfig::new(["10.0.0.0/8"])?)
    ///     .run(3000)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn trusted_proxies(mut self, config: crate::security::TrustedProxyConfig) -> Self {
        self.trusted_proxy = Some(config);
        self
    }

    /// Start the HTTP server on the specified port
    ///
    /// The listen host comes from `HOST`, then the legacy `RULLST_HOST` (the
    /// process environment first, then `.env`), defaulting to `127.0.0.1`, or
    /// `0.0.0.0` in staging and production. It must be an IP address (IPv6
    /// with or without brackets, such as `::` or `[::1]`) or `localhost`,
    /// which binds `127.0.0.1`. Other host names are rejected with
    /// [`ServerError::InvalidAddress`] instead of being resolved, so a shell
    /// that exports `HOST` as the machine name (tcsh does) cannot silently
    /// bind a LAN interface; set `HOST` explicitly in that case.
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run(self, port: u16) -> Result<(), ServerError> {
        self.run_with_shutdown(port, shutdown_signal()).await
    }

    /// Starts the HTTP server with a caller-supplied graceful-shutdown future.
    ///
    /// This is useful for embedded process supervisors and deterministic tests.
    /// Resolving the future begins lifecycle draining before Axum waits for
    /// already accepted requests. [`Self::run`] remains the OS-signal default.
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run_with_shutdown<F>(self, port: u16, shutdown: F) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let lifecycle = self.lifecycle.clone();
        let result = self.run_inner(port, shutdown).await;
        if result.is_err()
            && let Some(lifecycle) = lifecycle
        {
            lifecycle.mark_stopped();
        }
        result
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
