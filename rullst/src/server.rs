pub use crate::Router;
use crate::scheduler::Scheduler;
use rullst_orm::Orm;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use tower_service::Service;

#[non_exhaustive]
/// The central application server builder for Rullst.
///
/// Configures and boots the Axum HTTP server, ORM connection pool, task scheduler,
/// hot-reload DLL watcher, traffic shield, and rate limiter in a single fluent chain.
///
/// # Example
/// ```rust,no_run
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
    router: Router,
    db_url: Option<String>,
    scheduler: Option<Scheduler>,
    hot_reload_lib: Option<String>,
    shield: Option<crate::resilience::TrafficShield>,
    limiter: Option<crate::resilience::RateLimiter>,
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
            eprintln!(
                "⚠️  Rullst Dev: Set RUST_BACKTRACE=1 in your environment for richer error traces."
            );
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

        let (initial_router, library) = match load_dylib_router(&lib_path, is_dev) {
            Ok(r) => r,
            Err(e) => {
                println!(
                    "\x1b[31m❌ Falha ao carregar dylib inicial: {}. Certifique-se de que a biblioteca dinâmica foi compilada rodando 'cargo build --lib'.\x1b[0m",
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
        let mut app = self.router.into_axum();

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

/// Tower service that atomically swaps the Axum router at runtime during hot-reload development.
/// Wraps the router in an `Arc<RwLock<>>` so handlers continue serving in-flight requests
/// while the new router is being compiled and installed.
#[derive(Clone)]
pub struct HotSwapService {
    current_router: Arc<RwLock<axum::Router>>,
    active_libraries: Arc<Mutex<Vec<libloading::Library>>>,
    lib_path: String,
    is_dev: bool,
    shield: Option<crate::resilience::TrafficShield>,
    limiter: Option<crate::resilience::RateLimiter>,
}

impl HotSwapService {
    #[cfg_attr(mutants, mutants::skip)]
    fn handle_oneshot_error() -> Result<axum::response::Response, std::convert::Infallible> {
        match axum::response::Response::builder()
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::empty())
        {
            Ok(res) => Ok(res),
            Err(_) => {
                let mut res = axum::response::Response::new(axum::body::Body::empty());
                *res.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(res)
            }
        }
    }

    async fn handle_panic_error(
        join_err: tokio::task::JoinError,
    ) -> Result<axum::response::Response, std::convert::Infallible> {
        let message = if join_err.is_panic() {
            let panic_payload = join_err.into_panic();
            if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unhandled application panic".to_string()
            }
        } else {
            "Request task was cancelled or aborted".to_string()
        };

        let backtrace = std::backtrace::Backtrace::capture();
        let html_content = crate::error_console::render_console_html(&message, &backtrace).await;

        match axum::response::Response::builder()
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(axum::body::Body::from(html_content))
        {
            Ok(res) => Ok(res),
            Err(_) => Self::handle_oneshot_error(),
        }
    }
}

impl<'a, L: axum::serve::Listener> Service<axum::serve::IncomingStream<'a, L>> for HotSwapService {
    type Response = HotSwapService;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    #[cfg_attr(mutants, mutants::skip)]
    #[cfg_attr(mutants, mutants::skip)]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: axum::serve::IncomingStream<'a, L>) -> Self::Future {
        std::future::ready(Ok(self.clone()))
    }
}

