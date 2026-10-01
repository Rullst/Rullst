use crate::Router;
use crate::lifecycle::ApplicationLifecycle;
use crate::scheduler::{Scheduler, SchedulerHandle};
use crate::server::dylib_loader::load_dylib_router;
use crate::server::hotswap::{HotSwapService, PeerAwareHotSwap};
#[cfg(feature = "orm")]
use rullst_orm::Orm;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

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

    async fn run_inner<F>(mut self, port: u16, shutdown: F) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let dotenv = Self::load_dotenv_values().await?;
        #[cfg(feature = "orm")]
        crate::artisan::runner::intercept_artisan_command(None, self.db_url.as_deref()).await;
        let _ = crate::telemetry::init_telemetry();
        let app_config = Self::load_config().await?;
        let environment = resolve_environment(&app_config, &dotenv)?;
        // `RullstConfig::environment` reports the same environment.
        crate::config::record_project_environment_selector(
            dotenv
                .get("RULLST_ENV")
                .or_else(|| dotenv.get("APP_ENV"))
                .cloned(),
        );
        self.resolve_trusted_proxy(&app_config.security)?;

        self.init_database(&app_config, &dotenv).await?;
        let addr = Self::setup_networking(port, app_config.app.port, environment, &dotenv)?;
        let mut scheduler_handle = self.start_scheduler()?;
        let shield_lifecycle = self.start_traffic_shield()?;

        let server = async move {
            if let Some(lib_path) = self.hot_reload_lib.take() {
                self.run_hot_reload(lib_path, addr, environment, shutdown)
                    .await
            } else {
                self.run_static(app_config, addr, environment, shutdown)
                    .await
            }
        };
        // The server owns the only scheduler handle: log task failures as
        // they happen and keep them out of the clean-shutdown result.
        let server_result =
            super::scheduler_supervision::serve_while_draining(server, scheduler_handle.as_mut())
                .await;

        if let Some(shield) = shield_lifecycle {
            shield.shutdown();
        }
        let scheduler_result = super::scheduler_supervision::stop_scheduler(scheduler_handle).await;

        match server_result {
            Err(error) => Err(error),
            Ok(()) => scheduler_result,
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_dotenv_values() -> Result<HashMap<String, String>, ServerError> {
        super::database_url::load_dotenv_file(std::path::Path::new(".env")).await
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_config() -> Result<crate::config::RullstConfig, ServerError> {
        let app_config =
            super::database_url::load_config_file(std::path::Path::new("Rullst.toml")).await?;

        app_config
            .validate()
            .map_err(|error| ServerError::Configuration(error.to_string()))?;
        let _ = crate::config::RullstConfig::set_global(app_config.clone());
        Ok(app_config)
    }

    #[cfg(feature = "orm")]
    #[cfg_attr(mutants, mutants::skip)]
    async fn init_database(
        &mut self,
        app_config: &crate::config::RullstConfig,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        if rullst_orm::Orm::try_pool().is_ok() {
            return Ok(());
        }

        if self.db_url.is_none() {
            self.db_url = super::database_url::resolve_database_url(
                None,
                read_optional_environment_variable,
                dotenv,
                app_config,
            )?;
        }

        if let Some(db_url) = &self.db_url {
            println!("Initializing Orm database pool...");
            Orm::init(db_url)
                .await
                .map_err(|error| ServerError::Database(error.to_string()))?;
            println!("Database initialized successfully.");
        }

        Ok(())
    }

    #[cfg(not(feature = "orm"))]
    #[cfg_attr(mutants, mutants::skip)]
    async fn init_database(
        &mut self,
        app_config: &crate::config::RullstConfig,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        let database_requested = super::database_url::resolve_database_url(
            self.db_url.as_deref(),
            read_optional_environment_variable,
            dotenv,
            app_config,
        )?
        .is_some();

        if database_requested {
            return Err(ServerError::Database(
                "rullst-core was compiled without the `orm` feature".to_string(),
            ));
        }

        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn start_scheduler(&mut self) -> Result<Option<SchedulerHandle>, ServerError> {
        self.scheduler
            .take()
            .map(Scheduler::start)
            .transpose()
            .map_err(ServerError::from)
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn start_traffic_shield(
        &self,
    ) -> Result<Option<crate::resilience::TrafficShield>, ServerError> {
        let Some(shield) = self.shield.clone() else {
            return Ok(None);
        };
        shield.start().map_err(ServerError::from)?;
        Ok(Some(shield))
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn setup_networking(
        fallback_port: u16,
        configured_port: Option<u16>,
        environment: crate::config::Environment,
        dotenv: &HashMap<String, String>,
    ) -> Result<SocketAddr, ServerError> {
        let host_str = read_optional_environment_variable("HOST")?
            .or(read_optional_environment_variable("RULLST_HOST")?)
            .or_else(|| dotenv.get("HOST").cloned())
            .or_else(|| dotenv.get("RULLST_HOST").cloned())
            .unwrap_or_else(|| {
                if environment.requires_secure_defaults() {
                    "0.0.0.0".to_string()
                } else {
                    "127.0.0.1".to_string()
                }
            });

        let env_port_value =
            read_optional_environment_variable("PORT")?.or_else(|| dotenv.get("PORT").cloned());
        let env_port = match env_port_value {
            Some(value) => Some(value.parse::<u16>().map_err(|_| {
                ServerError::Configuration(format!("PORT must be a valid u16, got `{value}`"))
            })?),
            None => None,
        };
        let port = env_port.or(configured_port).unwrap_or(fallback_port);

        let addr: SocketAddr =
            format!("{host_str}:{port}")
                .parse()
                .map_err(|_| ServerError::InvalidAddress {
                    host: host_str,
                    port,
                })?;

        if development_console_enabled(cfg!(debug_assertions), environment)
            && addr.ip().is_unspecified()
        {
            eprintln!(
                "⚠️  Rullst Dev: Self-Healing Console mounted on /_rullst/*\n\
                   Set RULLST_ENV=production to disable before deploying."
            );
        }

        Ok(addr)
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn run_hot_reload<F>(
        self,
        lib_path: String,
        addr: SocketAddr,
        environment: crate::config::Environment,
        shutdown: F,
    ) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        if !cfg!(debug_assertions) || !environment.allows_development_tools() {
            return Err(ServerError::HotReloadDisabled);
        }

        let is_dev = true;
        let reload_token = resolve_hot_reload_token()?;

        println!(
            "\x1b[36mRullst: initializing the authenticated development-library reload boundary...\x1b[0m"
        );

        let (initial_router, library) = match load_dylib_router(&lib_path, is_dev) {
            Ok(r) => r,
            Err(e) => {
                println!(
                    "\x1b[31m❌ Failed to load initial dylib: {}. Make sure the dynamic library was compiled by running 'cargo build --lib'.\x1b[0m",
                    e
                );
                return Err(ServerError::HotReload(e.to_string()));
            }
        };

        let current_router = Arc::new(RwLock::new(initial_router));
        let active_libraries = Arc::new(Mutex::new(vec![library]));
        let (hmr_sender, _receiver) = tokio::sync::broadcast::channel(32);

        let hotswap_service = HotSwapService {
            current_router: current_router.clone(),
            active_libraries: active_libraries.clone(),
            hmr_sender,
            reload_lock: Arc::new(tokio::sync::Mutex::new(())),
            reload_token,
            lib_path: lib_path.clone(),
            is_dev,
            shield: self.shield.clone(),
            limiter: self.limiter.clone(),
            lifecycle: self.lifecycle.clone(),
            trusted_proxy: self.trusted_proxy_layer(),
        };

        println!(
            "Rullst framework serving on http://{} (authenticated development hot reload)",
            addr
        );
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        mark_lifecycle_ready(self.lifecycle.as_ref())?;
        let lifecycle = self.lifecycle.clone();
        let result = axum::serve(listener, PeerAwareHotSwap(hotswap_service))
            .with_graceful_shutdown(shutdown_with_lifecycle(shutdown, lifecycle.clone()))
            .await
            .map_err(ServerError::from);
        mark_lifecycle_stopped(lifecycle.as_ref());
        result
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn run_static<F>(
        self,
        app_config: crate::config::RullstConfig,
        addr: SocketAddr,
        environment: crate::config::Environment,
        shutdown: F,
    ) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let lifecycle = self.lifecycle.clone();
        let app = self.into_static_app(app_config.security, environment)?;

        println!("Rullst framework serving on http://{}", addr);
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        mark_lifecycle_ready(lifecycle.as_ref())?;
        let result = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_with_lifecycle(shutdown, lifecycle.clone()))
        .await
        .map_err(ServerError::from);
        mark_lifecycle_stopped(lifecycle.as_ref());
        result
    }
}

/// Reads an optional process environment variable for database URL resolution.
///
/// An absent variable is `None`; a non-Unicode value is a configuration error
/// that names the variable but never contains its value. Hidden support API for
/// first-party tools such as Rullst Studio, not a stable extension point.
#[doc(hidden)]
pub fn read_optional_environment_variable(name: &str) -> Result<Option<String>, ServerError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ServerError::Configuration(format!(
            "{name} is not valid Unicode"
        ))),
    }
}

