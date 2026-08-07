use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use dashmap::DashSet;
use std::sync::OnceLock;

static DECEPTION_ROUTES: OnceLock<DashSet<String>> = OnceLock::new();

pub fn default_deception_traps() -> DashSet<String> {
    let set = DashSet::new();
    for route in &[
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
    ] {
        set.insert((*route).to_string());
    }
    set
}

pub fn global_deception_routes() -> &'static DashSet<String> {
    DECEPTION_ROUTES.get_or_init(default_deception_traps)
}

/// Registers a custom decoy route dynamically in the deception trap registry.
pub fn register_deception_trap(route: &str) {
    let clean = if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{}", route)
    };
    global_deception_routes().insert(clean);
}

/// Middleware that checks incoming request URIs against dynamic deception trap routes.
pub async fn deception_trap_middleware(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if global_deception_routes().contains(&path) {
        let client_ip = req
            .headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("127.0.0.1")
            .split(',')
            .next()
            .unwrap_or("127.0.0.1")
            .trim()
            .to_string();

        SecurityStore::global().record_honeypot_trap(&client_ip, &path);
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

    #[test]
    fn test_register_deception_trap() {
        register_deception_trap("/api/v1/secret_test");
        assert!(global_deception_routes().contains("/api/v1/secret_test"));
    }
}
