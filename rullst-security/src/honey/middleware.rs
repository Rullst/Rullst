use super::bans::BanList;
use crate::error::SecurityError;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, Request, Response, StatusCode, header},
    response::IntoResponse,
};
use std::collections::HashSet;
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

/// Shared honeypot configuration and bounded, expiring peer bans.
///
/// A request lookup touches only its own peer entry. Expired bans are pruned in
/// expiry order when a ban is added or counted, and at capacity the ban that
/// expires soonest is evicted, so neither path scans every retained ban.
#[derive(Clone, Debug)]
pub struct HoneypotState {
    banned_ips: Arc<Mutex<BanList>>,
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
            banned_ips: Arc::new(Mutex::new(BanList::default())),
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
        let Ok(mut bans) = self.banned_ips.lock() else {
            // A poisoned security state must not silently allow requests.
            return true;
        };
        bans.is_banned(ip, Instant::now())
    }

    pub fn ban_ip(&self, ip: String) {
        if let Ok(ip) = ip.parse::<IpAddr>() {
            self.ban_peer(ip);
        }
    }

    fn ban_peer(&self, ip: IpAddr) {
        let now = Instant::now();
        let Some(expires_at) = now.checked_add(self.ban_ttl) else {
            return;
        };
        let Ok(mut bans) = self.banned_ips.lock() else {
            return;
        };
        bans.prune_expired(now);
        bans.insert(ip, expires_at, self.max_bans);
    }

    /// Matches only a complete configured URI path; substrings and prefixes are not traps.
    pub fn is_trap(&self, path: &str) -> bool {
        self.trap_paths
            .iter()
            .any(|trap| trap.eq_ignore_ascii_case(path))
    }

    pub fn banned_count(&self) -> usize {
        let Ok(mut bans) = self.banned_ips.lock() else {
            return self.max_bans;
        };
        bans.prune_expired(Instant::now());
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

/// Whether the request shows that a web page initiated it.
///
/// Browsers label every request they send to a potentially trustworthy origin
/// with `Sec-Fetch-Site`: `none` for a user-typed URL or bookmark, and
/// `same-origin`, `same-site` or `cross-site` when a document started the
/// load, such as an `<img>` on another site, Markdown in user content or a
/// scripted navigation. Browsers without fetch metadata still name the
/// initiating page in `Origin` or `Referer`. A page can make its visitors'
/// browsers request a trap path, so such a hit must not ban the visitor's
/// address. The headers are client-controlled: a scanner that sends them
/// avoids the ban, but not the refusal.
fn is_page_initiated(headers: &HeaderMap) -> bool {
    match headers.get("sec-fetch-site") {
        Some(site) => ["same-origin", "same-site", "cross-site"]
            .iter()
            .any(|initiator| site.as_bytes().eq_ignore_ascii_case(initiator.as_bytes())),
        None => headers.contains_key(header::ORIGIN) || headers.contains_key(header::REFERER),
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
            // Refuse every trap hit, but ban only a direct request: a page
            // can make any visitor's browser load a trap URL.
            let page_initiated = is_page_initiated(req.headers());
            let telemetry = crate::telemetry::SecurityStore::global();
            if page_initiated {
                telemetry.record_honeypot_observation(&client_ip, &path);
            } else {
                if let Some(ip) = peer_ip {
                    self.state.ban_peer(ip);
                }
                telemetry.record_honeypot_trap_with_ttl(&client_ip, &path, self.state.ban_ttl);
            }
            tracing::warn!(target: "rullst_security::honey", ip = %client_ip, path = %path, page_initiated, "Honeypot trap triggered");
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
mod tests;
