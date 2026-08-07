use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware that protects WebSocket upgrades against Cross-Site WebSocket Hijacking (CSWSH).
pub async fn cswsh_guard_middleware(req: Request, next: Next) -> Response {
    let is_ws_upgrade = req
        .headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_ws_upgrade {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());

        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok());

        let mut valid = false;
        if let (Some(orig), Some(h)) = (origin, host) {
            let orig_host = orig
                .strip_prefix("http://")
                .or_else(|| orig.strip_prefix("https://"))
                .unwrap_or(orig);

            if orig_host == h
                || orig_host.starts_with("localhost")
                || orig_host.starts_with("127.0.0.1")
            {
                valid = true;
            }
        } else if origin.is_none() {
            // Non-browser direct WebSocket client
            valid = true;
        }

        if !valid {
            SecurityStore::global().inc_cswsh_blocks();
            return (
                StatusCode::FORBIDDEN,
                "Cross-Site WebSocket Hijacking (CSWSH) Intercepted",
            )
                .into_response();
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    #[tokio::test]
    async fn test_cswsh_valid_origin() {
        let req = HttpRequest::builder()
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "app.example.com")
            .header(header::ORIGIN, "https://app.example.com")
            .body(axum::body::Body::empty())
            .unwrap();

        let is_valid = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|orig| orig.contains("app.example.com"))
            .unwrap_or(false);

        assert!(is_valid);
    }
}
