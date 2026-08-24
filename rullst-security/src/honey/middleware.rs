use crate::error::SecurityError;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

/// Default lifetime of a honeypot peer ban.
pub const DEFAULT_HONEYPOT_BAN_TTL: Duration = Duration::from_secs(15 * 60);
/// Default upper bound for concurrently tracked banned peers.
pub const DEFAULT_MAX_HONEYPOT_BANS: usize = 100_000;
/// Upper bound for configured exact trap paths.
pub const MAX_HONEYPOT_TRAP_PATHS: usize = 1_024;

#[derive(Clone, Debug)]
pub struct HoneypotState {
    banned_ips: Arc<Mutex<HashMap<IpAddr, Instant>>>,
    trap_paths: Arc<Vec<String>>,
    ban_ttl: Duration,
    max_bans: usize,
}

impl Default for HoneypotState {
    fn default() -> Self {
        Self::new(vec![
            "/.env".to_string(),
            "/.env.local".to_string(),
            "/.env.production".to_string(),
            "/.git/config".to_string(),
            "/.aws/credentials".to_string(),
            "/.vscode/sftp.json".to_string(),
            "/.ds_store".to_string(),
            "/admin.php".to_string(),
            "/wp-login.php".to_string(),
            "/wp-admin/".to_string(),
            "/phpmyadmin/".to_string(),
            "/config.json".to_string(),
            "/setup.php".to_string(),
            "/xmlrpc.php".to_string(),
            "/actuator/health".to_string(),
            "/console".to_string(),
            "/api/v1/debug".to_string(),
            "/swagger-ui.html".to_string(),
            "/database.sqlite".to_string(),
            "/backup.sql".to_string(),
            "/server-status".to_string(),
            "/docker-compose.yml".to_string(),
        ])
    }
}

impl HoneypotState {
    /// Compatibility constructor using bounded, expiring defaults.
    ///
    /// Invalid paths are ignored and excess paths are truncated. Use [`Self::try_with_limits`]
    /// when configuration errors must abort application startup.
    pub fn new(trap_paths: Vec<String>) -> Self {
        let trap_paths = canonical_trap_paths(trap_paths)
            .into_iter()
            .take(MAX_HONEYPOT_TRAP_PATHS)
            .collect();
        Self::new_inner(
            trap_paths,
            DEFAULT_HONEYPOT_BAN_TTL,
            DEFAULT_MAX_HONEYPOT_BANS,
        )
    }

    /// Creates a honeypot state with explicit finite ban lifetime and cardinality limits.
    pub fn try_with_limits(
        trap_paths: Vec<String>,
        ban_ttl: Duration,
        max_bans: usize,
    ) -> Result<Self, SecurityError> {
        if ban_ttl.is_zero() {
            return Err(SecurityError::General(
                "honeypot ban TTL must be greater than zero".to_string(),
            ));
        }
        if max_bans == 0 {
            return Err(SecurityError::General(
                "honeypot maximum ban cardinality must be greater than zero".to_string(),
            ));
        }
        if trap_paths.iter().any(|path| !is_valid_trap_path(path)) {
            return Err(SecurityError::General(
                "honeypot trap paths must be absolute paths without queries, fragments, or control characters"
                    .to_string(),
            ));
        }

        let paths = canonical_trap_paths(trap_paths);
        if paths.len() > MAX_HONEYPOT_TRAP_PATHS {
            return Err(SecurityError::General(format!(
                "honeypot accepts at most {MAX_HONEYPOT_TRAP_PATHS} trap paths"
            )));
        }
        if paths.is_empty() {
            return Err(SecurityError::General(
                "honeypot requires at least one valid absolute trap path".to_string(),
            ));
        }

        Ok(Self::new_inner(paths, ban_ttl, max_bans))
    }

    fn new_inner(trap_paths: Vec<String>, ban_ttl: Duration, max_bans: usize) -> Self {
        Self {
            banned_ips: Arc::new(Mutex::new(HashMap::new())),
            trap_paths: Arc::new(trap_paths),
            ban_ttl,
            max_bans,
        }
    }

    pub fn is_banned(&self, ip: &str) -> bool {
        let Ok(ip) = ip.parse::<IpAddr>() else {
            return false;
        };
        self.is_peer_banned(ip)
    }

    fn is_peer_banned(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let Ok(mut bans) = self.banned_ips.lock() else {
            // A poisoned security state must not silently allow requests.
            return true;
        };
        bans.retain(|_, expires_at| *expires_at > now);
        bans.contains_key(&ip)
    }

    pub fn ban_ip(&self, ip: String) {
        if let Ok(ip) = ip.parse::<IpAddr>() {
            self.ban_peer(ip);
        }
    }

