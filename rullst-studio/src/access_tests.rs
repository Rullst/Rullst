#![allow(clippy::expect_used)]

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

    // A dual-stack listener reports IPv4 clients as IPv4-mapped peers.
    for (peer, expected) in [
        ("[::ffff:127.0.0.1]:42000", StatusCode::OK),
        ("[::ffff:192.0.2.20]:42000", StatusCode::FORBIDDEN),
    ] {
        let response = router
            .clone()
            .oneshot(request_from(Some(peer)))
            .await
            .expect("mapped-peer Studio response");
        assert_eq!(response.status(), expected, "{peer}");
    }

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

#[cfg(debug_assertions)]
#[tokio::test]
// TM-STUDIO-01: a same-host proxy or tunnel connects from loopback.
async fn relayed_requests_and_secure_environments_are_refused() {
    let router = LocalStudioAccess::loopback_only()
        .protect_router(Router::new().route("/", get(|| async { StatusCode::OK })))
        .expect("debug Studio access");
    let status = |request: Request<Body>| {
        let router = router.clone();
        async move {
            router
                .oneshot(request)
                .await
                .expect("Studio response")
                .status()
        }
    };
    let local = || request_from(Some("127.0.0.1:42000"));

    assert_eq!(status(local()).await, StatusCode::OK);
    for (name, value) in [
        ("forwarded", "for=203.0.113.7"),
        ("x-forwarded-for", "203.0.113.7"),
        ("x-forwarded-host", "studio.example"),
        ("x-forwarded-proto", "https"),
        ("x-real-ip", "203.0.113.7"),
        ("via", "1.1 tunnel"),
        ("true-client-ip", "203.0.113.7"),
    ] {
        let mut request = local();
        request
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
        assert_eq!(status(request).await, StatusCode::FORBIDDEN, "{name}");
    }
    // nginx proxies with HTTP/1.0 and adds no forwarding header by default.
    for (version, expected) in [
        (axum::http::Version::HTTP_10, StatusCode::FORBIDDEN),
        (axum::http::Version::HTTP_11, StatusCode::OK),
        (axum::http::Version::HTTP_2, StatusCode::OK),
    ] {
        let mut request = local();
        *request.version_mut() = version;
        assert_eq!(status(request).await, expected, "{version:?}");
    }
    for (environment, expected) in [
        (Environment::Development, StatusCode::OK),
        (Environment::Test, StatusCode::OK),
        (Environment::Staging, StatusCode::FORBIDDEN),
        (Environment::Production, StatusCode::FORBIDDEN),
    ] {
        let mut request = local();
        request.extensions_mut().insert(environment);
        assert_eq!(status(request).await, expected, "{environment}");
    }
}

#[test]
fn build_errors_explain_themselves() {
    assert_eq!(
        StudioBuildError::LocalAccessRequiresDebugBuild.to_string(),
        "Studio local access is refused: it runs only in a debug build whose environment (RULLST_ENV, APP_ENV, .env or [app] env) is development or test; shared environments need an application-owned authenticated boundary"
    );
    assert_eq!(
        StudioBuildError::RandomnessUnavailable.to_string(),
        "Studio could not obtain operating-system randomness for local action tokens"
    );
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
