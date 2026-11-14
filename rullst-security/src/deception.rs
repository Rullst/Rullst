//! Process-wide decoy routes that refuse and record probes, without bans.
//!
//! **Risk reduced:** reconnaissance of well-known sensitive paths going
//! unnoticed.
//!
//! **How:** [`deception_trap_middleware`] compares the exact request path with
//! a global registry (default decoys plus at most [`MAX_DECEPTION_TRAPS`]
//! paths you register), answers `403` and records a telemetry event.
//!
//! **Known limits:** it does not ban the peer, so a scanner keeps probing
//! other paths; use [`crate::HoneypotLayer`] when you want bans. The registry
//! is global to the process and not shared between instances.
//!
//! **Operator duties:** register only paths your application never serves and
//! review the recorded events.

use crate::{SecurityError, telemetry::SecurityStore};
use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use dashmap::DashSet;
use std::net::SocketAddr;
use std::sync::OnceLock;

static DECEPTION_ROUTES: OnceLock<DeceptionRoutes> = OnceLock::new();
/// Maximum dynamic trap paths retained by the process-local registry.
pub const MAX_DECEPTION_TRAPS: usize = 1_024;

const DEFAULT_TRAPS: [&str; 11] = [
    "/.env",
    "/.env.local",
    "/.env.production",
    "/.git/config",
    "/.aws/credentials",
    "/admin.php",
    "/wp-login.php",
    "/wp-admin/",
    "/phpmyadmin/",
    "/api/v1/admin/debug",
    "/graphql/v1",
];

/// The process-wide decoy paths that [`deception_trap_middleware`] refuses,
/// returned by [`global_deception_routes`]. Paths are added with
/// [`try_register_deception_trap`] and never removed.
#[derive(Debug)]
pub struct DeceptionRoutes {
    routes: DashSet<String>,
}

impl DeceptionRoutes {
    fn with_defaults() -> Self {
        Self {
            routes: DEFAULT_TRAPS
                .iter()
                .map(|route| (*route).to_string())
                .collect(),
        }
    }

    /// Whether `path` is a registered decoy (an exact match).
    pub fn contains(&self, path: &str) -> bool {
        self.routes.contains(path)
    }

    /// The number of registered decoys, defaults included.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether no decoy is registered.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// The registered decoy paths, sorted.
    pub fn routes(&self) -> Vec<String> {
        let mut routes: Vec<String> = self
            .routes
            .iter()
            .map(|route| route.key().clone())
            .collect();
        routes.sort();
        routes
    }
}

/// The default decoy paths, sorted.
pub fn default_deception_traps() -> Vec<String> {
    let mut routes: Vec<String> = DEFAULT_TRAPS
        .iter()
        .map(|route| (*route).to_string())
        .collect();
    routes.sort();
    routes
}

/// Returns the process-wide decoy registry, initialized with the defaults.
pub fn global_deception_routes() -> &'static DeceptionRoutes {
    DECEPTION_ROUTES.get_or_init(DeceptionRoutes::with_defaults)
}

/// Registers a custom decoy route dynamically in the deception trap registry.
pub fn register_deception_trap(route: &str) {
    let _ = try_register_deception_trap(route);
}

/// Validates and registers a bounded exact decoy path.
pub fn try_register_deception_trap(route: &str) -> Result<(), SecurityError> {
    let clean = if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    };
    if clean.len() > 2_048
        || clean.contains(['?', '#'])
        || clean.chars().any(char::is_control)
        || clean
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(SecurityError::General(
            "deception traps must be exact bounded paths without traversal, queries, fragments, or control characters"
                .to_string(),
        ));
    }
    let routes = global_deception_routes();
    if routes.len() >= MAX_DECEPTION_TRAPS && !routes.contains(&clean) {
        return Err(SecurityError::General(format!(
            "deception trap registry accepts at most {MAX_DECEPTION_TRAPS} paths"
        )));
    }
    routes.routes.insert(clean);
    Ok(())
}

