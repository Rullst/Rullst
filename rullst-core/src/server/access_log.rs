//! The path an access-log line shows: the matched route template, or a
//! bounded and escaped request path ([`crate::config::AccessLogPaths`]).

use axum::extract::MatchedPath;
use axum::http::Extensions;
use std::fmt::Write;

/// Longest request path, in bytes, a line shows for a request that matched
/// no route; a longer one is cut and ends in `…`.
const MAX_UNMATCHED_PATH_BYTES: usize = 128;

/// The access-log settings `Server` resolves at start-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccessLog {
    /// Whether successful `/health` and `/ready` requests are logged.
    pub(crate) log_health_probes: bool,
    /// Whether lines show the matched route instead of the request path.
    pub(crate) route_paths: bool,
}

impl AccessLog {
    /// The path a line shows for a request to `path` with `extensions`.
    pub(crate) fn shown_path(self, path: &str, extensions: &Extensions) -> String {
        if !self.route_paths {
            return path.to_string();
        }
        match extensions.get::<MatchedPath>() {
            Some(route) => route.as_str().to_string(),
            None => bounded_path(path),
        }
    }
}

/// `path` with every byte outside visible ASCII written `%XX`, cut after
/// [`MAX_UNMATCHED_PATH_BYTES`] bytes.
fn bounded_path(path: &str) -> String {
    let mut shown = String::with_capacity(path.len().min(MAX_UNMATCHED_PATH_BYTES) + 3);
    for byte in path.bytes() {
        let width = if byte.is_ascii_graphic() { 1 } else { 3 };
        if shown.len() + width > MAX_UNMATCHED_PATH_BYTES {
            shown.push('…');
            break;
        }
        if width == 1 {
            shown.push(char::from(byte));
        } else {
            let _ = write!(shown, "%{byte:02X}");
        }
    }
    shown
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    const ROUTES: AccessLog = AccessLog {
        log_health_probes: true,
        route_paths: true,
    };
    const PATHS: AccessLog = AccessLog {
        log_health_probes: true,
        route_paths: false,
    };

    #[test]
    fn unmatched_paths_are_escaped_and_bounded() {
        assert_eq!(bounded_path("/wp-login.php"), "/wp-login.php");
        assert_eq!(bounded_path("/a b\u{7f}é"), "/a%20b%7F%C3%A9");
        let exact = format!("/{}", "a".repeat(MAX_UNMATCHED_PATH_BYTES - 1));
        assert_eq!(bounded_path(&exact), exact);
        let long = format!("{exact}b");
        assert_eq!(bounded_path(&long), format!("{exact}…"));
        // An escape that would cross the bound is not written in part.
        let escaped = format!("/{}é", "a".repeat(MAX_UNMATCHED_PATH_BYTES - 3));
        assert_eq!(
            bounded_path(&escaped),
            format!("/{}…", "a".repeat(MAX_UNMATCHED_PATH_BYTES - 3))
        );
        let fits = format!("/{}é", "a".repeat(MAX_UNMATCHED_PATH_BYTES - 7));
        assert_eq!(
            bounded_path(&fits),
            format!("/{}%C3%A9", "a".repeat(MAX_UNMATCHED_PATH_BYTES - 7))
        );
    }

    /// The path each request's line shows, as the server's middleware sees it.
    async fn shown(log: AccessLog, uris: &[&str]) -> Vec<String> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        let router = axum::Router::new()
            .route("/booking/{token}", axum::routing::get(|| async { "ok" }))
            .nest(
                "/api",
                axum::Router::new().route("/users/{id}", axum::routing::get(|| async { "ok" })),
            )
            .layer(axum::middleware::from_fn(
                move |request: axum::extract::Request, next: axum::middleware::Next| {
                    let record = record.clone();
                    async move {
                        let path = log.shown_path(request.uri().path(), request.extensions());
                        record.lock().unwrap().push(path);
                        next.run(request).await
                    }
                },
            ));
        for uri in uris {
            let response = router
                .clone()
                .oneshot(Request::get(*uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        }
        seen.lock().unwrap().clone()
    }

    #[tokio::test]
    async fn route_paths_show_the_template_and_never_the_secret() {
        let uris = [
            "/booking/s3cr3t-t0ken?ref=mail",
            "/api/users/42",
            "/missing/s3cr3t",
        ];
        assert_eq!(
            shown(ROUTES, &uris).await,
            ["/booking/{token}", "/api/users/{id}", "/missing/s3cr3t"]
        );
        assert_eq!(
            shown(PATHS, &uris).await,
            ["/booking/s3cr3t-t0ken", "/api/users/42", "/missing/s3cr3t"]
        );
    }
}
