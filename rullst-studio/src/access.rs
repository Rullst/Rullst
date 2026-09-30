use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::Response,
};
use std::{fmt, net::SocketAddr};

/// Explicit capability for serving Studio to verified loopback peers in a
/// debug build.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct LocalStudioAccess {
    _private: (),
}

/// Request-local proof that the debug and loopback Studio boundary accepted
/// the peer, host authority, and (for unsafe methods) same-origin request.
///
/// This type remains crate-private so application code cannot manufacture the
/// marker and bypass the access middleware for privileged Studio handlers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VerifiedLocalStudioAccess;

/// Response of a privileged Studio handler reached without the marker, for
/// example through a raw router mounted outside [`LocalStudioAccess`].
pub(crate) fn verified_local_access_required() -> Response {
    let mut response = Response::new(Body::from("Verified local Studio access is required"));
    *response.status_mut() = StatusCode::FORBIDDEN;
    response
}

impl LocalStudioAccess {
    /// Opts in to the debug-only, loopback-verified Studio boundary.
    pub const fn loopback_only() -> Self {
        Self { _private: () }
    }

    /// Applies the Studio loopback boundary to a router.
    pub fn protect_router<S>(&self, router: Router<S>) -> Result<Router<S>, StudioBuildError>
    where
        S: Clone + Send + Sync + 'static,
    {
        if !cfg!(debug_assertions) {
            return Err(StudioBuildError::LocalAccessRequiresDebugBuild);
        }
        Ok(router.layer(axum::middleware::from_fn(loopback_only_middleware)))
    }
}

/// Errors returned while constructing a Studio access boundary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioBuildError {
    /// Credential-free Studio access is unavailable in release builds.
    LocalAccessRequiresDebugBuild,
    /// The OS could not provide the key used for opaque local action tokens.
    RandomnessUnavailable,
}

impl fmt::Display for StudioBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalAccessRequiresDebugBuild => formatter.write_str(
                "Studio local access requires a debug build; shared environments need an application-owned authenticated boundary",
            ),
            Self::RandomnessUnavailable => formatter.write_str(
                "Studio could not obtain operating-system randomness for local action tokens",
            ),
        }
    }
}

impl std::error::Error for StudioBuildError {}

/// Referrer policy stamped on Studio responses.
///
/// Browsers derive a form POST's `Origin` from the submitting document's
/// policy: under `no-referrer` even a same-origin POST carries `Origin: null`,
/// which the check below cannot verify. `same-origin` keeps the real origin on
/// Studio's own form submissions and still sends no referrer to other origins.
const STUDIO_REFERRER_POLICY: &str = "same-origin";

async fn loopback_only_middleware(mut request: Request, next: Next) -> Response {
    let is_loopback = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|connection| connection.0.ip().is_loopback());

    let local_host = local_host_authority(request.headers());
    let origin_valid = request
        .headers()
        .get(header::ORIGIN)
        .map(|origin| {
            same_origin(origin, local_host.as_deref())
                || opaque_origin_from_same_origin_document(origin, request.headers())
        })
        .unwrap_or(true);
    let unsafe_method = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let unsafe_origin_present = !unsafe_method || request.headers().contains_key(header::ORIGIN);

    if is_loopback && local_host.is_some() && origin_valid && unsafe_origin_present {
        request.extensions_mut().insert(VerifiedLocalStudioAccess);
        let mut response = next.run(request).await;
        response
            .headers_mut()
            .entry(header::CACHE_CONTROL)
            .or_insert(axum::http::HeaderValue::from_static("no-store"));
        response
            .headers_mut()
            .entry(header::X_CONTENT_TYPE_OPTIONS)
            .or_insert(axum::http::HeaderValue::from_static("nosniff"));
        response
            .headers_mut()
            .entry(header::X_FRAME_OPTIONS)
            .or_insert(axum::http::HeaderValue::from_static("DENY"));
        response
            .headers_mut()
            .entry(header::REFERRER_POLICY)
            .or_insert(axum::http::HeaderValue::from_static(STUDIO_REFERRER_POLICY));
        response
    } else {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::FORBIDDEN;
        response
    }
}

fn local_host_authority(headers: &HeaderMap) -> Option<String> {
    let authority = headers
        .get(header::HOST)?
        .to_str()
        .ok()?
        .parse::<axum::http::uri::Authority>()
        .ok()?;
    let host = authority.host();
    let ip_host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let is_local = host.eq_ignore_ascii_case("localhost")
        || ip_host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    is_local.then(|| authority.as_str().to_ascii_lowercase())
}

