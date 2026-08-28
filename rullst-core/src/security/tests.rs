#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::csrf::{CsrfToken, extract_token_from_body, generate_csrf_token, is_csrf_exempt_path};
use super::headers::headers_middleware;
use super::pii::{mask_pii, pii_masking_middleware};
use super::waf::waf_middleware;

#[test]
fn test_mask_pii_credit_card() {
    let raw = "My card number is 1234-5678-1234-5678 and it is secret.";
    let masked = mask_pii(raw);
    assert!(masked.contains("****-****-****-5678"));
    assert!(!masked.contains("1234-5678-1234"));

    // Space-separated credit card to catch `replace == with !=` mutant on `c == ' '`
    let raw_spaces = "Another card 4321 8765 4321 8765 here.";
    let masked_spaces = mask_pii(raw_spaces);
    assert!(masked_spaces.contains("**** **** **** 8765"));

    // Ensure that too many spaces/hyphens prevents it from being recognized as a single card.
    // This catches the cargo-mutants mutation: replace `+=` with `*=` on `non_digits`.
    let raw_gaps = "1234---5678-1234-5678";
    let masked_gaps = mask_pii(raw_gaps);
    assert_eq!(masked_gaps, raw_gaps);
}

#[test]
fn test_mask_pii_edge_cases() {
    assert_eq!(mask_pii(""), "");
    assert_eq!(mask_pii("a@b.c"), "a@b.c");
    assert_eq!(
        mask_pii("admin123@longdomain.com"),
        "a*******@longdomain.com"
    );
    assert_eq!(mask_pii("invalid_email@"), "invalid_email@");
    assert_eq!(mask_pii("my card is 1234"), "my card is 1234");
}

#[test]
fn test_mask_pii_email() {
    let raw = "Contact me at venelouis@rullst.com or admin@domain.org.";
    let masked = mask_pii(raw);
    assert!(masked.contains("v********@rullst.com"));
    assert!(masked.contains("a****@domain.org"));
}

#[tokio::test]
async fn pii_middleware_only_rewrites_safe_buffered_text() {
    use axum::{
        body::Body,
        http::{HeaderValue, Request, Response, StatusCode, header},
        routing::get,
    };
    use tower::ServiceExt;

    async fn text_handler() -> Response<Body> {
        let mut response = Response::new(Body::from("email=user@example.com"));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        response
            .headers_mut()
            .insert(header::ETAG, HeaderValue::from_static("\"stale\""));
        response
    }

    async fn binary_handler() -> Response<Body> {
        let payload = b"\xffuser@example.com".to_vec();
        let mut response = Response::new(Body::from(payload));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        response
    }

    async fn event_stream_handler() -> Response<Body> {
        let mut response = Response::new(Body::from("data: user@example.com\n\n"));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        response
    }

    async fn oversized_handler() -> Response<Body> {
        let mut payload = vec![b'a'; 2 * 1024 * 1024 + 1];
        payload.extend_from_slice(b" user@example.com");
        let mut response = Response::new(Body::from(payload));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        response
    }

    let app = axum::Router::new()
        .route("/text", get(text_handler))
        .route("/binary", get(binary_handler))
        .route("/events", get(event_stream_handler))
        .route("/oversized", get(oversized_handler))
        .layer(axum::middleware::from_fn(pii_masking_middleware));

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/text").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::ETAG).is_none());
    let declared_length = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .unwrap();
    assert_eq!(declared_length, body.len());
    assert_eq!(body.as_ref(), b"email=u***@example.com");

    for (path, limit) in [
        ("/binary", 1_024usize),
        ("/events", 1_024usize),
        ("/oversized", 2 * 1024 * 1024 + 10_000),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), limit)
            .await
            .unwrap();
        assert!(
            bytes
                .windows(b"user@example.com".len())
                .any(|window| window == b"user@example.com"),
            "{path} must bypass PII masking without truncation"
        );
    }
}

