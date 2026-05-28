use crate::Router;
use crate::scheduler::Scheduler;
use rust_eloquent::Eloquent;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use tower_service::Service;

#[non_exhaustive]
pub struct Server {
    router: Router,
    db_url: Option<String>,
    scheduler: Option<Scheduler>,
    hot_reload_lib: Option<String>,
    shield: Option<crate::resilience::TrafficShield>,
    limiter: Option<crate::resilience::RateLimiter>,
}

impl Server {
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

    pub fn new_hot<S: Into<String>>(lib_path: S) -> Self {
        Server {
            router: Router::new(),
            db_url: None,
            scheduler: None,
            hot_reload_lib: Some(lib_path.into()),
            shield: None,
            limiter: None,
        }
    }

    /// Set a database URL to automatically initialize the Eloquent connection pool at startup
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
    pub async fn run(mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        if self.db_url.is_none()
            && let Ok(toml_content) = std::fs::read_to_string("Rullst.toml")
        {
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url")
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    self.db_url = Some(val.trim().trim_matches('"').to_string());
                }
            }
        }

        if let Some(db_url) = self.db_url {
            println!("Initializing Eloquent database pool...");
            match Eloquent::init(&db_url).await {
                Ok(_) => println!("Database initialized successfully."),
                Err(e) => eprintln!(
                    "⚠️ Rullst Warning: Failed to initialize database: {}. Database features will be offline.",
                    e
                ),
            }
        }

        // Start the scheduler if one was attached
        if let Some(scheduler) = self.scheduler.take() {
            scheduler.start();
        }

        let is_dev =
            std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
        if is_dev && std::env::var("RUST_BACKTRACE").is_err() {
            eprintln!(
                "⚠️  Rullst Dev: Set RUST_BACKTRACE=1 in your environment for richer error traces."
            );
        }

        let host = if is_dev && std::env::var("RULLST_HOST").is_err() {
            [127, 0, 0, 1]
        } else {
            [0, 0, 0, 0]
        };
        let addr = SocketAddr::from((host, port));

        // I-1: Warn when dev-only routes are exposed on a non-loopback address
        if is_dev && addr.ip().is_unspecified() {
            eprintln!(
                "⚠️  Rullst Dev: Self-Healing Console mounted on /_rullst/*\n\
                   Set APP_ENV=production to disable before deploying."
            );
        }

        if let Some(lib_path) = self.hot_reload_lib {
            // --- Hot Reloading Mode ---
            println!("\x1b[36m⚡ Inicializando Rullst em Modo Hot-Reloading via dylib...\x1b[0m");

            // Load initial router and library
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

            // Setup file watcher
            let (tx, rx) = std::sync::mpsc::channel();
            use notify::{RecommendedWatcher, RecursiveMode, Watcher};
            let mut watcher = RecommendedWatcher::new(
                move |res| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                notify::Config::default(),
            )?;

            if std::path::Path::new("src").exists() {
                watcher.watch(std::path::Path::new("src"), RecursiveMode::Recursive)?;
            }

            // Spawn dynamic loader watch thread
            let current_router_clone = current_router.clone();
            let active_libraries_clone = active_libraries.clone();
            let lib_path_clone = lib_path.clone();

            std::thread::spawn(move || {
                let mut last_build = std::time::Instant::now();
                while let Ok(_event) = rx.recv() {
                    // Debounce
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    while rx.try_recv().is_ok() {}

                    if last_build.elapsed() < std::time::Duration::from_secs(1) {
                        continue;
                    }

                    let (tx, rx_build) = std::sync::mpsc::channel();
                    let current_dir =
                        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    std::thread::spawn(move || {
                        let res = std::process::Command::new("cargo")
                            .arg("build")
                            .arg("--lib")
                            .current_dir(current_dir)
                            .status();
                        let _ = tx.send(res);
                    });

                    // Timeout of 120 seconds to prevent hanging build processes
                    let build_success = match rx_build
                        .recv_timeout(std::time::Duration::from_secs(120))
                    {
                        Ok(Ok(status)) => status.success(),
                        Ok(Err(e)) => {
                            eprintln!("⚠️ Rullst Hot-Reload: failed to execute cargo build: {}", e);
                            false
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            eprintln!(
                                "⚠️ Rullst Hot-Reload: cargo build timed out after 120 seconds!"
                            );
                            false
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => false,
                    };

                    if build_success {
                        println!(
                            "\x1b[32m✨ Rullst Hot-Reload: Recompilado com sucesso! Carregando dylib...\x1b[0m"
                        );
                        match load_dylib_router(&lib_path_clone, is_dev) {
                            Ok((new_router, new_lib)) => {
                                // H-1: Recover from poisoned write lock rather than panicking
                                match current_router_clone.write() {
                                    Ok(mut guard) => *guard = new_router,
                                    Err(poisoned) => *poisoned.into_inner() = new_router,
                                };

                                // Keep all loaded libraries in memory to prevent segmentation faults
                                // for concurrent requests executing handlers from older versions
                                let mut active_libs = active_libraries_clone
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner());
                                active_libs.push(new_lib);

                                println!(
                                    "\x1b[32m🚀 Rullst Hot-Reload: Roteamento atualizado e hot-swapped instantaneamente!\x1b[0m"
                                );
                            }
                            Err(e) => {
                                println!(
                                    "\x1b[31m❌ Rullst Hot-Reload: Erro ao carregar dylib recém-compilada: {}\x1b[0m",
                                    e
                                );
                            }
                        }
                    } else {
                        println!(
                            "\x1b[31m❌ Rullst Hot-Reload: Falha ao compilar o código fonte. Corrija os erros para aplicar o hot-swap.\x1b[0m"
                        );
                    }

                    last_build = std::time::Instant::now();
                }
            });

            let hotswap_service = HotSwapService {
                current_router,
                shield: self.shield.clone(),
                limiter: self.limiter.clone(),
            };

            println!(
                "Rullst framework serving on http://{} (Hot-Reload Ativo)",
                addr
            );
            println!(
                "🚀 Visit: http://localhost:{} to see the result!",
                addr.port()
            );

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, hotswap_service).await?;
        } else {
            // --- Standard Static Routing Mode ---
            let mut app = self.router.into_axum();

            // Serve static files from "static" directory if it exists
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

            println!("Rullst framework serving on http://{}", addr);
            println!(
                "🚀 Visit: http://localhost:{} to see the result!",
                addr.port()
            );

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct HotSwapService {
    current_router: Arc<RwLock<axum::Router>>,
    shield: Option<crate::resilience::TrafficShield>,
    limiter: Option<crate::resilience::RateLimiter>,
}

impl<'a, L: axum::serve::Listener> Service<axum::serve::IncomingStream<'a, L>> for HotSwapService {
    type Response = HotSwapService;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

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

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
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
                Ok(Err(_)) => {
                    // H-2: Handle oneshot error gracefully
                    Ok(axum::response::Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(axum::body::Body::empty())
                        .unwrap())
                }
                Err(join_err) => {
                    // I-2: If a panic occurred during the oneshot invocation, catch it and present the Self-Healing Console
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
                    let html_content =
                        crate::error_console::render_console_html(&message, &backtrace).await;

                    Ok(axum::response::Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .header(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
                        .body(axum::body::Body::from(html_content))
                        .unwrap())
                }
            }
        })
    }
}

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
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(filename)
                    && name.contains("_active_")
                    && name.ends_with(lib_extension)
                {
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
    let lib = unsafe { libloading::Library::new(temp_path)? };
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
            ));
    }

    Ok((axum_router, lib))
}

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
                    if tokio::fs::metadata(&local_path_str).await.map(|m| m.is_file()).unwrap_or(false) {
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

#[cfg(test)]
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
}
