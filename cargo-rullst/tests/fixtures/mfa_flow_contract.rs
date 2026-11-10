//! The generated two-step verification (`make:mfa`) through the real
//! generated router, migrations and session cookies: setup with a QR code,
//! confirmation that ends the other sessions, the second step after the
//! password and after a provider sign-in, the attempt limit, turning it off
//! and the reset command.
use rullst::server::{Body, Request, Response, StatusCode};
use tower::ServiceExt;

type App = rullst::web::axum::Router;

/// Each request comes from its own client, so the per-client sign-in budget
/// never applies here.
static NEXT_PEER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);
/// The double-submit CSRF token every request carries.
const CSRF: &str = "mfaflowcontractcsrftoken";
const EMAIL: &str = "second@example.test";
const PASSWORD: &str = "correct-horse-battery";

async fn send(app: &App, method: &str, uri: &str, cookies: &[&str], form: &str) -> Response {
    let n = NEXT_PEER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let peer = std::net::SocketAddr::from(([10, 2, (n >> 8) as u8, n as u8], 40_000));
    let csrf = format!("rullst_csrf={CSRF}");
    let mut all = vec![csrf.as_str()];
    all.extend_from_slice(cookies);
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", all.join("; "));
    if method == "POST" {
        request = request
            .header("content-type", "application/x-www-form-urlencoded")
            .header("x-csrf-token", CSRF);
    }
    let mut request = request.body(Body::from(form.to_owned())).unwrap();
    request
        .extensions_mut()
        .insert(rullst::web::axum::extract::ConnectInfo(peer));
    app.clone().oneshot(request).await.unwrap()
}

async fn get(app: &App, uri: &str, cookies: &[&str]) -> Response {
    send(app, "GET", uri, cookies, "").await
}

async fn post(app: &App, uri: &str, cookies: &[&str], form: &str) -> Response {
    send(app, "POST", uri, cookies, form).await
}

async fn text(response: Response) -> String {
    let bytes = rullst::web::axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn set_cookie(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_owned())
        .find(|cookie| cookie.starts_with(&format!("{name}=")))
}

fn pair(cookie: &str) -> String {
    cookie.split(';').next().unwrap().to_owned()
}

fn location(response: &Response) -> String {
    response.headers()["location"].to_str().unwrap().to_owned()
}

fn redirected(response: &Response, target: &str) {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(response), target);
}

/// The session of a sign-in that went straight to `target`.
fn session(response: &Response, target: &str) -> String {
    redirected(response, target);
    pair(&set_cookie(response, "rullst_session").unwrap())
}

async fn login(app: &App) -> Response {
    post(app, "/login", &[], &format!("email={EMAIL}&password={PASSWORD}")).await
}

/// The challenge cookie of a sign-in that must pass the second step.
fn challenged(response: &Response) -> String {
    redirected(response, "/login/mfa");
    assert!(set_cookie(response, "rullst_session").is_none());
    let cookie = set_cookie(response, "rullst_mfa_challenge").unwrap();
    assert!(
        cookie.ends_with("; Path=/login/mfa; Max-Age=300; HttpOnly; SameSite=Lax"),
        "{cookie}"
    );
    pair(&cookie)
}

async fn secret(account: i32) -> String {
    let (secret,): (String,) =
        rullst::db::sqlx::query_as("SELECT secret FROM user_mfa_factors WHERE user_id = ?")
            .bind(i64::from(account))
            .fetch_one(rullst::orm::Orm::pool().unwrap())
            .await
            .unwrap();
    secret
}

/// Each time step is accepted once; this test runs within one or two steps,
/// so it lets the current step be used again.
async fn rewind(account: i32) {
    rullst::db::sqlx::query(
        "UPDATE user_mfa_factors SET last_accepted_step = last_accepted_step - 2 WHERE user_id = ?",
    )
    .bind(i64::from(account))
    .execute(rullst::orm::Orm::pool().unwrap())
    .await
    .unwrap();
}

