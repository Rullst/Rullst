use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Request},
    http::StatusCode,
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

    if is_loopback {
        next.run(request).await
    } else {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::FORBIDDEN;
        response
    }
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
}
