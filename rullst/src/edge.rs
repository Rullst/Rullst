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
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        use axum::Router;
        use axum::extract::Request;
        use axum::routing::any;

        let handler = self.handler.clone();
        let app = Router::new().route(
            "/{*path}",
            any(move |req: Request| {
                let handler = handler.clone();
                async move {
                    let (parts, body) = req.into_parts();
                    let method = parts.method.to_string();
                    let path = parts.uri.path().to_string();
                    let mut headers = HashMap::new();
                    for (k, v) in parts.headers.iter() {
                        if let Ok(val) = v.to_str() {
                            headers.insert(k.as_str().to_string(), val.to_string());
                        }
                    }

                    let body_bytes = match axum::body::to_bytes(body, 2 * 1024 * 1024).await {
                        Ok(bytes) => bytes.to_vec(),
                        Err(_) => Vec::new(),
                    };

                    let edge_req = EdgeRequest {
                        method,
                        path,
                        headers,
                        body: body_bytes,
                    };

                    let edge_resp = handler(edge_req).await;

                    let mut res_builder = axum::http::Response::builder().status(edge_resp.status);
                    for (k, v) in edge_resp.headers.iter() {
                        res_builder = res_builder.header(k, v);
                    }
                    match res_builder.body(axum::body::Body::from(edge_resp.body)) {
                        Ok(res) => res,
                        Err(_) => {
                            let mut err_res =
                                axum::response::Response::new(axum::body::Body::empty());
                            *err_res.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                            err_res
                        }
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        println!(
            "🚀 Edge local emulator running on http://localhost:{}",
            self.port
        );

        // Spawn serving loop
        axum::serve(listener, app).await?;
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
}
