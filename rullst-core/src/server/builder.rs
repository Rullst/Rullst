use crate::Router;
use crate::scheduler::Scheduler;
use crate::server::dylib_loader::load_dylib_router;
use crate::server::hotswap::HotSwapService;
use crate::server::server_middleware::zstd_static_middleware;
use rullst_orm::Orm;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

#[non_exhaustive]
/// The central application server builder for Rullst.
///
/// Configures and boots the Axum HTTP server, ORM connection pool, task scheduler,
/// hot-reload DLL watcher, traffic shield, and rate limiter in a single fluent chain.
///
/// # Example
/// ```rust,ignore
/// use rullst::{Server, routes, routing::get};
///
/// #[tokio::main]
/// async fn main() {
///     Server::new(routes![get("/" => || async { "OK" })])
///         .with_db("sqlite://app.db")
///         .run(3000)
///         .await
///         .unwrap();
/// }
/// ```
pub struct Server {
    pub(crate) router: Router,
    pub(crate) db_url: Option<String>,
    pub(crate) scheduler: Option<Scheduler>,
    pub(crate) hot_reload_lib: Option<String>,
    pub(crate) shield: Option<crate::resilience::TrafficShield>,
    pub(crate) limiter: Option<crate::resilience::RateLimiter>,
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
        }
    }

    /// Creates a `Server` in **hot-reload** mode that loads the application router from
    /// a compiled `cdylib` dynamic library at the given `lib_path`.
    /// The background file-watcher recompiles and hot-swaps the router on source changes.
    #[allow(clippy::panic)]
    pub fn new_hot<S: Into<String>>(lib_path: S) -> Self {
        if !cfg!(debug_assertions) {
            panic!(
                "CRITICAL SECURITY: Hot-Reloading (new_hot) is strictly disabled in release mode to prevent RCE vulnerabilities via dynamic library injection."
            );
        }

        Server {
            router: Router::new(),
            db_url: None,
            scheduler: None,
            hot_reload_lib: Some(lib_path.into()),
            shield: None,
            limiter: None,
        }
    }

    /// Set a database URL to automatically initialize the Orm connection pool at startup
    pub fn with_db<S: Into<String>>(mut self, db_url: S) -> Self {
        self.db_url = Some(db_url.into());
        self
    }

    /// Attach a task scheduler that runs alongside the HTTP server.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rullst::scheduler::Scheduler;
    ///
    /// let scheduler = Scheduler::new()
    ///     .task("0 0 * * *", || async { cleanup().await });
    ///
    /// Server::new(router)
    ///     .schedule(scheduler)
    ///     .run(3000)
    ///     .await?;
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

    /// Start the HTTP server on the specified port
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run(mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let _ = crate::artisan::check_and_run_artisan(vec![], vec![]).await;
        let _ = crate::telemetry::init_telemetry();
        let app_config = Self::load_config().await;

        self.init_database(&app_config).await;
        self.start_scheduler();

        let is_dev =
            std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
        let addr = Self::setup_networking(port, is_dev);

        if let Some(lib_path) = self.hot_reload_lib.take() {
            self.run_hot_reload(lib_path, addr, is_dev).await
        } else {
            self.run_static(app_config, addr, is_dev).await
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_config() -> crate::config::RullstConfig {
        let mut app_config = crate::config::RullstConfig::new();
        if std::path::Path::new("Rullst.toml").exists() {
            match crate::config::RullstConfig::load_from_file("Rullst.toml").await {
                Ok(c) => {
                    let _ = crate::config::RullstConfig::set_global(c.clone());
                    app_config = c;
                }
                Err(e) => {
                    eprintln!("⚠️ Rullst Warning: Failed to parse Rullst.toml: {}", e);
                    let _ = crate::config::RullstConfig::set_global(app_config.clone());
                }
            }
        } else {
            let _ = crate::config::RullstConfig::set_global(app_config.clone());
        }
        app_config
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn init_database(&mut self, app_config: &crate::config::RullstConfig) {
        if rullst_orm::Orm::try_pool().is_ok() {
            return;
        }

        let _ = dotenvy::from_filename_override(".env");
        let _ = dotenvy::dotenv();

        if self.db_url.is_none() {
            if let Ok(env_db_url) = std::env::var("DATABASE_URL") {
                self.db_url = Some(env_db_url);
            } else if let Some(ref url) = app_config.database.url {
                self.db_url = Some(url.clone());
            }
        }

        if let Some(db_url) = &self.db_url {
            println!("Initializing Orm database pool...");
            match Orm::init(db_url).await {
                Ok(_) => println!("Database initialized successfully."),
                Err(e) => eprintln!(
                    "⚠️ Rullst Warning: Failed to initialize database: {}. Database features will be offline.",
                    e
                ),
            }
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn start_scheduler(&mut self) {
        if let Some(scheduler) = self.scheduler.take() {
            scheduler.start();
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn setup_networking(port: u16, is_dev: bool) -> SocketAddr {
        if is_dev && std::env::var("RUST_BACKTRACE").is_err() {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
            }
        }

        let host_str = std::env::var("HOST").unwrap_or_else(|_| {
            if is_dev && std::env::var("RULLST_HOST").is_err() {
                "127.0.0.1".to_string()
            } else {
                "0.0.0.0".to_string()
            }
        });

        let env_port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(port);

        let addr: SocketAddr = format!("{}:{}", host_str, env_port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], env_port)));

        if is_dev && addr.ip().is_unspecified() {
            eprintln!(
                "⚠️  Rullst Dev: Self-Healing Console mounted on /_rullst/*\n\
                   Set APP_ENV=production to disable before deploying."
            );
        }

        addr
    }

    #[allow(clippy::panic)]
    #[cfg_attr(mutants, mutants::skip)]
    async fn run_hot_reload(
        self,
        lib_path: String,
        addr: SocketAddr,
        is_dev: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !cfg!(debug_assertions) {
            panic!("CRITICAL SECURITY: Hot-Reloading is strictly disabled in release mode!");
        }

        println!("\x1b[36m⚡ Inicializando Rullst em Modo Hot-Reloading via dylib...\x1b[0m");
        println!("\x1b[36m⚡ Initializing Rullst in Hot-Reloading mode via dylib...\x1b[0m");

        let (initial_router, library) = match load_dylib_router(&lib_path, is_dev) {
            Ok(r) => r,
            Err(e) => {
                println!(
                    "\x1b[31m❌ Failed to load initial dylib: {}. Make sure the dynamic library was compiled by running 'cargo build --lib'.\x1b[0m",
                    e
                );
                return Err(e);
            }
        };

        let current_router = Arc::new(RwLock::new(initial_router));
        let active_libraries = Arc::new(Mutex::new(vec![library]));

        let hotswap_service = HotSwapService {
            current_router: current_router.clone(),
            active_libraries: active_libraries.clone(),
            lib_path: lib_path.clone(),
            is_dev,
            shield: self.shield,
            limiter: self.limiter,
        };

        println!(
            "Rullst framework serving on http://{} (Hot-Reload Ativo via CLI WebSocket)",
            addr
        );
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, hotswap_service).await?;

        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn run_static(
        self,
        app_config: crate::config::RullstConfig,
        addr: SocketAddr,
        is_dev: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut app = self
            .router
            .into_axum()
            .merge(crate::scalar::scalar_docs_router("/openapi.json"));

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

        app = app.layer(axum::Extension(app_config.security.clone()));

        if !app_config.security.cors_allow_origins.is_empty() {
            use tower_http::cors::CorsLayer;
            let origins: Vec<axum::http::HeaderValue> = app_config
                .security
                .cors_allow_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            app = app.layer(CorsLayer::new().allow_origin(origins));
        }

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

        if !is_dev {
            if app_config.security.enable_pii_masking {
                app = app.layer(axum::middleware::from_fn(
                    crate::security::pii_masking_middleware,
                ));
            }
            app = app
                .layer(axum::middleware::from_fn(
                    crate::security::headers_middleware,
                ))
                .layer(axum::middleware::from_fn(crate::security::csrf_middleware))
                .layer(axum::middleware::from_fn(crate::security::waf_middleware));
        }

        println!("Rullst framework serving on http://{}", addr);
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