impl Service<axum::extract::Request> for HotSwapService {
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[cfg_attr(mutants, mutants::skip)]
    #[cfg_attr(mutants, mutants::skip)]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
        if req.uri().path() == "/_rullst/internal/reload_dylib"
            && req.method() == axum::http::Method::POST
        {
            let lib_path = self.lib_path.clone();
            let is_dev = self.is_dev;
            let current_router = self.current_router.clone();
            let active_libraries = self.active_libraries.clone();

            return Box::pin(async move {
                match load_dylib_router(&lib_path, is_dev) {
                    Ok((new_router, new_lib)) => {
                        match current_router.write() {
                            Ok(mut guard) => *guard = new_router,
                            Err(poisoned) => *poisoned.into_inner() = new_router,
                        };

                        let mut active_libs =
                            active_libraries.lock().unwrap_or_else(|p| p.into_inner());
                        active_libs.push(new_lib);
                        if active_libs.len() > 3 {
                            active_libs.remove(0);
                        }

                        println!(
                            "\x1b[32m🚀 Rullst Hot-Reload: Dylib swapped instantly via webhook!\x1b[0m"
                        );

                        let res = axum::response::Response::builder()
                            .status(axum::http::StatusCode::OK)
                            .body(axum::body::Body::from("Swapped"))
                            .unwrap_or_else(|_| {
                                axum::response::Response::new(axum::body::Body::empty())
                            });
                        Ok(res)
                    }
                    Err(e) => {
                        eprintln!(
                            "\x1b[31m❌ Rullst Hot-Reload: Error loading new dylib: {}\x1b[0m",
                            e
                        );
                        let res = axum::response::Response::builder()
                            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(axum::body::Body::from(e.to_string()))
                            .unwrap_or_else(|_| {
                                axum::response::Response::new(axum::body::Body::empty())
                            });
                        Ok(res)
                    }
                }
            });
        }
        // H-1: Recover from poisoned RwLock instead of panicking
        let mut router = match self.current_router.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        if let Some(ref limiter) = self.limiter {
            let lim = limiter.clone();
            router = router.layer(axum::middleware::from_fn(move |req, next| {
                crate::resilience::rate_limit_middleware(lim.clone(), req, next)
            }));
        }

        if let Some(ref shield) = self.shield {
            let sh = shield.clone();
            router = router.layer(axum::middleware::from_fn(move |req, next| {
                crate::resilience::backpressure_middleware(sh.clone(), req, next)
            }));
        }
        use tower::ServiceExt;
        let fut = router.oneshot(req);
        Box::pin(async move {
            let handle = tokio::spawn(async move { fut.await });
            match handle.await {
                Ok(Ok(res)) => Ok(res),
                Ok(Err(_)) => Self::handle_oneshot_error(),
                Err(join_err) => Self::handle_panic_error(join_err).await,
            }
        })
    }
}