/// The panic console and `/_rullst/explain`/`/_rullst/autofix` are mounted only
/// in debug builds running in Development, like hot reload and the generation
/// probe. An unset environment resolves to Development, so a release binary must
/// not rely on the environment alone.
pub(super) fn development_console_enabled(
    debug_build: bool,
    environment: crate::config::Environment,
) -> bool {
    debug_build && environment.allows_development_tools()
}

fn resolve_hot_reload_token() -> Result<Arc<str>, ServerError> {
    let token = read_optional_environment_variable("RULLST_HMR_TOKEN")?.ok_or_else(|| {
        ServerError::HotReloadConfiguration(
            "RULLST_HMR_TOKEN is missing; start hot reload through `cargo rullst dev` or `cargo rullst dash`"
                .to_string(),
        )
    })?;
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ServerError::HotReloadConfiguration(
            "RULLST_HMR_TOKEN must contain exactly 64 hexadecimal characters".to_string(),
        ));
    }
    Ok(Arc::from(token))
}

fn resolve_environment(
    config: &crate::config::RullstConfig,
    dotenv: &HashMap<String, String>,
) -> Result<crate::config::Environment, ServerError> {
    super::project_settings::resolve_environment(dotenv, config.app.env.as_deref())
}

/// Listens for OS termination signals (SIGINT / SIGTERM / Ctrl+C) to drain in-flight requests cleanly.
#[cfg_attr(mutants, mutants::skip)]
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut stream) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            stream.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            super::console::stdout_line(format_args!("\n🛑 [Rullst Shutdown] Received SIGINT (Ctrl+C). Draining in-flight requests..."));
        },
        _ = terminate => {
            super::console::stdout_line(format_args!("\n🛑 [Rullst Shutdown] Received SIGTERM. Draining in-flight requests..."));
        },
    }
}

