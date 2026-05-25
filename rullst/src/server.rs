use crate::Router;
use crate::scheduler::Scheduler;
use rust_eloquent::Eloquent;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, Mutex};
use tower_service::Service;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;


#[non_exhaustive]
pub struct Server {
    router: Router,
    db_url: Option<String>,
    scheduler: Option<Scheduler>,
    hot_reload_lib: Option<String>,
}

impl Server {
    pub fn new(router: Router) -> Self {
        Server {
            router,
            db_url: None,
            scheduler: None,
            hot_reload_lib: None,
        }
    }

    pub fn new_hot<S: Into<String>>(lib_path: S) -> Self {
        Server {
            router: Router::new(),
            db_url: None,
            scheduler: None,
            hot_reload_lib: Some(lib_path.into()),
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
            Eloquent::init(&db_url).await?;
            println!("Database initialized successfully.");
        }

        // Start the scheduler if one was attached
        if let Some(scheduler) = self.scheduler.take() {
            scheduler.start();
        }

        let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
        if is_dev && std::env::var("RUST_BACKTRACE").is_err() {
            eprintln!(
                "⚠️  Rullst Dev: Set RUST_BACKTRACE=1 in your environment for richer error traces."
            );
        }

        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        if let Some(lib_path) = self.hot_reload_lib {
            // --- Hot Reloading Mode ---
            println!("\x1b[36m⚡ Inicializando Rullst em Modo Hot-Reloading via dylib...\x1b[0m");

            // Load initial router and library
            let (initial_router, library) = match load_dylib_router(&lib_path, is_dev) {
                Ok(r) => r,
                Err(e) => {
                    println!("\x1b[31m❌ Falha ao carregar dylib inicial: {}. Certifique-se de que a biblioteca dinâmica foi compilada rodando 'cargo build --lib'.\x1b[0m", e);
                    return Err(e);
                }
            };

            let current_router = Arc::new(RwLock::new(initial_router));
            let active_library = Arc::new(Mutex::new(Some(library)));

            // Setup file watcher
            let (tx, rx) = std::sync::mpsc::channel();
            use notify::{Watcher, RecommendedWatcher, RecursiveMode};
            let mut watcher = RecommendedWatcher::new(move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            }, notify::Config::default())?;

            if std::path::Path::new("src").exists() {
                watcher.watch(std::path::Path::new("src"), RecursiveMode::Recursive)?;
            }

            // Spawn dynamic loader watch thread
            let current_router_clone = current_router.clone();
            let active_library_clone = active_library.clone();
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

                    println!("\x1b[33m🔄 Rullst Hot-Reload: Alteração detectada em 'src/'. Recompilando dylib...\x1b[0m");

                    let build_status = std::process::Command::new("cargo")
                        .arg("build")
                        .arg("--lib")
                        .status();

                    if let Ok(status) = build_status && status.success() {
                        println!("\x1b[32m✨ Rullst Hot-Reload: Recompilado com sucesso! Carregando dylib...\x1b[0m");
                        match load_dylib_router(&lib_path_clone, is_dev) {
                            Ok((new_router, new_lib)) => {
                                // Swap router under lock
                                *current_router_clone.write().unwrap() = new_router;
                                
                                // Swap library under lock to keep memory mapped safely
                                let mut active_lib = active_library_clone.lock().unwrap();
                                *active_lib = Some(new_lib); // Old library is safely dropped here!

                                println!("\x1b[32m🚀 Rullst Hot-Reload: Roteamento atualizado e hot-swapped instantaneamente!\x1b[0m");
                            }
                            Err(e) => {
                                println!("\x1b[31m❌ Rullst Hot-Reload: Erro ao carregar dylib recém-compilada: {}\x1b[0m", e);
                            }
                        }
                    } else {
                        println!("\x1b[31m❌ Rullst Hot-Reload: Falha ao compilar o código fonte. Corrija os erros para aplicar o hot-swap.\x1b[0m");
                    }

                    last_build = std::time::Instant::now();
                }
            });

            let hotswap_service = HotSwapService { current_router };
            println!("Rullst framework serving on http://{} (Hot-Reload Ativo)", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, hotswap_service).await?;
        } else {
            // --- Standard Static Routing Mode ---
            let mut app = self.router.into_axum();

            // Serve static files from "static" directory if it exists
            if std::path::Path::new("static").exists() {
                app = app.nest_service(
                    "/static",
                    tower_http::services::ServeDir::new("static"),
                );
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

            println!("Rullst framework serving on http://{}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct HotSwapService {
    current_router: Arc<RwLock<axum::Router>>,
}

impl<'a> Service<axum::serve::IncomingStream<'a>> for HotSwapService {
    type Response = HotSwapService;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: axum::serve::IncomingStream<'a>) -> Self::Future {
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
        let router = self.current_router.read().unwrap().clone();
        use tower::ServiceExt;
        let fut = router.oneshot(req);
        Box::pin(async move {
            let res = fut.await.unwrap();
            Ok(res)
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
    
    let full_lib_path = if lib_path.ends_with(".dll") || lib_path.ends_with(".so") || lib_path.ends_with(".dylib") {
        lib_path.to_string()
    } else {
        format!("{}.{}", lib_path, lib_extension)
    };

    let path_buf = std::path::Path::new(&full_lib_path);
    if !path_buf.exists() {
        return Err(format!("Dylib not found at: {}", full_lib_path).into());
    }

    let parent = path_buf.parent().unwrap_or(std::path::Path::new("."));
    let filename = path_buf.file_stem().unwrap().to_str().unwrap();

    // Clean up older active files that are no longer locked by any process
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(filename) && name.contains("_active_") && name.ends_with(lib_extension) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_filename = format!("{}_active_{}.{}", filename, ts, lib_extension);
    let temp_path = parent.join(temp_filename);
    
    // Copy the dylib file to prevent locking issues
    std::fs::copy(&full_lib_path, &temp_path)?;
    
    let lib = unsafe { libloading::Library::new(temp_path)? };
    let init_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut Router> = unsafe { lib.get(b"rullst_router_init")? };
    let router_ptr = unsafe { init_fn() };
    
    // Convert *mut Router back to Router box and extract it
    let rullst_router = unsafe { *Box::from_raw(router_ptr) };
    
    // Convert Rullst Router to Axum Router
    let mut axum_router = rullst_router.into_axum();
    
    // Serve static files from "static" directory if it exists
    if std::path::Path::new("static").exists() {
        axum_router = axum_router.nest_service(
            "/static",
            tower_http::services::ServeDir::new("static"),
        );
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