/// Accepts `Origin: null` only when Fetch Metadata proves that a document of
/// this exact origin sent the request. Browsers send that combination for a
/// same-origin form POST when a host layer replaces Studio's referrer policy
/// with `no-referrer`. `Sec-Fetch-Site` is a forbidden request header that page
/// scripts cannot set; cross-site and same-site documents (for example another
/// local port) receive `cross-site` or `same-site` and stay rejected, as does a
/// bare `Origin: null`.
fn opaque_origin_from_same_origin_document(
    origin: &axum::http::HeaderValue,
    headers: &HeaderMap,
) -> bool {
    origin.as_bytes() == b"null"
        && headers
            .get("sec-fetch-site")
            .is_some_and(|site| site.as_bytes() == b"same-origin")
}

fn same_origin(origin: &axum::http::HeaderValue, local_authority: Option<&str>) -> bool {
    let Some(local_authority) = local_authority else {
        return false;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    let scheme_allowed = origin
        .scheme_str()
        .is_some_and(|scheme| matches!(scheme, "http" | "https"));
    scheme_allowed
        && origin
            .authority()
            .is_some_and(|authority| authority.as_str().eq_ignore_ascii_case(local_authority))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use axum::{http::Request, routing::get};
    use tower::ServiceExt;

    fn request_from(peer: Option<&str>) -> Request<Body> {
        let mut request = Request::builder()
            .uri("/")
            .header(header::HOST, "127.0.0.1:5555")
            .body(Body::empty())
            .expect("valid Studio test request");
        if let Some(peer) = peer {
            request.extensions_mut().insert(ConnectInfo(
                peer.parse::<SocketAddr>().expect("valid Studio test peer"),
            ));
        }
        request
    }

    #[tokio::test]
    // TM-STUDIO-01: the built-in local boundary rejects remote and unknown peers.
    async fn protected_router_accepts_only_a_verified_loopback_peer() {
        let access = LocalStudioAccess::loopback_only();
        let protected =
            access.protect_router(Router::new().route("/", get(|| async { StatusCode::OK })));

        if !cfg!(debug_assertions) {
            assert!(matches!(
                protected,
                Err(StudioBuildError::LocalAccessRequiresDebugBuild)
            ));
            return;
        }

        let router = protected.expect("debug Studio access");

        let local = router
            .clone()
            .oneshot(request_from(Some("127.0.0.1:42000")))
            .await
            .expect("local Studio response");
        assert_eq!(local.status(), StatusCode::OK);
        assert_eq!(
            local.headers().get(header::CACHE_CONTROL),
            Some(&axum::http::HeaderValue::from_static("no-store"))
        );
        assert_eq!(
            local.headers().get(header::X_FRAME_OPTIONS),
            Some(&axum::http::HeaderValue::from_static("DENY"))
        );

        let remote = router
            .clone()
            .oneshot(request_from(Some("192.0.2.20:42000")))
            .await
            .expect("remote Studio response");
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);

        let missing = router
            .oneshot(request_from(None))
            .await
            .expect("missing-peer Studio response");
        assert_eq!(missing.status(), StatusCode::FORBIDDEN);
    }

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn protected_router_rejects_rebinding_and_cross_origin_mutations() {
        let router = LocalStudioAccess::loopback_only()
            .protect_router(
                Router::new().route("/mutate", axum::routing::post(|| async { StatusCode::OK })),
            )
            .expect("debug Studio access");

        let request = |host: &'static str, origin: Option<&'static str>| {
            let mut builder = Request::builder()
                .method(Method::POST)
                .uri("/mutate")
                .header(header::HOST, host);
            if let Some(origin) = origin {
                builder = builder.header(header::ORIGIN, origin);
            }
            let mut request = builder.body(Body::empty()).expect("valid request");
            request.extensions_mut().insert(ConnectInfo(
                "127.0.0.1:42000"
                    .parse::<SocketAddr>()
                    .expect("loopback peer"),
            ));
            request
        };

        let same_origin = router
            .clone()
            .oneshot(request("127.0.0.1:5555", Some("http://127.0.0.1:5555")))
            .await
            .expect("same-origin response");
        assert_eq!(same_origin.status(), StatusCode::OK);

        for denied in [
            request("attacker.example", Some("http://attacker.example")),
            request("127.0.0.1:5555", Some("https://attacker.example")),
            request("127.0.0.1:5555", None),
        ] {
            let response = router
                .clone()
                .oneshot(denied)
                .await
                .expect("denied Studio response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }

    /// Headers a browser attaches to a plain HTML form POST: the Fetch
    /// standard's "append a request `Origin` header" step plus Fetch Metadata.
    /// Chrome 154 was observed to send exactly these values for each case.
    #[cfg(debug_assertions)]
    fn browser_form_post(
        referrer_policy: &str,
        document_origin: &str,
        studio_origin: &str,
    ) -> (String, &'static str) {
        let host = |origin: &str| {
            let authority = origin.split_once("://").map_or(origin, |(_, rest)| rest);
            authority.split(':').next().unwrap_or(authority).to_string()
        };
        let same = document_origin == studio_origin;
        let downgrade = document_origin.starts_with("https:") && studio_origin.starts_with("http:");
        let origin = match referrer_policy {
            "no-referrer" => "null",
            "same-origin" if !same => "null",
            "no-referrer-when-downgrade" | "strict-origin" | "strict-origin-when-cross-origin"
                if downgrade =>
            {
                "null"
            }
            _ => document_origin,
        };
        let site = if same {
            "same-origin"
        } else if host(document_origin) == host(studio_origin) {
            "same-site"
        } else {
            "cross-site"
        };
        (origin.to_string(), site)
    }

    #[cfg(debug_assertions)]
    #[tokio::test]
    // TM-STUDIO-06: Studio's own forms stay writable in a real browser, while
    // cross-origin documents stay rejected whatever referrer policy they pick.
    async fn browser_form_posts_follow_the_served_referrer_policy() {
        let router = LocalStudioAccess::loopback_only()
            .protect_router(
                Router::new()
                    .route("/page", get(|| async { StatusCode::OK }))
                    .route("/mutate", axum::routing::post(|| async { StatusCode::OK })),
            )
            .expect("debug Studio access");
        let studio = "http://127.0.0.1:5555";
        let send = |method: Method, path: &str, origin: Option<&str>, site: Option<&str>| {
            let mut builder = Request::builder()
                .method(method)
                .uri(path)
                .header(header::HOST, "127.0.0.1:5555");
            if let Some(origin) = origin {
                builder = builder.header(header::ORIGIN, origin);
            }
            if let Some(site) = site {
                builder = builder.header("sec-fetch-site", site);
            }
            let mut request = builder.body(Body::empty()).expect("valid request");
            request.extensions_mut().insert(ConnectInfo(
                "127.0.0.1:42000"
                    .parse::<SocketAddr>()
                    .expect("loopback peer"),
            ));
            router.clone().oneshot(request)
        };

        let page = send(Method::GET, "/page", None, None)
            .await
            .expect("Studio page response");
        let served_policy = page
            .headers()
            .get(header::REFERRER_POLICY)
            .and_then(|value| value.to_str().ok())
            .expect("Studio pages declare a referrer policy")
            .to_string();
        assert_eq!(served_policy, "same-origin");

        // A form on a Studio page, under the policy Studio serves and under a
        // host layer that replaced it with `no-referrer`.
        for policy in [served_policy.as_str(), "no-referrer"] {
            let (origin, site) = browser_form_post(policy, studio, studio);
            let response = send(Method::POST, "/mutate", Some(&origin), Some(site))
                .await
                .expect("Studio form response");
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Studio form under {policy}"
            );
        }

        // Other documents, including another local port (same-site), choose
        // their own policy; none of their form posts may reach a handler.
        for document in ["http://attacker.example", "http://127.0.0.1:3000"] {
            for policy in [
                "no-referrer",
                "same-origin",
                "strict-origin-when-cross-origin",
            ] {
                let (origin, site) = browser_form_post(policy, document, studio);
                let response = send(Method::POST, "/mutate", Some(&origin), Some(site))
                    .await
                    .expect("foreign form response");
                assert_eq!(
                    response.status(),
                    StatusCode::FORBIDDEN,
                    "{document} under {policy}"
                );
            }
        }

        // `Origin: null` is never proof on its own, nor with weaker metadata.
        for site in [None, Some("same-site"), Some("cross-site"), Some("none")] {
            let response = send(Method::POST, "/mutate", Some("null"), site)
                .await
                .expect("opaque-origin response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{site:?}");
        }
        let response = send(
            Method::POST,
            "/mutate",
            Some("http://attacker.example"),
            Some("same-origin"),
        )
        .await
        .expect("mismatched-origin response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn protected_router_fails_closed_in_release() {
        let protected = LocalStudioAccess::loopback_only().protect_router(
            Router::<()>::new().route("/mutate", axum::routing::post(|| async { StatusCode::OK })),
        );

        assert!(matches!(
            protected,
            Err(StudioBuildError::LocalAccessRequiresDebugBuild)
        ));
    }
}
