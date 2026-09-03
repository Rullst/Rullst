use crate::server::dylib_loader::load_dylib_router;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use subtle::ConstantTimeEq;
use tower_service::Service;

pub(crate) const RELOAD_TOKEN_HEADER: &str = "x-rullst-hmr-token";
pub(crate) const MAX_HOT_RELOAD_GENERATIONS: usize = 64;

/// Tower service that atomically swaps the Axum router at runtime during hot-reload development.
/// Wraps the router in an `Arc<RwLock<>>` so handlers continue serving in-flight requests
/// while the new router is being compiled and installed.
#[derive(Clone)]
pub struct HotSwapService {
    pub(crate) current_router: Arc<RwLock<axum::Router>>,
    pub(crate) active_libraries: Arc<Mutex<Vec<libloading::Library>>>,
    pub(crate) hmr_sender: tokio::sync::broadcast::Sender<String>,
    pub(crate) reload_lock: Arc<tokio::sync::Mutex<()>>,
    pub(crate) reload_token: Arc<str>,
    pub(crate) lib_path: String,
    pub(crate) is_dev: bool,
    pub(crate) shield: Option<crate::resilience::TrafficShield>,
    pub(crate) limiter: Option<crate::resilience::RateLimiter>,
    pub(crate) lifecycle: Option<crate::lifecycle::ApplicationLifecycle>,
}

impl HotSwapService {
    fn response(
        status: axum::http::StatusCode,
        body: impl Into<axum::body::Body>,
    ) -> axum::response::Response {
        match axum::response::Response::builder()
            .status(status)
            .body(body.into())
        {
            Ok(response) => response,
            Err(_) => {
                let mut response = axum::response::Response::new(axum::body::Body::empty());
                *response.status_mut() = status;
                response
            }
        }
    }

    fn reload_is_authorized(&self, request: &axum::extract::Request) -> bool {
        let Some(provided) = request
            .headers()
            .get(RELOAD_TOKEN_HEADER)
            .and_then(|value| value.to_str().ok())
        else {
            return false;
        };
        provided.len() == self.reload_token.len()
            && bool::from(provided.as_bytes().ct_eq(self.reload_token.as_bytes()))
    }

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
        if req.uri().path() == "/_rullst_hmr" && req.method() == axum::http::Method::GET {
            let hmr_sender = self.hmr_sender.clone();
            return Box::pin(async move {
                use tower::ServiceExt;
                let hmr_router = axum::Router::new()
                    .route("/_rullst_hmr", axum::routing::get(hmr_websocket))
                    .with_state(hmr_sender);
                match hmr_router.oneshot(req).await {
                    Ok(response) => Ok(response),
                    Err(_) => Self::handle_oneshot_error(),
                }
            });
        }
        if req.uri().path() == "/_rullst/internal/reload_dylib"
            && req.method() == axum::http::Method::POST
        {
            if !self.reload_is_authorized(&req) {
                return Box::pin(async {
                    Ok(Self::response(
                        axum::http::StatusCode::FORBIDDEN,
                        axum::body::Body::empty(),
                    ))
                });
            }
            let lib_path = self.lib_path.clone();
            let is_dev = self.is_dev;
            let current_router = self.current_router.clone();
            let active_libraries = self.active_libraries.clone();
            let reload_lock = self.reload_lock.clone();
            let hmr_sender = self.hmr_sender.clone();

            return Box::pin(async move {
                let _reload_guard = reload_lock.lock().await;
                let generation_count = active_libraries
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .len();
                if generation_limit_reached(generation_count) {
                    return Ok(Self::response(
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        "Hot-reload generation limit reached; restart `cargo rullst dash` to release retained development libraries.",
                    ));
                }
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
                        let _ = hmr_sender.send(r#"{"type":"UI_UPDATE"}"#.to_string());

                        Ok(Self::response(axum::http::StatusCode::OK, "Swapped"))
                    }
                    Err(e) => {
                        eprintln!(
                            "\x1b[31m❌ Rullst Hot-Reload: Error loading new dylib: {}\x1b[0m",
                            e
                        );
                        Ok(Self::response(
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            e.to_string(),
                        ))
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
        if let Some(ref lifecycle) = self.lifecycle {
            router = crate::lifecycle::apply_lifecycle(router, lifecycle.clone());
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

async fn hmr_websocket(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(sender): axum::extract::State<tokio::sync::broadcast::Sender<String>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| forward_hmr_messages(socket, sender.subscribe()))
}

async fn forward_hmr_messages(
    mut socket: axum::extract::ws::WebSocket,
    mut receiver: tokio::sync::broadcast::Receiver<String>,
) {
    loop {
        match receiver.recv().await {
            Ok(message) => {
                if socket
                    .send(axum::extract::ws::Message::Text(message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

pub(crate) fn generation_limit_reached(generation_count: usize) -> bool {
    generation_count >= MAX_HOT_RELOAD_GENERATIONS
}

#[cfg(test)]
#[path = "hotswap_tests.rs"]
mod tests;
