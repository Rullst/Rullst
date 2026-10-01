//! Rullst Edge Runtime (`rullst::edge`)
//!
//! Native support for compiling and running Rullst apps on WebAssembly edge infrastructure
//! (Cloudflare Workers, Fastly Compute, AWS Lambda@Edge) abstracting Tokio/WASI differences.

use std::collections::HashMap;
use std::future::Future;

/// Environment-agnostic HTTP request payload designed for maximum compatibility.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct EdgeRequest {
    /// HTTP method (e.g., "GET", "POST").
    pub method: String,
    /// Request URL path (e.g., "/users").
    pub path: String,
    /// Raw query string after `?`, still percent-encoded (e.g. `q=rust&page=2`),
    /// or `None` when the URL has no `?`.
    ///
    /// Unpublished v13 API.
    pub query: Option<String>,
    /// Collection of request headers.
    pub headers: HashMap<String, String>,
    /// Raw request body in bytes.
    pub body: Vec<u8>,
}

impl EdgeRequest {
    /// Creates a new `EdgeRequest` using constructor and builder pattern for backwards compatibility.
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query: None,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Appends a header key-value pair to the request.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the raw request body.
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

/// Environment-agnostic HTTP response payload designed for maximum compatibility.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct EdgeResponse {
    /// HTTP status code (e.g., 200, 404).
    pub status: u16,
    /// Collection of response headers.
    pub headers: HashMap<String, String>,
    /// Raw response body in bytes.
    pub body: Vec<u8>,
}

impl EdgeResponse {
    /// Creates a new `EdgeResponse` using constructor and builder pattern for backwards compatibility.
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Appends a header key-value pair to the response.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the raw response body.
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

/// Environment-agnostic task spawner mapping to native Tokio or WASM local execution environments.
#[cfg_attr(mutants, mutants::skip)]
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(future);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::spawn(future);
    }
}

/// Portable Edge server running a local Axum emulator on native, and a direct executor on WASM.
#[non_exhaustive]
pub struct EdgeServer<F> {
    /// The edge HTTP request handler function.
    pub handler: F,
    /// Optional: Local port to bind the emulation server.
    pub port: u16,
}

impl<F, Fut> EdgeServer<F>
where
    F: Fn(EdgeRequest) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = EdgeResponse> + Send + 'static,
{
    /// Creates a new `EdgeServer` with the specified request handler.
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            port: 3000,
        }
    }

    /// Sets the local TCP port of the emulation server.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Serves request handling either natively as an emulator or natively in WASM edge runtimes.
    ///
    /// The native emulator binds `127.0.0.1` unless `HOST` or `RULLST_HOST` is
    /// set (the same variables, in the same order, as [`crate::Server`]). It
    /// answers every path including `/`. A request body larger than
    /// 2 MiB is rejected with `413`, and a body that cannot be
    /// read with `400`, without calling the handler.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let host = emulator_host(crate::server::builder::read_optional_environment_variable)?;
        let listener = tokio::net::TcpListener::bind((host.as_str(), self.port)).await?;
        println!(
            "🚀 Edge local emulator running on http://{}",
            listener.local_addr()?
        );

        axum::serve(listener, emulator_router(self.handler)).await?;
        Ok(())
    }

    /// Serves request handling natively inside WASM WASI edge loops.
    #[cfg(target_arch = "wasm32")]
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // In actual Cloudflare Workers or WASM Edge targets,
        // the global handler is registered statically.
        // We log execution readiness for testing.
        web_sys::console::log_1(&"🚀 Rullst Edge Runtime serving on WASM target".into());
        Ok(())
    }
}

