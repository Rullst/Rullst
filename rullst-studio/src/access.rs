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
}

impl fmt::Display for StudioBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalAccessRequiresDebugBuild => formatter.write_str(
                "Studio local access requires a debug build; shared environments need an application-owned authenticated boundary",
            ),
        }
    }
}

impl std::error::Error for StudioBuildError {}

async fn loopback_only_middleware(request: Request, next: Next) -> Response {
    let is_loopback = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|connection| connection.0.ip().is_loopback());

    let local_host = local_host_authority(request.headers());
    let origin_valid = request
        .headers()
        .get(header::ORIGIN)
        .map(|origin| same_origin(origin, local_host.as_deref()))
        .unwrap_or(true);
    let unsafe_method = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let unsafe_origin_present = !unsafe_method || request.headers().contains_key(header::ORIGIN);

    if is_loopback && local_host.is_some() && origin_valid && unsafe_origin_present {
        next.run(request).await
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
}
