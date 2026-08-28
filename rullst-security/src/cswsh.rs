use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::{StatusCode, Uri, header, uri::Authority},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::str::FromStr;

/// Explicit origin policy for WebSocket upgrades.
#[derive(Clone, Debug, Default)]
pub struct CswsPolicy {
    allowed_origins: Vec<NormalizedOrigin>,
    allow_missing_origin: bool,
}

impl CswsPolicy {
    /// Builds an exact, normalized allowlist of `http`/`https` origins.
    pub fn try_new<I, S>(origins: I) -> Result<Self, CswsPolicyError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let allowed_origins = origins
            .into_iter()
            .map(|origin| {
                NormalizedOrigin::parse(origin.as_ref())
                    .ok_or_else(|| CswsPolicyError::InvalidOrigin(origin.as_ref().to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            allowed_origins,
            allow_missing_origin: false,
        })
    }

    /// Explicitly permits clients that do not send an `Origin` header.
    ///
    /// Keep this disabled for browser/session-authenticated WebSockets. Enable
    /// it only when another mandatory authentication mechanism protects direct
    /// non-browser clients.
    #[must_use]
    pub fn allow_missing_origin(mut self, allow: bool) -> Self {
        self.allow_missing_origin = allow;
        self
    }
}

/// Invalid CSWSH policy configuration.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CswsPolicyError {
    /// An allowlisted origin is not an absolute `http` or `https` origin.
    #[error("invalid WebSocket origin `{0}`")]
    InvalidOrigin(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizedOrigin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl NormalizedOrigin {
    fn parse(value: &str) -> Option<Self> {
        let uri = Uri::from_str(value).ok()?;
        let scheme = uri.scheme_str()?.to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        if uri.query().is_some() || (uri.path() != "/" && !uri.path().is_empty()) {
            return None;
        }
        let authority = uri.authority()?;
        let port = normalize_port(&scheme, authority.port_u16());
        Some(Self {
            scheme,
            host: authority.host().to_ascii_lowercase(),
            port,
        })
    }

    fn matches_host(&self, host: &str) -> bool {
        let Ok(authority) = Authority::from_str(host) else {
            return false;
        };
        self.host.eq_ignore_ascii_case(authority.host())
            && self.port == normalize_port(&self.scheme, authority.port_u16())
    }
}

fn normalize_port(scheme: &str, port: Option<u16>) -> Option<u16> {
    match (scheme, port) {
        ("http", Some(80)) | ("https", Some(443)) => None,
        (_, port) => port,
    }
}

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

        let policy = req.extensions().get::<CswsPolicy>();
        let valid = match origin.and_then(NormalizedOrigin::parse) {
            Some(parsed_origin) => match policy {
                Some(policy) if !policy.allowed_origins.is_empty() => {
                    policy.allowed_origins.contains(&parsed_origin)
                }
                _ => host.is_some_and(|host| parsed_origin.matches_host(host)),
            },
            None if origin.is_none() => policy.is_some_and(|policy| policy.allow_missing_origin),
            None => false,
        };

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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request as HttpRequest, middleware, routing::get};
    use tower::ServiceExt;

    fn guarded_app() -> Router {
        Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(cswsh_guard_middleware))
    }

    #[tokio::test]
    async fn test_cswsh_valid_origin() {
        let req = HttpRequest::builder()
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "app.example.com")
            .header(header::ORIGIN, "https://app.example.com")
            .body(axum::body::Body::empty())
            .unwrap();

        let origin = req.headers()[header::ORIGIN].to_str().unwrap();
        let host = req.headers()[header::HOST].to_str().unwrap();
        assert!(NormalizedOrigin::parse(origin).is_some_and(|parsed| parsed.matches_host(host)));
    }

    #[test]
    fn deceptive_localhost_prefixes_are_rejected() {
        let malicious = NormalizedOrigin::parse("https://localhost.evil.example").unwrap();
        assert!(!malicious.matches_host("localhost"));
        assert!(!malicious.matches_host("127.0.0.1"));

        let deceptive_ip = NormalizedOrigin::parse("http://127.0.0.1.evil.example").unwrap();
        assert!(!deceptive_ip.matches_host("127.0.0.1"));
    }

    #[tokio::test]
    async fn middleware_rejects_a_deceptive_localhost_origin() {
        let request = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "localhost")
            .header(header::ORIGIN, "https://localhost.evil.example")
            .body(Body::empty())
            .unwrap();

        let response = guarded_app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_accepts_only_the_exact_same_host_by_default() {
        let request = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "localhost:3000")
            .header(header::ORIGIN, "http://localhost:3000")
            .body(Body::empty())
            .unwrap();

        let response = guarded_app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn origin_normalization_is_exact_and_port_aware() {
        let origin = NormalizedOrigin::parse("https://APP.EXAMPLE.COM:443").unwrap();
        assert!(origin.matches_host("app.example.com"));
        assert!(!origin.matches_host("app.example.com:444"));
        assert!(NormalizedOrigin::parse("https://app.example.com/path").is_none());
        assert!(NormalizedOrigin::parse("javascript://app.example.com").is_none());
    }

    #[test]
    fn configured_allowlist_rejects_invalid_origins() {
        assert!(CswsPolicy::try_new(["https://app.example.com"]).is_ok());
        assert!(CswsPolicy::try_new(["https://app.example.com/path"]).is_err());
    }

    #[tokio::test]
    async fn explicit_policy_controls_allowlists_and_missing_origins() {
        let policy = CswsPolicy::try_new(["https://trusted.example"])
            .expect("trusted origin should be valid");
        let mut allowlisted = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "internal.example")
            .header(header::ORIGIN, "https://trusted.example")
            .body(Body::empty())
            .unwrap();
        allowlisted.extensions_mut().insert(policy.clone());
        assert_eq!(
            guarded_app().oneshot(allowlisted).await.unwrap().status(),
            StatusCode::OK
        );

        let mut missing_origin = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "internal.example")
            .body(Body::empty())
            .unwrap();
        missing_origin.extensions_mut().insert(policy.clone());
        assert_eq!(
            guarded_app()
                .oneshot(missing_origin)
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );

        let mut explicitly_allowed = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "internal.example")
            .body(Body::empty())
            .unwrap();
        explicitly_allowed
            .extensions_mut()
            .insert(policy.allow_missing_origin(true));
        assert_eq!(
            guarded_app()
                .oneshot(explicitly_allowed)
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn malformed_origins_fail_closed_and_ordinary_http_passes_through() {
        let malformed = HttpRequest::builder()
            .uri("/")
            .header(header::UPGRADE, "websocket")
            .header(header::HOST, "app.example.com")
            .header(header::ORIGIN, "not-an-origin")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            guarded_app().oneshot(malformed).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let ordinary_http = HttpRequest::builder()
            .uri("/")
            .header(header::ORIGIN, "https://untrusted.example")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            guarded_app().oneshot(ordinary_http).await.unwrap().status(),
            StatusCode::OK
        );
    }

    #[test]
    fn malformed_host_headers_never_match_an_origin() {
        let origin = NormalizedOrigin::parse("https://app.example.com").unwrap();
        assert!(!origin.matches_host("not a valid authority"));
    }
}