async fn shutdown_with_lifecycle<F>(shutdown: F, lifecycle: Option<ApplicationLifecycle>)
where
    F: std::future::Future<Output = ()>,
{
    shutdown.await;
    if let Some(lifecycle) = lifecycle {
        let _ = lifecycle.begin_draining();
    }
}

fn mark_lifecycle_ready(lifecycle: Option<&ApplicationLifecycle>) -> Result<(), ServerError> {
    match lifecycle {
        Some(lifecycle) => lifecycle.mark_ready().map_err(ServerError::from),
        None => Ok(()),
    }
}

fn mark_lifecycle_stopped(lifecycle: Option<&ApplicationLifecycle>) {
    if let Some(lifecycle) = lifecycle {
        lifecycle.mark_stopped();
    }
}

/// Parses dotenv content with errors that never contain file content: dotenvy's
/// own parse error embeds the unparsed remainder, which can include secrets.
pub(super) fn parse_dotenv(content: &str) -> Result<HashMap<String, String>, ServerError> {
    let mut values = HashMap::new();
    for (index, entry) in dotenvy::from_read_iter(content.as_bytes()).enumerate() {
        let (name, value) = entry.map_err(|error| {
            ServerError::Configuration(match error {
                dotenvy::Error::LineParse(..) => {
                    format!("invalid .env syntax in entry {}", index + 1)
                }
                dotenvy::Error::Io(error) => format!("failed to read .env: {}", error.kind()),
                _ => "invalid .env file".to_string(),
            })
        })?;
        values.insert(name, value);
    }
    Ok(values)
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
