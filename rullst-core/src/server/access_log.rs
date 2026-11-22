//! Which requests the access log writes (`RULLST_ACCESS_LOG`), and the path
//! a line shows: the matched route template, or a bounded and escaped request
//! path ([`crate::config::AccessLogPaths`]).

use axum::extract::MatchedPath;
use axum::http::Extensions;
use axum::response::Response;
use std::fmt::Write;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};

/// Longest request path, in bytes, a line shows for a request that matched
/// no route; a longer one is cut and ends in `…`.
const MAX_UNMATCHED_PATH_BYTES: usize = 128;

/// Largest `N` of `RULLST_ACCESS_LOG=1/N`.
pub(crate) const MAX_SAMPLE_INTERVAL: u32 = 1_000_000;

/// Which completed requests the access log writes, from `RULLST_ACCESS_LOG`.
/// Every variant still applies the health-probe and development-poll rules
/// of [`super::console::should_log`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Recording {
    /// Every request (`on`, the default).
    All,
    /// Only responses with a status of 400 or above (`errors`).
    Errors,
    /// Every response with a status of 400 or above, and one in `N` of the
    /// others (`1/N`).
    Sample(NonZeroU32),
    /// No request (`off`): `Server` does not install the access log.
    Off,
}

/// Requests below status 400 that a [`Recording::Sample`] log has seen.
static SAMPLED: AtomicU64 = AtomicU64::new(0);

impl Recording {
    /// Whether a response with `status` is written.
    pub(crate) fn records(self, status: u16) -> bool {
        match self {
            Self::All => true,
            Self::Off => false,
            Self::Errors => status >= 400,
            Self::Sample(interval) => {
                status >= 400 || in_sample(SAMPLED.fetch_add(1, Ordering::Relaxed), interval)
            }
        }
    }
}

/// Whether the `seen`-th sampled request (counting from 0) is written: the
/// first of every `interval`.
fn in_sample(seen: u64, interval: NonZeroU32) -> bool {
    seen.is_multiple_of(u64::from(interval.get()))
}

/// The framework layer that answered a request itself, before the
/// application's handler ran: the CSRF check (and its machine-endpoint
/// authentication), the WAF, the rate limiter, Traffic Shield or lifecycle
/// admission. The layer records it on its response, so the access log names
/// it (`refused by csrf`), also on the line that
/// [`super::console::refusal_log_middleware`] writes outside those layers,
/// which the access log inside them never sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Refusal(&'static str);

impl Refusal {
    /// The CSRF check of the security baseline.
    pub(crate) const CSRF: Self = Self("csrf");
    /// The authentication of a `Server::with_machine_endpoints` route.
    pub(crate) const MACHINE_ENDPOINT: Self = Self("machine-endpoint");
    /// The WAF of the security baseline.
    pub(crate) const WAF: Self = Self("waf");
    /// `Server::rate_limit` (`rate_limit_middleware`).
    pub(crate) const RATE_LIMIT: Self = Self("rate-limit");
    /// Traffic Shield's load shedding.
    pub(crate) const TRAFFIC_SHIELD: Self = Self("traffic-shield");
    /// Lifecycle admission while the application is not ready or drains.
    pub(crate) const LIFECYCLE: Self = Self("lifecycle");

    /// The layer's name on an access-log line.
    pub(crate) fn layer(self) -> &'static str {
        self.0
    }
}

/// `response`, recorded as answered by `layer` ([`Refusal`]).
pub(crate) fn refused(mut response: Response, layer: Refusal) -> Response {
    response.extensions_mut().insert(layer);
    response
}

/// The access-log settings `Server` resolves at start-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccessLog {
    /// Whether successful `/health` and `/ready` requests are logged.
    pub(crate) log_health_probes: bool,
    /// Whether lines show the matched route instead of the request path.
    pub(crate) route_paths: bool,
    /// Which requests are written.
    pub(crate) recording: Recording,
}

/// The path an access-log line shows.
#[derive(Debug, Clone)]
pub(crate) enum ShownPath {
    /// The route that matched, shared with the request.
    Route(MatchedPath),
    /// The request path, or its bounded form.
    Text(String),
}

impl ShownPath {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Route(route) => route.as_str(),
            Self::Text(text) => text,
        }
    }
}

impl AccessLog {
    /// The path a line shows for a request to `path` with `extensions`.
    pub(crate) fn shown_path(self, path: &str, extensions: &Extensions) -> ShownPath {
        self.shown_path_for(path, extensions.get::<MatchedPath>())
    }

    /// The path a line shows for a request to `path` that matched `route`.
    pub(crate) fn shown_path_for(self, path: &str, route: Option<&MatchedPath>) -> ShownPath {
        if !self.route_paths {
            return ShownPath::Text(path.to_string());
        }
        match route {
            Some(route) => ShownPath::Route(route.clone()),
            None => ShownPath::Text(bounded_path(path)),
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
        recording: Recording::All,
    };
    const PATHS: AccessLog = AccessLog {
        log_health_probes: true,
        route_paths: false,
        recording: Recording::All,
    };

    #[test]
    fn recordings_choose_statuses_and_samples() {
        for status in [200, 302, 399] {
            assert!(Recording::All.records(status));
            assert!(!Recording::Errors.records(status));
            assert!(!Recording::Off.records(status));
        }
        for status in [400, 404, 503] {
            assert!(Recording::All.records(status));
            assert!(Recording::Errors.records(status));
            assert!(!Recording::Off.records(status));
        }
        let three = NonZeroU32::new(3).unwrap();
        let picked: Vec<bool> = (0..7).map(|seen| in_sample(seen, three)).collect();
        assert_eq!(picked, [true, false, false, true, false, false, true]);
        let one = NonZeroU32::new(1).unwrap();
        assert!((0..4).all(|seen| in_sample(seen, one)));
        // Only this test samples, so 12 consecutive successes write exactly 3.
        let sample = Recording::Sample(NonZeroU32::new(4).unwrap());
        assert_eq!((0..12).filter(|_| sample.records(200)).count(), 3);
        assert!((0..5).all(|_| sample.records(500)));
    }

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
            // rullst-access: public — test fixture; the token is the link's capability.
            .route("/booking/{token}", axum::routing::get(|| async { "ok" }))
            .nest(
                "/api",
                // rullst-access: public — test fixture that only checks the logged route.
                axum::Router::new().route("/users/{id}", axum::routing::get(|| async { "ok" })),
            )
            .layer(axum::middleware::from_fn(
                move |request: axum::extract::Request, next: axum::middleware::Next| {
                    let record = record.clone();
                    async move {
                        let path = log.shown_path(request.uri().path(), request.extensions());
                        record.lock().unwrap().push(path.as_str().to_string());
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