#[tokio::test]
async fn test_waf_middleware_blocks_malicious_query() {
    use axum::http::{Request, StatusCode};

    // Not currently possible to test axum middlewares easily without setting up an app.
    // We will test `waf_middleware` via a router approach.
    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "OK" }))
        .route_layer(axum::middleware::from_fn(waf_middleware));

    // Use reqwest or tower::ServiceExt to call the app
    let req = Request::builder()
        .uri("/?q=select%20")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let req2 = Request::builder()
        .uri("/?q=hello")
        .body(axum::body::Body::empty())
        .unwrap();
    let res2 = tower::ServiceExt::oneshot(app, req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn waf_inspects_and_preserves_bounded_request_bodies() {
    use axum::{
        body::{Body, Bytes},
        http::{Request, StatusCode, header},
        routing::post,
    };
    use tower::ServiceExt;

    async fn echo(body: Bytes) -> Bytes {
        body
    }

    let app = axum::Router::new()
        .route("/echo", post(echo))
        .route_layer(axum::middleware::from_fn(waf_middleware));

    let attack = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"query":"UNION SELECT password FROM users"}"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(attack).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let clean_payload = br#"{"message":"hello","count":2}"#.to_vec();
    let clean = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(clean_payload.clone()))
        .unwrap();
    let response = app.clone().oneshot(clean).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let echoed = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .unwrap();
    assert_eq!(echoed.as_ref(), clean_payload);

    let oversized = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_LENGTH, (1024 * 1024 + 1).to_string())
        .body(Body::from(vec![b'a'; 1024 * 1024 + 1]))
        .unwrap();
    let response = app.clone().oneshot(oversized).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let encoded = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(Body::from("compressed"))
        .unwrap();
    let response = app.oneshot(encoded).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn test_headers_middleware_injects_security_headers() {
    use super::CspNonce;
    use axum::{
        Extension,
        http::{Request, StatusCode},
    };

    let app = axum::Router::new()
        .route(
            "/",
            axum::routing::get(
                |Extension(nonce): Extension<CspNonce>| async move { nonce.to_string() },
            ),
        )
        .route_layer(axum::middleware::from_fn(headers_middleware));

    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = res.headers();
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
    assert_eq!(headers.get("X-XSS-Protection").unwrap(), "0");
    assert_eq!(
        headers.get("Strict-Transport-Security").unwrap(),
        "max-age=63072000; includeSubDomains; preload"
    );
    assert_eq!(
        headers.get("Permissions-Policy").unwrap(),
        "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
    );
    assert_eq!(
        headers.get("Cross-Origin-Embedder-Policy").unwrap(),
        "require-corp"
    );
    let csp = headers
        .get("Content-Security-Policy")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(!csp.contains("unsafe-inline"));
    assert!(!csp.contains("unsafe-eval"));
    assert!(!csp.contains("{NONCE}"));
    let body = axum::body::to_bytes(res.into_body(), 1_024).await.unwrap();
    let nonce = std::str::from_utf8(&body).unwrap();
    assert!(csp.contains(&format!("'nonce-{nonce}'")));
}

#[test]
fn test_generate_csrf_token() {
    let token1 = generate_csrf_token();
    let token2 = generate_csrf_token();
    assert_eq!(token1.len(), 32);
    assert_eq!(token2.len(), 32);
    assert_ne!(token1, token2);
}

#[tokio::test]
async fn csrf_get_exposes_the_exact_cookie_token_to_form_handlers() {
    use axum::{Extension, body::Body, http::Request, routing::get};
    use tower::ServiceExt;

    let app = axum::Router::new()
        .route(
            "/form",
            get(|Extension(token): Extension<CsrfToken>| async move { token.as_str().to_owned() }),
        )
        .layer(axum::middleware::from_fn(super::csrf::csrf_middleware));

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/form")
                .body(Body::empty())
                .expect("valid first request"),
        )
        .await
        .expect("first response");
    let cookie = first
        .headers()
        .get(axum::http::header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("CSRF cookie")
        .to_owned();
    let first_body = axum::body::to_bytes(first.into_body(), 128)
        .await
        .expect("first body");
    let first_token = std::str::from_utf8(&first_body).expect("UTF-8 token");
    assert!(cookie.starts_with(&format!("rullst_csrf={first_token};")));

    let second = app
        .oneshot(
            Request::builder()
                .uri("/form")
                .header(
                    axum::http::header::COOKIE,
                    format!("rullst_csrf={first_token}"),
                )
                .body(Body::empty())
                .expect("valid second request"),
        )
        .await
        .expect("second response");
    assert!(
        second
            .headers()
            .get(axum::http::header::SET_COOKIE)
            .is_none()
    );
    let second_body = axum::body::to_bytes(second.into_body(), 128)
        .await
        .expect("second body");
    assert_eq!(second_body.as_ref(), first_token.as_bytes());
}

