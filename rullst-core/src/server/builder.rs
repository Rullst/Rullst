use crate::Router;
use crate::lifecycle::{ApplicationLifecycle, apply_lifecycle};
use crate::scheduler::{Scheduler, SchedulerHandle};
use crate::server::dylib_loader::load_dylib_router;
use crate::server::hotswap::HotSwapService;
use crate::server::server_middleware::zstd_static_middleware;
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
        }
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
    pub fn shield(mut self, shield: crate::resilience::TrafficShield) -> Self {
        self.shield = Some(shield);
        self
    }

    /// Attaches a global RateLimiter to the server.
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
        let _ = crate::artisan::check_and_run_artisan(vec![], vec![]).await;
        let _ = crate::telemetry::init_telemetry();
        let app_config = Self::load_config().await?;
        let environment = resolve_environment(&app_config, &dotenv)?;

        self.init_database(&app_config, &dotenv).await?;
        let addr = Self::setup_networking(port, app_config.app.port, environment, &dotenv)?;
        let scheduler_handle = self.start_scheduler()?;
        let shield_lifecycle = self.start_traffic_shield()?;

        let server_result = if let Some(lib_path) = self.hot_reload_lib.take() {
            self.run_hot_reload(lib_path, addr, environment, shutdown)
                .await
        } else {
            self.run_static(app_config, addr, environment, shutdown)
                .await
        };

        if let Some(shield) = shield_lifecycle {
            shield.shutdown();
        }
        let scheduler_result = match scheduler_handle {
            Some(handle) => handle.shutdown().await.map_err(ServerError::from),
            None => Ok(()),
        };

        match server_result {
            Err(error) => Err(error),
            Ok(()) => scheduler_result,
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_dotenv_values() -> Result<HashMap<String, String>, ServerError> {
        if !std::path::Path::new(".env").exists() {
            return Ok(HashMap::new());
        }

        let content = tokio::fs::read_to_string(".env").await?;
        dotenvy::from_read_iter(content.as_bytes())
            .map(|entry| entry.map_err(|error| ServerError::Configuration(error.to_string())))
            .collect()
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_config() -> Result<crate::config::RullstConfig, ServerError> {
        let app_config = if std::path::Path::new("Rullst.toml").exists() {
            crate::config::RullstConfig::load_from_file("Rullst.toml")
                .await
                .map_err(|error| ServerError::Configuration(error.to_string()))?
        } else {
            crate::config::RullstConfig::new()
        };

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
            if let Some(env_db_url) = read_optional_environment_variable("DATABASE_URL")? {
                self.db_url = Some(env_db_url);
            } else if let Some(dotenv_db_url) = dotenv.get("DATABASE_URL") {
                self.db_url = Some(dotenv_db_url.clone());
            } else if let Some(ref url) = app_config.database.url {
                self.db_url = Some(url.clone());
            }
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
        let database_requested = self.db_url.is_some()
            || app_config.database.url.is_some()
            || dotenv.contains_key("DATABASE_URL")
            || read_optional_environment_variable("DATABASE_URL")?.is_some();

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

        if environment.allows_development_tools() && addr.ip().is_unspecified() {
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
            shield: self.shield,
            limiter: self.limiter,
            lifecycle: self.lifecycle.clone(),
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
        let result = axum::serve(listener, hotswap_service)
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
        let is_dev = environment.allows_development_tools();
        let mut app = self.router.into_axum();

        app = app.layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let method = req.method().to_string();
                let path = req.uri().path().to_string();
                let start = std::time::Instant::now();
                let res = next.run(req).await;
                let status = res.status().as_u16();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                if !path.starts_with("/_rullst_hmr") {
                    println!(
                        "[HTTP] {} {} -> {} ({:.2} ms)",
                        method, path, status, elapsed
                    );
                }
                res
            },
        ));

        if std::path::Path::new("static").exists() {
            app = app
                .nest_service(
                    "/static",
                    tower_http::services::ServeDir::new("static").precompressed_br(),
                )
                .layer(axum::middleware::from_fn(zstd_static_middleware));
        }

        if is_dev {
            app = app
                .route(
                    "/_rullst/explain",
                    axum::routing::get(crate::error_console::handle_explain),
                )
                .route(
                    "/_rullst/autofix",
                    axum::routing::post(crate::error_console::handle_autofix),
                )
                .layer(axum::middleware::from_fn(
                    crate::error_console::catch_panic_middleware,
                ));
        }

        if let Some(limiter) = self.limiter {
            app = app.layer(axum::middleware::from_fn(move |req, next| {
                crate::resilience::rate_limit_middleware(limiter.clone(), req, next)
            }));
        }

        if let Some(shield) = self.shield {
            app = app.layer(axum::middleware::from_fn(move |req, next| {
                crate::resilience::backpressure_middleware(shield.clone(), req, next)
            }));
        }

        if let Some(lifecycle) = self.lifecycle.clone() {
            app = apply_lifecycle(app, lifecycle);
        }

        app = crate::security::apply_security_baseline(app, app_config.security, environment)
            .map_err(|error| ServerError::Configuration(error.to_string()))?;

        println!("Rullst framework serving on http://{}", addr);
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        mark_lifecycle_ready(self.lifecycle.as_ref())?;
        let lifecycle = self.lifecycle.clone();
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

fn read_optional_environment_variable(name: &str) -> Result<Option<String>, ServerError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ServerError::Configuration(format!(
            "{name} is not valid Unicode"
        ))),
    }
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
    let rullst_env = read_optional_environment_variable("RULLST_ENV")?;
    let app_env = read_optional_environment_variable("APP_ENV")?;
    let fallback = dotenv
        .get("RULLST_ENV")
        .or_else(|| dotenv.get("APP_ENV"))
        .map(String::as_str)
        .or(config.app.env.as_deref());

    crate::config::Environment::resolve(rullst_env.as_deref(), app_env.as_deref(), fallback)
        .map_err(|error| ServerError::Configuration(error.to_string()))
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
            println!("\n🛑 [Rullst Shutdown] Received SIGINT (Ctrl+C). Draining in-flight requests...");
        },
        _ = terminate => {
            println!("\n🛑 [Rullst Shutdown] Received SIGTERM. Draining in-flight requests...");
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

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
