use axum::{
    Router,
    routing::{get, post},
};
use rullst::security::{csrf_middleware, pii_masking_middleware, waf_middleware};
use rullst::testing::TestApp;

fn build_security_app() -> Router {
    Router::new()
        .route("/get", get(|| async { "GET OK" }))
        .route("/post", post(|| async { "POST OK" }))
}

#[tokio::test]
async fn test_csrf_flow() {
    let app =
        TestApp::new(build_security_app().route_layer(axum::middleware::from_fn(csrf_middleware)));

    // 1. GET request sets CSRF cookie
    let res_get = app.get("/get").await;
    res_get.assert_status(200);
    res_get.assert_has_cookie("rullst_csrf");
    let token = res_get.cookie_value("rullst_csrf").unwrap();

    // 2. GET request with existing CSRF cookie doesn't overwrite it
    let res_get_existing = app
        .get("/get")
        .header("cookie", format!("rullst_csrf={}", token))
        .await;
    res_get_existing.assert_status(200);

    // 3. POST request without cookie -> 403 Forbidden
    let res_post_no_cookie = app.post("/post").await;
    res_post_no_cookie.assert_status(403);
    res_post_no_cookie.assert_see("CSRF token cookie missing");

    // 4. POST request with cookie but without token -> 403 Forbidden
    let res_post_no_token = app
        .post("/post")
        .header("cookie", format!("rullst_csrf={}", token))
        .await;
    res_post_no_token.assert_status(403);

    // 5. POST request with matching cookie and X-CSRF-Token header -> 200 OK
    let res_post_header = app
        .post("/post")
        .header("cookie", format!("rullst_csrf={}", token))
        .header("X-CSRF-Token", &token)
        .await;
    res_post_header.assert_status(200);
    res_post_header.assert_see("POST OK");

    // 6. POST request with matching cookie and form token -> 200 OK
    let form_data = [("_token", &token)];
    let res_post_form = app
        .post("/post")
        .header("cookie", format!("rullst_csrf={}", token))
        .form(&form_data)
        .await;
    res_post_form.assert_status(200);
    res_post_form.assert_see("POST OK");

    // 7. POST request with mismatched token -> 403 Forbidden
    let res_post_mismatch = app
        .post("/post")
        .header("cookie", format!("rullst_csrf={}", token))
        .header("X-CSRF-Token", "wrong_token_12345678901234567890123")
        .await;
    res_post_mismatch.assert_status(403);
}

#[tokio::test]
async fn test_pii_masking() {
    let pii_app = Router::new()
        .route(
            "/pii",
            get(|| async { "Email: venelouis@rullst.com, Card: 1234-5678-1234-5678" }),
        )
        .layer(axum::middleware::from_fn(pii_masking_middleware));
    let app = TestApp::new(pii_app);

    let res = app.get("/pii").await;
    res.assert_status(200);
    res.assert_see("Email: v********@rullst.com");
    res.assert_see("Card: ****-****-****-5678");
}

#[tokio::test]
async fn test_waf_referrer_and_cookie() {
    let waf_app = Router::new()
        .route("/waf", get(|| async { "WAF OK" }))
        .layer(axum::middleware::from_fn(waf_middleware));
    let app = TestApp::new(waf_app);

    // 1. Malicious referrer -> 403 Forbidden
    let res_ref = app
        .get("/waf")
        .header("referer", "http://attacker.com/sql?q=drop table users;")
        .await;
    res_ref.assert_status(403);
    res_ref.assert_see("Access Denied: Malicious pattern detected by Rullst Shield WAF");

    // 2. Malicious cookie -> 403 Forbidden
    let res_cookie = app
        .get("/waf")
        .header("cookie", "session=123; tracking=select * from admin;")
        .await;
    res_cookie.assert_status(403);

    // 3. Normal request -> 200 OK
    let res_normal = app.get("/waf").header("referer", "http://google.com").await;
    res_normal.assert_status(200);
}