#[cfg_attr(mutants, mutants::skip)]
#[cfg_attr(mutants, mutants::skip)]
fn load_dylib_router(
    lib_path: &str,
    is_dev: bool,
) -> Result<(axum::Router, libloading::Library), Box<dyn std::error::Error>> {
    let lib_extension = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let full_lib_path = if lib_path.ends_with(".dll")
        || lib_path.ends_with(".so")
        || lib_path.ends_with(".dylib")
    {
        lib_path.to_string()
    } else {
        format!("{}.{}", lib_path, lib_extension)
    };

    let path_buf = std::path::Path::new(&full_lib_path);
    if !path_buf.exists() {
        return Err(format!("Dylib not found at: {}", full_lib_path).into());
    }

    let parent = path_buf.parent().unwrap_or(std::path::Path::new("."));
    let filename = path_buf
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Caminho da dylib inválido ou caracteres não UTF-8 detectados",
            )
        })?;

    // Clean up older active files that are no longer locked by any process
    let expected_prefix = format!("{}_active_", filename);
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&expected_prefix) && name.ends_with(lib_extension) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    // L-1: Use UUID v4 instead of nanosecond timestamp to guarantee filename uniqueness
    //       even on systems with low-resolution clocks.
    let unique_id = uuid::Uuid::new_v4().as_simple().to_string();
    let temp_filename = format!("{}_active_{}.{}", filename, unique_id, lib_extension);
    let temp_path = parent.join(temp_filename);

    // Copy the dylib file to prevent locking issues
    std::fs::copy(&full_lib_path, &temp_path)?;

    // SAFETY: The code below performs dynamic library loading and raw pointer extraction.
    // Invariant requirements for safety:
    //  - The dynamically loaded library must expose a symbol `rullst_router_init`
    //    with signature `unsafe extern "C" fn() -> *mut Router`.
    //  - The `rullst_router_init` function must allocate a `Box<Router>` and return
    //    the raw pointer produced by `Box::into_raw`. The caller here uses
    //    `Box::from_raw` to take ownership of that pointer; therefore the
    //    library MUST NOT keep or free that pointer after returning it.
    //  - The loaded library must be ABI-compatible with the host Router layout.
    //  - The `temp_path` copy must remain available on disk for the lifetime of
    //    any references into the loaded library. We keep the returned `Library`
    //    object to ensure the library remains mapped until dropped by the caller.
    //  - Calls into the plugin must be synchronized appropriately by the host if
    //    the plugin is not internally thread-safe.
    //
    // This block is `unsafe` because it relies on the above invariants; any
    // future changes to router ABI or plugin implementations must be reflected
    // here and documented. Review and audit this section when upgrading
    // `libloading`, `Router` types, or changing the plugin API.
    let lib = unsafe { libloading::Library::new(&temp_path)? };
    if let Err(e) = std::fs::remove_file(&temp_path) {
        #[cfg(not(target_os = "windows"))]
        eprintln!(
            "⚠️ Rullst: failed to remove temporary dylib file at {:?}: {}",
            temp_path, e
        );
        #[cfg(target_os = "windows")]
        {
            // On Windows, sharing violation (error code 32) is normal, so we only log other errors.
            if e.raw_os_error() != Some(32) {
                eprintln!(
                    "⚠️ Rullst: failed to remove temporary dylib file at {:?}: {}",
                    temp_path, e
                );
            }
        }
    }
    let init_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut Router> =
        unsafe { lib.get(b"rullst_router_init")? };
    let router_ptr = unsafe { init_fn() };

    // Convert *mut Router back to Router box and extract it
    let rullst_router = unsafe { *Box::from_raw(router_ptr) };

    // Convert Rullst Router to Axum Router
    let mut axum_router = rullst_router.into_axum();

    // Serve static files from "static" directory if it exists
    if std::path::Path::new("static").exists() {
        axum_router = axum_router
            .nest_service(
                "/static",
                tower_http::services::ServeDir::new("static").precompressed_br(),
            )
            .layer(axum::middleware::from_fn(zstd_static_middleware));
    }

    // Attach development explain / console routes
    if is_dev {
        axum_router = axum_router
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
            ))
            .layer(axum::middleware::from_fn(inject_hmr_script));
    }

    Ok((axum_router, lib))
}

async fn inject_hmr_script(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let res = next.run(req).await;

    if let Some(content_type) = res.headers().get(axum::http::header::CONTENT_TYPE) {
        if content_type.to_str().unwrap_or("").contains("text/html") {
            let (mut parts, body) = res.into_parts();
            if let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await {
                let mut html = String::from_utf8_lossy(&bytes).to_string();

                let port = std::env::var("PORT")
                    .ok()
                    .and_then(|p| p.parse::<u16>().ok())
                    .unwrap_or(3000);
                let ws_port = port + 1;

                let script = format!(
                    r#"
<!-- Rullst Hybrid Hot-Reloading -->
<script src="https://unpkg.com/morphdom@2.7.4/dist/morphdom-umd.js"></script>
<script>
    (function() {{
        const ws = new WebSocket("ws://localhost:{}/_rullst_hmr");
        ws.onmessage = (e) => {{
            const data = JSON.parse(e.data);
            if (data.type === "UI_UPDATE") {{
                fetch(window.location.href)
                    .then(r => r.text())
                    .then(newHtml => {{
                        const parser = new DOMParser();
                        const doc = parser.parseFromString(newHtml, 'text/html');
                        morphdom(document.body, doc.body, {{
                            onBeforeElUpdated: function(fromEl, toEl) {{
                                if (fromEl.isEqualNode(toEl)) return false;
                                return true;
                            }}
                        }});
                    }});
            }}
        }};
    }})();
</script>
"#,
                    ws_port
                );
                if let Some(idx) = html.rfind("</body>") {
                    html.insert_str(idx, &script);
                } else {
                    html.push_str(&script);
                }

                parts.headers.remove(axum::http::header::CONTENT_LENGTH);
                return axum::response::Response::from_parts(parts, axum::body::Body::from(html));
            } else {
                // If we failed to read bytes, just return empty body or we could somehow return the parts?
                // We'll just return it as empty for now, or preferably panic since it's dev mode.
                return axum::response::Response::from_parts(parts, axum::body::Body::empty());
            }
        }
    }

    res
}

