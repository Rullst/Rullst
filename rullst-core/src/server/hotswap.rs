use crate::server::dylib_loader::load_dylib_router;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use tower_service::Service;

/// Tower service that atomically swaps the Axum router at runtime during hot-reload development.
/// Wraps the router in an `Arc<RwLock<>>` so handlers continue serving in-flight requests
/// while the new router is being compiled and installed.
#[derive(Clone)]
pub struct HotSwapService {
    pub(crate) current_router: Arc<RwLock<axum::Router>>,
    pub(crate) active_libraries: Arc<Mutex<Vec<libloading::Library>>>,
    pub(crate) lib_path: String,
    pub(crate) is_dev: bool,
    pub(crate) shield: Option<crate::resilience::TrafficShield>,
    pub(crate) limiter: Option<crate::resilience::RateLimiter>,
}

impl HotSwapService {
    #[cfg_attr(mutants, mutants::skip)]
    pub(crate) fn handle_oneshot_error()
    -> Result<axum::response::Response, std::convert::Infallible> {
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

    pub(crate) async fn handle_panic_error(
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
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[cfg_attr(mutants, mutants::skip)]
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
                        // Never unload a previous application library while a
                        // request may still be executing code from it. Hot reload
                        // is development-only, so retaining these handles until
                        // server shutdown is the safe lifecycle trade-off.

                        println!(
                            "\x1b[32mRullst hot reload: development library swap completed.\x1b[0m"
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
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let start = std::time::Instant::now();
        use tower::ServiceExt;
        let fut = router.oneshot(req);
        Box::pin(async move {
            let handle = tokio::spawn(async move { fut.await });
            match handle.await {
                Ok(Ok(res)) => {
                    let status = res.status().as_u16();
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    if !path.starts_with("/_rullst_hmr") {
                        println!(
                            "[HTTP] {} {} -> {} ({:.2} ms)",
                            method, path, status, elapsed
                        );
                    }
                    Ok(res)
                }
                Ok(Err(_)) => Self::handle_oneshot_error(),
                Err(join_err) => Self::handle_panic_error(join_err).await,
            }
        })
    }
}

#[cfg(test)]
#[path = "hotswap_tests.rs"]
mod tests;