#[test]
fn test_mask_pii() {
    // Valid emails
    assert_eq!(mask_pii("user@example.com"), "u***@example.com");
    assert_eq!(mask_pii("us@example.com"), "u*@example.com");
    assert_eq!(
        mask_pii("a.b+c-d_e%f@example.com"),
        "a**********@example.com"
    );
    assert_eq!(mask_pii("user@sub.example.com"), "u***@sub.example.com");

    // Edge cases that should NOT be masked
    assert_eq!(mask_pii("u@example.com"), "u@example.com"); // username too short
    assert_eq!(mask_pii("user@ex.com"), "u***@ex.com");
    assert_eq!(mask_pii("user@e.c"), "user@e.c"); // domain too short
    assert_eq!(mask_pii("user@examplecom"), "user@examplecom"); // no dot in domain
    assert_eq!(mask_pii("no_at_symbol.com"), "no_at_symbol.com");

    // Mixed content
    assert_eq!(
        mask_pii("Contact me at user@example.com or other@test.com."),
        "Contact me at u***@example.com or o****@test.com."
    );
}

#[test]
fn test_extract_token_from_body() {
    assert_eq!(
        extract_token_from_body(b"_token=secret123"),
        Some("secret123".to_string())
    );
    assert_eq!(extract_token_from_body(b"other=value"), None);
    assert_eq!(extract_token_from_body(b"invalid_body"), None);
}

#[test]
fn test_is_csrf_exempt_path() {
    // Exempt files
    assert!(is_csrf_exempt_path("/robots.txt"));
    assert!(is_csrf_exempt_path("/sitemap.xml"));
    assert!(is_csrf_exempt_path("/favicon.ico"));
    assert!(is_csrf_exempt_path("/static/bundle.js"));
    assert!(is_csrf_exempt_path("/doc.txt"));
    assert!(is_csrf_exempt_path("/feed.xml"));
    assert!(is_csrf_exempt_path("/app.ico"));
    assert!(is_csrf_exempt_path("/schema.json"));
    assert!(is_csrf_exempt_path("/style.css"));
    assert!(is_csrf_exempt_path("/script.js"));
    assert!(is_csrf_exempt_path("/logo.png"));
    assert!(is_csrf_exempt_path("/photo.jpg"));
    assert!(is_csrf_exempt_path("/image.jpeg"));
    assert!(is_csrf_exempt_path("/icon.svg"));
    assert!(is_csrf_exempt_path("/banner.webp"));
    assert!(is_csrf_exempt_path("/module.wasm"));

    // Non-exempt application routes
    assert!(!is_csrf_exempt_path("/dashboard"));
    assert!(!is_csrf_exempt_path("/users/login"));
    assert!(!is_csrf_exempt_path("/posts/create"));
    assert!(!is_csrf_exempt_path("/api/v1/orders"));
}

#[tokio::test]
async fn test_tenant_guard_middleware() {
    use super::tenant_guard::{
        TenantContext, strict_tenant_guard_middleware, tenant_guard_middleware,
    };
    use axum::extract::Extension;
    use tower_service::Service;

    let app = axum::Router::new()
        .route(
            "/tenant-data",
            axum::routing::get(|ext: Option<Extension<TenantContext>>| async move {
                match ext {
                    Some(Extension(ctx)) => format!("tenant: {}", ctx.tenant_id),
                    None => "no tenant".to_string(),
                }
            }),
        )
        .layer(axum::middleware::from_fn(tenant_guard_middleware));

    let mut service = app;

    // Client-controlled tenant headers never establish authorization context.
    let req = axum::http::Request::builder()
        .uri("/tenant-data")
        .header("X-Tenant-ID", "org_12345")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), "no tenant");

    // Trusted authentication middleware inserts a validated extension.
    let authenticated_app = axum::Router::new()
        .route(
            "/tenant-data",
            axum::routing::get(|Extension(ctx): Extension<TenantContext>| async move {
                format!("tenant: {}", ctx.tenant_id)
            }),
        )
        .layer(axum::middleware::from_fn(tenant_guard_middleware))
        .layer(Extension(TenantContext::try_new("org_12345").unwrap()));
    let mut authenticated_service = authenticated_app;
    let req = axum::http::Request::builder()
        .uri("/tenant-data")
        .header("X-Tenant-ID", "attacker-selected")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = authenticated_service.call(req).await.unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(
        String::from_utf8(bytes.to_vec()).unwrap(),
        "tenant: org_12345"
    );

    // Strict guard requires authenticated context, not a header.
    let strict_app = axum::Router::new()
        .route("/strict-data", axum::routing::get(|| async { "OK" }))
        .layer(axum::middleware::from_fn(strict_tenant_guard_middleware));

    let mut strict_service = strict_app;
    let req = axum::http::Request::builder()
        .uri("/strict-data")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = strict_service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);
}