    fn ban_peer(&self, ip: IpAddr) {
        let now = Instant::now();
        let Ok(mut bans) = self.banned_ips.lock() else {
            return;
        };
        bans.retain(|_, expires_at| *expires_at > now);

        if bans.len() >= self.max_bans
            && !bans.contains_key(&ip)
            && let Some(oldest_ip) = bans
                .iter()
                .min_by_key(|(_, expires_at)| **expires_at)
                .map(|(ip, _)| *ip)
        {
            bans.remove(&oldest_ip);
        }
        let Some(expires_at) = now.checked_add(self.ban_ttl) else {
            return;
        };
        bans.insert(ip, expires_at);
    }

    /// Matches only a complete configured URI path; substrings and prefixes are not traps.
    pub fn is_trap(&self, path: &str) -> bool {
        self.trap_paths
            .iter()
            .any(|trap| trap.eq_ignore_ascii_case(path))
    }

    pub fn banned_count(&self) -> usize {
        let now = Instant::now();
        let Ok(mut bans) = self.banned_ips.lock() else {
            return self.max_bans;
        };
        bans.retain(|_, expires_at| *expires_at > now);
        bans.len()
    }
}

fn canonical_trap_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| is_valid_trap_path(path))
        .filter(|path| seen.insert(path.to_ascii_lowercase()))
        .collect()
}

fn is_valid_trap_path(path: &str) -> bool {
    path.starts_with('/')
        && path.len() <= 2_048
        && !path.contains('?')
        && !path.contains('#')
        && !path.chars().any(char::is_control)
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
        // ConnectInfo is created from the accepted socket. Untrusted forwarding headers are never
        // used as an enforcement identity.
        let peer_ip = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|connection| connection.0.ip());

        if peer_ip.is_some_and(|ip| self.state.is_peer_banned(ip)) {
            let response = (
                StatusCode::FORBIDDEN,
                "Access Denied: IP Banned by Rullst Honey",
            )
                .into_response();
            return Box::pin(async move { Ok(response) });
        }

        let path = req.uri().path().to_string();
        if self.state.is_trap(&path) {
            let client_ip = peer_ip
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            if let Some(ip) = peer_ip {
                self.state.ban_peer(ip);
            }
            tracing::warn!(target: "rullst_security::honey", ip = %client_ip, path = %path, "Honeypot trap triggered");
            crate::telemetry::SecurityStore::global().record_honeypot_trap_with_ttl(
                &client_ip,
                &path,
                self.state.ban_ttl,
            );
            let response = (
                StatusCode::FORBIDDEN,
                "Access Denied: Honeypot Trap Triggered",
            )
                .into_response();
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(req);
        Box::pin(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use std::thread;
    use tower::ServiceExt;

    #[test]
    fn traps_use_exact_paths_and_bans_expire() {
        let state =
            HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_millis(10), 2)
                .expect("valid honeypot state");

        assert!(state.is_trap("/.env"));
        assert!(!state.is_trap("/download/.env"));
        assert!(!state.is_trap("/.environment"));

        state.ban_ip("192.0.2.1".to_string());
        assert!(state.is_banned("192.0.2.1"));
        thread::sleep(Duration::from_millis(20));
        assert!(!state.is_banned("192.0.2.1"));
    }

    #[test]
    fn ban_cardinality_is_bounded_and_invalid_ips_are_ignored() {
        let state =
            HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_secs(60), 2)
                .expect("valid honeypot state");

        state.ban_ip("not-an-ip".to_string());
        state.ban_ip("192.0.2.1".to_string());
        state.ban_ip("192.0.2.2".to_string());
        state.ban_ip("192.0.2.3".to_string());
        assert_eq!(state.banned_count(), 2);
    }

    #[tokio::test]
    async fn middleware_ignores_forwarded_identity_and_uses_socket_peer() {
        let state = HoneypotState::new(vec!["/.env".to_string()]);
        let app = Router::new()
            .route("/{*path}", get(|| async { StatusCode::OK }))
            .layer(HoneypotLayer::new(state.clone()));

        let forged = Request::builder()
            .uri("/.env")
            .header("x-forwarded-for", "198.51.100.99")
            .body(Body::empty())
            .expect("valid request");
        let response = app.clone().oneshot(forged).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(!state.is_banned("198.51.100.99"));

        let mut verified = Request::builder()
            .uri("/.env")
            .header("x-forwarded-for", "198.51.100.99")
            .body(Body::empty())
            .expect("valid request");
        verified.extensions_mut().insert(ConnectInfo(
            "192.0.2.25:443"
                .parse::<SocketAddr>()
                .expect("valid socket address"),
        ));
        let response = app.clone().oneshot(verified).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(state.is_banned("192.0.2.25"));
        assert!(!state.is_banned("198.51.100.99"));

        let lookalike = Request::builder()
            .uri("/download/.env")
            .body(Body::empty())
            .expect("valid request");
        let response = app.oneshot(lookalike).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
