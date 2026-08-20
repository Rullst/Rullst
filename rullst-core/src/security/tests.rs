#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::csrf::{extract_token_from_body, generate_csrf_token, is_csrf_exempt_path};
use super::headers::headers_middleware;
use super::pii::mask_pii;
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
async fn test_headers_middleware_injects_security_headers() {
    use axum::http::{Request, StatusCode};

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "OK" }))
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
    assert_eq!(headers.get("X-XSS-Protection").unwrap(), "1; mode=block");
    assert_eq!(
        headers.get("Strict-Transport-Security").unwrap(),
        "max-age=31536000; includeSubDomains; preload"
    );
    assert_eq!(
        headers.get("Permissions-Policy").unwrap(),
        "geolocation=(), camera=(), microphone=()"
    );
}

#[test]
fn test_generate_csrf_token() {
    let token1 = generate_csrf_token();
    let token2 = generate_csrf_token();
    assert_eq!(token1.len(), 32);
    assert_eq!(token2.len(), 32);
    assert_ne!(token1, token2);
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