/// Middleware that checks incoming request URIs against dynamic deception trap routes.
pub async fn deception_trap_middleware(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if global_deception_routes().contains(&path) {
        let client_ip = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|connection| connection.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        SecurityStore::global().record_honeypot_observation(&client_ip, &path);
        SecurityStore::global().inc_deception_hits();

        return (
            StatusCode::FORBIDDEN,
            Html("<h1>403 Access Denied</h1><p>Dynamic Security Deception Shield Engaged.</p>"),
        )
            .into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, middleware, routing::get};
    use tower::ServiceExt;

    #[test]
    fn the_registry_exposes_sorted_copies_of_its_paths() {
        let defaults = default_deception_traps();
        assert_eq!(defaults.len(), DEFAULT_TRAPS.len());
        assert!(defaults.windows(2).all(|pair| pair[0] < pair[1]));
        let fresh = DeceptionRoutes::with_defaults();
        assert_eq!(fresh.routes(), defaults);
        assert_eq!(fresh.len(), DEFAULT_TRAPS.len());
        assert!(!fresh.is_empty());
        let empty = DeceptionRoutes {
            routes: DashSet::new(),
        };
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.routes().is_empty());
        assert!(fresh.contains("/.env"));
        assert!(!fresh.contains("/.env/"));
        let local = DeceptionRoutes::with_defaults();
        local.routes.insert("/sorted-registry-trap".to_string());
        let local_routes = local.routes();
        assert_eq!(local.len(), DEFAULT_TRAPS.len() + 1);
        assert_eq!(local_routes.len(), local.len());
        assert!(local_routes.windows(2).all(|pair| pair[0] < pair[1]));
        // Other tests register traps in the process-wide registry at the same
        // time, so its size is never compared across two separate reads.
        try_register_deception_trap("/sorted-registry-trap").expect("valid trap");
        let routes = global_deception_routes().routes();
        assert!(routes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(routes.iter().any(|route| route == "/sorted-registry-trap"));
    }

    #[test]
    fn test_register_deception_trap() {
        try_register_deception_trap("/api/v1/secret_test").expect("valid trap");
        assert!(global_deception_routes().contains("/api/v1/secret_test"));
        assert!(try_register_deception_trap("../escape?token=secret").is_err());
    }

    #[test]
    fn traps_are_bounded_exact_paths() {
        let longest = format!("/longest-trap-{}", "a".repeat(2_048 - 14));
        assert_eq!(longest.len(), 2_048);
        assert!(try_register_deception_trap(&longest).is_ok());
        assert!(global_deception_routes().contains(&longest));
        for invalid in [
            format!("/longer-trap-{}", "a".repeat(2_048 - 12)),
            "/trap-with-query?x".to_string(),
            "/trap-with-fragment#x".to_string(),
            "/trap-with-control\u{7}".to_string(),
            "/trap/../escape".to_string(),
            "/trap/./same".to_string(),
        ] {
            assert!(
                try_register_deception_trap(&invalid).is_err(),
                "{} bytes",
                invalid.len()
            );
            assert!(!global_deception_routes().contains(&invalid));
        }
    }

    #[tokio::test]
    async fn middleware_blocks_registered_traps_and_allows_safe_routes() {
        register_deception_trap("agent_test_trap");
        let app = Router::new()
            .route("/safe", get(|| async { StatusCode::OK }))
            .fallback(|| async { StatusCode::NOT_FOUND })
            .layer(middleware::from_fn(deception_trap_middleware));

        let mut trapped = Request::get("/agent_test_trap")
            .body(Body::empty())
            .expect("request should be valid");
        trapped.extensions_mut().insert(ConnectInfo(
            "127.0.0.27:4123"
                .parse::<SocketAddr>()
                .expect("socket address should be valid"),
        ));
        let response = app
            .clone()
            .oneshot(trapped)
            .await
            .expect("trap request should complete");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .clone()
            .oneshot(
                Request::get("/agent_test_trap")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("trap without peer information should complete");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::get("/safe")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("safe request should complete");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