/// Largest request body the native emulator buffers for an [`EdgeRequest`].
#[cfg(not(target_arch = "wasm32"))]
const MAX_EDGE_BODY_BYTES: usize = 2 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
fn emulator_host(
    environment: impl Fn(&str) -> Result<Option<String>, crate::server::ServerError>,
) -> Result<String, crate::server::ServerError> {
    Ok(environment("HOST")?
        .or(environment("RULLST_HOST")?)
        .unwrap_or_else(|| "127.0.0.1".to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn emulator_router<F, Fut>(handler: F) -> axum::Router
where
    F: Fn(EdgeRequest) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = EdgeResponse> + Send + 'static,
{
    use axum::extract::DefaultBodyLimit;
    use axum::http::{HeaderMap, Method, StatusCode, Uri};
    use axum::routing::any;

    // `Bytes` rejects an oversized body with 413 and an unreadable one with 400
    // before the handler runs, instead of substituting an empty body.
    let service = any(
        move |method: Method, uri: Uri, header_map: HeaderMap, body: axum::body::Bytes| {
            let handler = handler.clone();
            async move {
                let mut headers = HashMap::new();
                for (k, v) in header_map.iter() {
                    if let Ok(val) = v.to_str() {
                        headers.insert(k.as_str().to_string(), val.to_string());
                    }
                }

                let edge_req = EdgeRequest {
                    method: method.to_string(),
                    path: uri.path().to_string(),
                    query: uri.query().map(str::to_string),
                    headers,
                    body: body.to_vec(),
                };

                let edge_resp = handler(edge_req).await;

                let mut res_builder = axum::http::Response::builder().status(edge_resp.status);
                for (k, v) in edge_resp.headers.iter() {
                    res_builder = res_builder.header(k, v);
                }
                match res_builder.body(axum::body::Body::from(edge_resp.body)) {
                    Ok(res) => res,
                    Err(_) => {
                        let mut err_res = axum::response::Response::new(axum::body::Body::empty());
                        *err_res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        err_res
                    }
                }
            }
        },
    );

    axum::Router::new()
        .route("/", service.clone())
        .route("/{*path}", service)
        .layer(DefaultBodyLimit::max(MAX_EDGE_BODY_BYTES))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_request_builder() {
        let req = EdgeRequest::new("POST", "/test")
            .with_header("X-Foo", "bar")
            .with_body(vec![1, 2, 3]);
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/test");
        assert_eq!(req.headers.get("X-Foo").map(|s| s.as_str()), Some("bar"));
        assert_eq!(req.body, vec![1, 2, 3]);
    }

    #[test]
    fn test_edge_response_builder() {
        let res = EdgeResponse::new(201)
            .with_header("Content-Type", "application/json")
            .with_body(vec![123, 125]);
        assert_eq!(res.status, 201);
        assert_eq!(
            res.headers.get("Content-Type").map(|s| s.as_str()),
            Some("application/json")
        );
        assert_eq!(res.body, vec![123, 125]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn emulator_rejects_unreadable_bodies_and_serves_the_root() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tower::ServiceExt;

        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let router = emulator_router(move |req: EdgeRequest| {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                EdgeResponse::new(200).with_body(format!("{} {}", req.path, req.body.len()).into())
            }
        });
        let send = |uri: &str, body: Vec<u8>| {
            router.clone().oneshot(
                axum::http::Request::post(uri)
                    .body(axum::body::Body::from(body))
                    .unwrap(),
            )
        };

        let response = send("/documents/1", vec![b'x'; MAX_EDGE_BODY_BYTES + 1])
            .await
            .unwrap();
        assert_eq!(response.status(), 413);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "handler must not run");

        let response = send("/", b"ok".to_vec()).await.unwrap();
        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"/ 2");

        let response = send("/documents/1", vec![b'x'; MAX_EDGE_BODY_BYTES])
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn emulator_forwards_the_query_string() {
        use tower::ServiceExt;

        let router = emulator_router(|req: EdgeRequest| async move {
            EdgeResponse::new(200).with_body(format!("{} {:?}", req.path, req.query).into())
        });
        for (uri, expected) in [
            ("/search?q=rust&page=2", r#"/search Some("q=rust&page=2")"#),
            ("/search", "/search None"),
        ] {
            let response = router
                .clone()
                .oneshot(
                    axum::http::Request::get(uri)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert_eq!(&body[..], expected.as_bytes(), "{uri}");
        }
        assert_eq!(EdgeRequest::new("GET", "/").query, None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[allow(clippy::unwrap_used)]
    fn emulator_binds_loopback_unless_a_host_is_configured() {
        assert_eq!(emulator_host(|_| Ok(None)).unwrap(), "127.0.0.1");
        let configured = |name: &str| Ok((name == "RULLST_HOST").then(|| "0.0.0.0".to_string()));
        assert_eq!(emulator_host(configured).unwrap(), "0.0.0.0");
        let both = |name: &str| Ok(Some(name.to_ascii_lowercase()));
        assert_eq!(emulator_host(both).unwrap(), "host");
    }
}