#[cfg_attr(mutants, mutants::skip)]
#[cfg_attr(mutants, mutants::skip)]
async fn zstd_static_middleware(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    if path.starts_with("/static/") {
        if let Some(accept_encoding) = req.headers().get(axum::http::header::ACCEPT_ENCODING) {
            if let Ok(accept_str) = accept_encoding.to_str() {
                if accept_str.contains("zstd") {
                    let local_path_str = format!("{}.zst", &path[1..]);
                    if tokio::fs::metadata(&local_path_str)
                        .await
                        .map(|m| m.is_file())
                        .unwrap_or(false)
                    {
                        let original_ext = std::path::Path::new(&path)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_string();

                        let new_uri = format!("{}.zst", path);
                        if let Ok(uri) = new_uri.parse::<axum::http::Uri>() {
                            *req.uri_mut() = uri;

                            let mut response = next.run(req).await;

                            response.headers_mut().insert(
                                axum::http::header::CONTENT_ENCODING,
                                axum::http::header::HeaderValue::from_static("zstd"),
                            );

                            let mime_type = match original_ext.as_str() {
                                "html" => "text/html; charset=utf-8",
                                "css" => "text/css; charset=utf-8",
                                "js" => "application/javascript; charset=utf-8",
                                "json" => "application/json; charset=utf-8",
                                "svg" => "image/svg+xml",
                                "wasm" => "application/wasm",
                                "xml" => "application/xml; charset=utf-8",
                                "txt" => "text/plain; charset=utf-8",
                                _ => "",
                            };

                            if !mime_type.is_empty() {
                                if let Ok(val) =
                                    axum::http::header::HeaderValue::from_str(mime_type)
                                {
                                    response
                                        .headers_mut()
                                        .insert(axum::http::header::CONTENT_TYPE, val);
                                }
                            }

                            return response;
                        }
                    }
                }
            }
        }
    }

    next.run(req).await
}

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────
pub use axum::{
    body::{Body, Bytes},
    extract::{Extension, Form, Json, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    middleware::{self, Next, from_fn},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, patch, post, put},
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::Router;
    use crate::scheduler::Scheduler;

    #[test]
    fn test_server_builder() {
        let router = Router::new();
        let server = Server::new(router).with_db("sqlite://test.db");

        assert_eq!(server.db_url, Some("sqlite://test.db".to_string()));
        assert!(server.scheduler.is_none());
    }

    #[test]
    fn test_server_scheduler_attach() {
        let router = Router::new();
        let scheduler = Scheduler::new();
        let server = Server::new(router).schedule(scheduler);

        assert!(server.scheduler.is_some());
    }

    #[tokio::test]
    async fn test_server_resilience_attach() {
        let router = Router::new();
        let shield = crate::resilience::TrafficShield::new(
            crate::resilience::TrafficShieldConfig::new().with_db_probe(false),
        );
        let limiter = crate::resilience::RateLimiter::new(
            crate::resilience::RateLimitConfig::per_second(10.0),
        );
        let server = Server::new(router).shield(shield).rate_limit(limiter);

        assert!(server.shield.is_some());
        assert!(server.limiter.is_some());
    }

    #[tokio::test]
    async fn test_hot_swap_service_call() {
        let router = axum::Router::new().route("/test", axum::routing::get(|| async { "swap ok" }));
        let current_router = Arc::new(RwLock::new(router));
        let mut service = HotSwapService {
            current_router,
            active_libraries: Arc::new(Mutex::new(vec![])),
            lib_path: "".to_string(),
            is_dev: false,
            shield: None,
            limiter: None,
        };

        use tower_service::Service;
        let req = axum::http::Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let res = service.call(req).await.unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        assert_eq!(body_bytes, "swap ok");
    }

    #[tokio::test]
    async fn test_hot_swap_service_panic() {
        async fn panic_handler() -> &'static str {
            panic!("Oops");
        }
        let router = axum::Router::new().route("/panic", axum::routing::get(panic_handler));
        let current_router = Arc::new(RwLock::new(router));
        let mut service = HotSwapService {
            current_router,
            active_libraries: Arc::new(Mutex::new(vec![])),
            lib_path: "".to_string(),
            is_dev: false,
            shield: None,
            limiter: None,
        };

        use tower_service::Service;
        let req = axum::http::Request::builder()
            .uri("/panic")
            .body(axum::body::Body::empty())
            .unwrap();

        let res = service.call(req).await.unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_hot_swap_service_poisoned_lock() {
        let router =
            axum::Router::new().route("/test", axum::routing::get(|| async { "recovered" }));
        let current_router = Arc::new(RwLock::new(router));
        // Poison the lock by panicking in a write guard thread
        let lock_clone = current_router.clone();
        let _ = std::thread::spawn(move || {
            let _guard = lock_clone.write().unwrap();
            panic!("poisoning lock");
        })
        .join();

        assert!(current_router.is_poisoned());

        let mut service = HotSwapService {
            current_router,
            active_libraries: Arc::new(Mutex::new(vec![])),
            lib_path: "".to_string(),
            is_dev: false,
            shield: None,
            limiter: None,
        };

        use tower_service::Service;
        let req = axum::http::Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let res = service.call(req).await.unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        let body_bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        assert_eq!(body_bytes, "recovered");
    }

    #[tokio::test]
    async fn test_hot_swap_service_reload_route() {
        // This test only verifies that the route matching works correctly,
        // we can't fully test dylib reloading here without complex setup.
        let router = axum::Router::new().route("/", axum::routing::get(|| async { "root" }));
        let current_router = Arc::new(RwLock::new(router));
        let mut service = HotSwapService {
            current_router: current_router.clone(),
            active_libraries: Arc::new(Mutex::new(vec![])),
            lib_path: "".to_string(),
            is_dev: true,
            shield: None,
            limiter: None,
        };

        use tower_service::Service;

        // 1. Valid request to reload (we expect a 500 error because the lib path is empty/invalid)
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/_rullst/internal/reload_dylib")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = service.call(req).await.unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        // 2. Invalid method
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/_rullst/internal/reload_dylib")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = service.call(req).await.unwrap();
        assert_ne!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR); // Will be 404 because not handled by HotSwapService reload block

        // 3. Invalid URI
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/_rullst/internal/other")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = service.call(req).await.unwrap();
        assert_ne!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_inject_hmr_script() {
        let router = axum::Router::new()
            .route(
                "/",
                axum::routing::get(|| async {
                    axum::response::Html("<html><body>Hello</body></html>")
                }),
            )
            .layer(axum::middleware::from_fn(inject_hmr_script));

        use tower_service::Service;
        let mut service = router;

        unsafe {
            std::env::set_var("PORT", "3000");
        }
        let req = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = service.call(req).await.unwrap();
        let body_bytes = axum::body::to_bytes(res.into_body(), 10240).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body_str.contains("Hello"));
        assert!(body_str.contains("Rullst Hybrid Hot-Reloading"));
        assert!(body_str.contains("ws://localhost:3001/_rullst_hmr"));
    }
}
