//! The per-launch key keeps other local accounts out (`TM-STUDIO-3`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::router_tests::{ORIGIN, Studio, request, session, studio};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use tower::ServiceExt;

async fn send(studio: &Studio, request: Request<Body>) -> (StatusCode, Vec<String>, String) {
    let response = studio.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect();
    let location = response
        .headers()
        .get(header::LOCATION)
        .map(|value| value.to_str().unwrap().to_string())
        .unwrap_or_default();
    (status, cookies, location)
}

fn get(uri: &str, headers: &[(&str, &str)]) -> Request<Body> {
    request(Method::GET, uri, "127.0.0.1:40000", headers, "")
}

#[tokio::test]
async fn the_printed_address_carries_a_fresh_256_bit_key() {
    let (first, second) = (studio().await, studio().await);
    let key = &first.state.launch.key;
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_ne!(key, &second.state.launch.key);
    assert_ne!(key, &first.state.form_token);
    assert_eq!(
        first.state.launch.address(),
        format!("http://127.0.0.1:5556/studio?key={key}")
    );
    assert_eq!(first.state.launch.cookie_name(), "rullst_studio_5556");
}

#[tokio::test]
async fn a_browser_without_the_key_sees_nothing() {
    let studio = studio().await;
    for headers in [
        &[][..],
        &[("cookie", "rullst_studio_5556=0000")][..],
        &[("cookie", "rullst_studio_7000=0000")][..],
    ] {
        let response = studio
            .app
            .clone()
            .oneshot(get("/studio", headers))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{headers:?}");
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("Rullst Studio is locked"), "{body}");
        assert!(!body.contains(&studio.state.form_token));
    }
    // The right key under another port's cookie name does not count either.
    let other_port = format!("rullst_studio_7000={}", studio.state.launch.key);
    let (status, _, _) = send(&studio, get("/studio", &[("cookie", &other_port)])).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A valid form token without the session cookie changes nothing.
    let form = format!(
        "form_token={}&name=shop&base_url=https%3A%2F%2Fshop.example&token=t",
        studio.state.form_token
    );
    let post = request(
        Method::POST,
        "/studio/projects",
        "127.0.0.1:40000",
        &[
            ("content-type", "application/x-www-form-urlencoded"),
            ("origin", ORIGIN),
        ],
        &form,
    );
    let (status, _, _) = send(&studio, post).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(studio.state.store.load().unwrap().is_empty());
}

#[tokio::test]
async fn opening_the_printed_address_sets_the_session_cookie() {
    let studio = studio().await;
    let key = studio.state.launch.key.clone();
    let (status, cookies, location) = send(&studio, get(&format!("/studio?key={key}"), &[])).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location, "/studio");
    assert_eq!(
        cookies,
        [format!(
            "rullst_studio_5556={key}; Path=/; HttpOnly; SameSite=Strict"
        )]
    );
    // Among other parameters too, but never through a POST or a wrong key.
    let (status, _, _) = send(&studio, get(&format!("/studio?a=1&key={key}"), &[])).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let post = request(
        Method::POST,
        &format!("/studio/projects?key={key}"),
        "127.0.0.1:40000",
        &[("origin", ORIGIN)],
        "",
    );
    let (status, cookies, _) = send(&studio, post).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(cookies.is_empty());
    let wrong = format!("/studio?key={}", "0".repeat(64));
    let (status, cookies, _) = send(&studio, get(&wrong, &[])).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(cookies.is_empty());
}

#[tokio::test]
async fn the_session_cookie_opens_every_page_among_other_cookies() {
    let studio = studio().await;
    let cookie = format!("theme=dark; {}; other=1", session(&studio));
    for path in ["/studio", "/studio/alerts", "/studio/settings"] {
        let (status, cookies, _) = send(&studio, get(path, &[("cookie", &cookie)])).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(cookies.is_empty(), "{path}");
    }
}
