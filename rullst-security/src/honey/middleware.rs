use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use dashmap::DashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone, Debug)]
pub struct HoneypotState {
    banned_ips: Arc<DashMap<String, u64>>,
    trap_paths: Arc<Vec<String>>,
}

impl Default for HoneypotState {
    fn default() -> Self {
        Self::new(vec![
            "/.env".to_string(),
            "/.git/config".to_string(),
            "/admin.php".to_string(),
            "/wp-login.php".to_string(),
            "/phpmyadmin".to_string(),
            "/config.json".to_string(),
        ])
    }
}

impl HoneypotState {
    pub fn new(trap_paths: Vec<String>) -> Self {
        Self {
            banned_ips: Arc::new(DashMap::new()),
            trap_paths: Arc::new(trap_paths),
        }
    }

    pub fn is_banned(&self, ip: &str) -> bool {
        self.banned_ips.contains_key(ip)
    }

    pub fn ban_ip(&self, ip: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.banned_ips.insert(ip, now);
    }

    pub fn is_trap(&self, path: &str) -> bool {
        let clean_path = path.to_lowercase();
        self.trap_paths.iter().any(|trap| clean_path.contains(trap))
    }

    pub fn banned_count(&self) -> usize {
        self.banned_ips.len()
    }
}

#[derive(Clone)]
pub struct HoneypotLayer {
    state: HoneypotState,
}

impl HoneypotLayer {
    pub fn new(state: HoneypotState) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for HoneypotLayer {
    type Service = HoneypotService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HoneypotService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct HoneypotService<S> {
    inner: S,
    state: HoneypotState,
}

impl<S> Service<Request<Body>> for HoneypotService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let client_ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("127.0.0.1")
            .split(',')
            .next()
            .unwrap_or("127.0.0.1")
            .trim()
            .to_string();

        if self.state.is_banned(&client_ip) {
            let res = (
                StatusCode::FORBIDDEN,
                "Access Denied: IP Banned by Rullst Honey",
            )
                .into_response();
            return Box::pin(async move { Ok(res) });
        }

        let path = req.uri().path().to_string();
        if self.state.is_trap(&path) {
            tracing::warn!(target: "rullst_security::honey", ip = %client_ip, path = %path, "Honeypot trap triggered. Banning IP.");
            self.state.ban_ip(client_ip);
            let res = (
                StatusCode::FORBIDDEN,
                "Access Denied: Honeypot Trap Triggered",
            )
                .into_response();
            return Box::pin(async move { Ok(res) });
        }

        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