fn code(secret: &str) -> String {
    rullst::security_runtime::generate_totp_code(secret).unwrap()
}

fn wrong(secret: &str) -> String {
    let current = code(secret);
    if current == "000000" { "111111".to_owned() } else { "000000".to_owned() }
}

async fn account(email: &str) -> i32 {
    crate::models::user::User::find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

#[tokio::test]
async fn generated_mfa_flow_contract() {
    // `Server` loads Rullst.toml; this contract calls the router directly.
    if let Ok(config) = std::fs::read_to_string("Rullst.toml") {
        let config = rullst::config::RullstConfig::from_toml(&config).unwrap();
        assert!(rullst::config::RullstConfig::set_global(config).is_ok());
    }
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() {
        migration.up().await.unwrap();
    }
    let router = crate::router().unwrap();
    let app = router.as_axum();

    // Registration signs in directly: a new account has no second factor.
    let registered = post(
        app,
        "/register",
        &[],
        &format!("name=Second+Step&email={EMAIL}&password={PASSWORD}"),
    )
    .await;
    let first = session(&registered, "/dashboard");
    let user = account(EMAIL).await;
    let dashboard = text(get(app, "/dashboard", &[&first]).await).await;
    assert!(dashboard.contains("href=\"/account/security\""), "{dashboard}");

    // The security page offers the setup; the setup shows a QR code and the key.
    let page = get(app, "/account/security", &[&first]).await;
    assert_eq!(page.status(), StatusCode::OK);
    assert_eq!(page.headers()["cache-control"], "no-store");
    assert!(text(page).await.contains("Set up two-step verification"));
    let setup = text(post(app, "/account/security/mfa/setup", &[&first], "").await).await;
    assert!(setup.contains("<div class=\"qr\"><svg"), "{setup}");
    assert!(!setup.contains("<?xml"));
    let key = secret(user).await;
    assert!(setup.contains(&key[..4]));
    assert!(!setup.contains("<script"));

    // A wrong code keeps the setup pending; the right one turns it on, signs
    // out every other session and renews this one.
    let other = session(&login(app).await, "/dashboard");
    let refused = post(app, "/account/security/mfa/confirm", &[&first], &format!("code={}", wrong(&key))).await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text(refused).await.contains("<svg"));
    let confirmed = post(app, "/account/security/mfa/confirm", &[&first], &format!("code={}", code(&key))).await;
    let signed_in = session(&confirmed, "/account/security?done=enabled");
    redirected(&get(app, "/dashboard", &[&other]).await, "/login");
    redirected(&get(app, "/dashboard", &[&first]).await, "/login");
    assert_eq!(get(app, "/dashboard", &[&signed_in]).await.status(), StatusCode::OK);
    let enabled = text(get(app, "/account/security?done=enabled", &[&signed_in]).await).await;
    assert!(enabled.contains("Two-step verification is on"), "{enabled}");

    // The password alone no longer signs in: only a challenge cookie.
    let challenge = challenged(&login(app).await);
    let prompt = get(app, "/login/mfa", &[&challenge]).await;
    assert_eq!(prompt.status(), StatusCode::OK);
    assert!(text(prompt).await.contains("Enter your code"));
    assert_eq!(get(app, "/login/mfa", &[]).await.status(), StatusCode::BAD_REQUEST);

    // Four wrong codes are refused one by one; the fifth ends the challenge,
    // and signing in again is refused while it lives.
    for _ in 0..4 {
        let response = post(app, "/login/mfa", &[&challenge], &format!("code={}", wrong(&key))).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(set_cookie(&response, "rullst_session").is_none());
    }
    let ended = post(app, "/login/mfa", &[&challenge], &format!("code={}", wrong(&key))).await;
    assert_eq!(ended.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(set_cookie(&ended, "rullst_mfa_challenge").unwrap().starts_with("rullst_mfa_challenge=; Path=/login/mfa; Max-Age=0;"));
    let used_up = post(app, "/login/mfa", &[&challenge], &format!("code={}", code(&key))).await;
    assert_eq!(used_up.status(), StatusCode::TOO_MANY_REQUESTS);
    let locked = login(app).await;
    assert_eq!(locked.status(), StatusCode::OK);
    assert!(text(locked).await.contains("Too many wrong codes"));

    // Once that challenge is gone (it expires after five minutes), the right
    // code signs in, once, where the sign-in was going.
    rullst::db::sqlx::query("DELETE FROM mfa_challenges")
        .execute(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap();
    rewind(user).await;
    let challenge = challenged(&login(app).await);
    let passed = post(app, "/login/mfa", &[&challenge], &format!("code={}", code(&key))).await;
    let second = session(&passed, "/dashboard");
    assert!(set_cookie(&passed, "rullst_mfa_challenge").unwrap().starts_with("rullst_mfa_challenge=;"));
    assert_eq!(get(app, "/dashboard", &[&second]).await.status(), StatusCode::OK);
    rewind(user).await;
    let replayed = post(app, "/login/mfa", &[&challenge], &format!("code={}", code(&key))).await;
    assert_eq!(replayed.status(), StatusCode::BAD_REQUEST);
    assert!(set_cookie(&replayed, "rullst_session").is_none());

    // Turning it off needs a current code.
    let kept = post(app, "/account/security/mfa/disable", &[&second], &format!("code={}", wrong(&key))).await;
    assert_eq!(kept.status(), StatusCode::UNPROCESSABLE_ENTITY);
    rewind(user).await;
    let off = post(app, "/account/security/mfa/disable", &[&second], &format!("code={}", code(&key))).await;
    redirected(&off, "/account/security?done=disabled");
    let direct = session(&login(app).await, "/dashboard");

    // A lost phone: the reset command removes the factor and signs out
    // every session.
    rullst::db::sqlx::query(
        "INSERT INTO user_mfa_factors (user_id, secret, confirmed, last_accepted_step) VALUES (?, ?, 1, NULL)",
    )
    .bind(i64::from(user))
    .bind(key.as_str())
    .execute(rullst::orm::Orm::pool().unwrap())
    .await
    .unwrap();
    challenged(&login(app).await);
    assert_eq!(
        super::admin::reset(EMAIL).await.unwrap(),
        format!("Two-step verification was removed for {EMAIL} and every session was signed out.")
    );
    redirected(&get(app, "/dashboard", &[&direct]).await, "/login");
    session(&login(app).await, "/dashboard");
    assert_eq!(
        super::admin::reset("nobody@example.test").await.unwrap_err(),
        "No account uses nobody@example.test."
    );

    // A provider sign-in (make:oauth) also stops at the second step.
    if std::path::Path::new("src/controllers/oauth/mod.rs").exists() {
        let started = get(app, "/auth/google", &[]).await;
        let provider = pair(&set_cookie(&started, "rullst_oauth_challenge").unwrap());
        let callback = location(&started).trim_start_matches("http://localhost:3000").to_owned();
        session(&get(app, &callback, &[&provider]).await, "/dashboard");
        let linked = account("mock@example.invalid").await;
        rullst::db::sqlx::query(
            "INSERT INTO user_mfa_factors (user_id, secret, confirmed, last_accepted_step) VALUES (?, ?, 1, NULL)",
        )
        .bind(i64::from(linked))
        .bind(key.as_str())
        .execute(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap();
        let started = get(app, "/auth/google?next=/account/security", &[]).await;
        let provider = pair(&set_cookie(&started, "rullst_oauth_challenge").unwrap());
        let callback = location(&started).trim_start_matches("http://localhost:3000").to_owned();
        let stopped = get(app, &callback, &[&provider]).await;
        let challenge = challenged(&stopped);
        assert!(set_cookie(&stopped, "rullst_oauth_challenge").unwrap().contains("Max-Age=0"));
        let passed = post(app, "/login/mfa", &[&challenge], &format!("code={}", code(&key))).await;
        session(&passed, "/account/security");
    }
}
