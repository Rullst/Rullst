//! Router-level tests of the blog showcase.

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use tower::ServiceExt;

fn test_router() -> rullst::Router {
    let nexus_auth = rullst_nexus::NexusAuthPolicy::basic(
        "integration-fixture-operator",
        "integration-fixture-credential",
    )
    .expect("valid test-only Nexus policy");
    super::router_with_nexus_auth(nexus_auth).expect("blog router")
}

#[tokio::test]
async fn honeypot_button_hits_the_real_deception_middleware() {
    let store = rullst_security::SecurityStore::global();
    let before = store.honeypot_traps_count.load(Ordering::Relaxed);
    let app = test_router().into_axum();
    let mut request = Request::get("/wp-admin")
        .body(Body::empty())
        .expect("honeypot request");
    request.extensions_mut().insert(ConnectInfo(
        "192.0.2.45:4242"
            .parse::<SocketAddr>()
            .expect("test peer address"),
    ));

    let response = app.oneshot(request).await.expect("honeypot response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(store.honeypot_traps_count.load(Ordering::Relaxed) > before);
    let events = store.live_events.lock().expect("security event lock");
    assert!(events.iter().any(|event| {
        event.event_type == "HONEYPOT_TRAP_TRIGGERED"
            && event.client_ip == "192.0.2.45"
            && event.details.contains("/wp-admin")
            && !event.verified_hmac
    }));
}

#[tokio::test]
async fn showcase_forms_render_and_enforce_the_double_submit_csrf_token() {
    // `Server` applies the canonical production baseline around the app. Model
    // that composition here so the example never regresses to two divergent
    // CSRF cookies when it also mounts the middleware explicitly.
    let app = test_router()
        .into_axum()
        .layer(axum::middleware::from_fn(rullst::security::csrf_middleware));

    for path in ["/", "/pricing"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).expect("GET request"))
            .await
            .expect("GET response");
        assert_eq!(response.status(), StatusCode::OK);
        let cookies = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter(|value| value.starts_with("rullst_csrf="))
            .collect::<Vec<_>>();
        assert_eq!(cookies.len(), 1, "one layer must own CSRF emission");
        let token = cookies[0]
            .split(';')
            .next()
            .and_then(|pair| pair.strip_prefix("rullst_csrf="))
            .expect("CSRF token")
            .to_owned();
        let body = to_bytes(response.into_body(), 512 * 1024)
            .await
            .expect("HTML body");
        let html = std::str::from_utf8(&body).expect("UTF-8 HTML");
        assert!(html.contains(&format!("name=\"_token\" value=\"{token}\"")));
    }

    let rejected = app
        .clone()
        .oneshot(
            Request::post("/posts")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("title=Blocked&body=Missing+token"))
                .expect("missing-token request"),
        )
        .await
        .expect("missing-token response");
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let token = rullst::security::generate_csrf_token();
    let accepted = app
        .clone()
        .oneshot(
            Request::post("/posts")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("rullst_csrf={token}"))
                .body(Body::from(format!(
                    "title=Accepted&body=Matching+token&_token={token}"
                )))
                .expect("matching-token request"),
        )
        .await
        .expect("matching-token response");
    assert_eq!(accepted.status(), StatusCode::SEE_OTHER);

    let pricing = app
        .clone()
        .oneshot(
            Request::get("/pricing")
                .body(Body::empty())
                .expect("pricing GET request"),
        )
        .await
        .expect("pricing GET response");
    let cookie = pricing
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("pricing CSRF cookie");
    let token = cookie
        .split(';')
        .next()
        .and_then(|pair| pair.strip_prefix("rullst_csrf="))
        .expect("pricing CSRF token")
        .to_owned();
    let pricing_body = to_bytes(pricing.into_body(), 512 * 1024)
        .await
        .expect("pricing HTML body");
    let pricing_html = std::str::from_utf8(&pricing_body).expect("pricing UTF-8 HTML");
    assert!(pricing_html.contains(&format!("name=\"_token\" value=\"{token}\"")));

    let checkout = app
        .oneshot(
            Request::post("/checkout")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("rullst_csrf={token}"))
                .body(Body::from(format!(
                    "provider=stripe&plan=pro_plan&email=fixture%40example.invalid&_token={token}"
                )))
                .expect("offline checkout fixture request"),
        )
        .await
        .expect("offline checkout fixture response");
    assert_eq!(checkout.status(), StatusCode::OK);
    let checkout_body = to_bytes(checkout.into_body(), 512 * 1024)
        .await
        .expect("offline checkout fixture body");
    let checkout_html = std::str::from_utf8(&checkout_body).expect("offline checkout UTF-8 HTML");
    assert!(checkout_html.contains("OFFLINE FIXTURE"));
}

#[tokio::test]
async fn the_unbuilt_wasm_island_demo_is_not_advertised() {
    let app = test_router().into_axum();
    for path in ["/editor", "/wasm-counter"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).expect("GET request"))
            .await
            .expect("GET response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
    let response = app
        .oneshot(Request::get("/").body(Body::empty()).expect("GET request"))
        .await
        .expect("GET response");
    let body = to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("HTML body");
    let html = std::str::from_utf8(&body).expect("UTF-8 HTML");
    assert!(!html.contains("/editor"));
    assert!(!html.contains("Wasm Island"));
}
