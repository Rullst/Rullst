//! Real generated SaaS router, middleware, handlers and users table on
//! SQLite: every sign-in is a registered session. Signing out ends every
//! copy of its cookie, "Your devices" lists and signs out devices, the
//! session epoch still ends every session at once, and deleting the account
//! with SQL deletes its sessions.
use super::*;
use rullst::server::Body;
use std::sync::atomic::{AtomicU8, Ordering};
use tower::ServiceExt;

const CSRF: &str = "SessionRegistryContractCsrfToken";
const MEMBER: &str = "member@example.test";
const PASSWORD: &str = "registry-contract-password-1";
const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";
const SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";
/// A distinct peer per request, so the per-client credential budget never
/// throttles the contract.
static PEER: AtomicU8 = AtomicU8::new(1);

type App = rullst::web::axum::Router;

struct Reply {
    status: StatusCode,
    location: String,
    session: Option<String>,
    body: String,
}

async fn call(app: &App, method: &str, uri: &str, session: Option<&str>, form: &str, agent: &str) -> Reply {
    let mut cookie = format!("rullst_csrf={CSRF}");
    if let Some(session) = session {
        cookie.push_str(&format!("; rullst_session={session}"));
    }
    let peer = std::net::SocketAddr::from(([127, 0, 1, PEER.fetch_add(1, Ordering::Relaxed)], 40_000));
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost:3000")
        .header("user-agent", agent)
        .header("cookie", cookie)
        .header("x-csrf-token", CSRF)
        .header("content-type", "application/x-www-form-urlencoded")
        .extension(rullst::web::axum::extract::ConnectInfo(peer))
        .body(Body::from(form.to_owned()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let location = response.headers().get("location").map_or("", |value| value.to_str().unwrap()).to_owned();
    let session = response.headers().get_all("set-cookie").iter().find_map(|value| {
        let value = value.to_str().unwrap().strip_prefix("rullst_session=")?;
        Some(value.split(';').next().unwrap().to_owned())
    });
    let body = rullst::web::axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap();
    Reply { status, location, session, body: String::from_utf8(body.to_vec()).unwrap() }
}

async fn sign_in(app: &App, agent: &str) -> String {
    let reply = call(app, "POST", "/login", None, &format!("_token={CSRF}&email={MEMBER}&password={PASSWORD}"), agent).await;
    assert_eq!((reply.status, reply.location.as_str()), (StatusCode::SEE_OTHER, "/dashboard"));
    reply.session.unwrap()
}

async fn dashboard(app: &App, session: &str) -> StatusCode {
    call(app, "GET", "/dashboard", Some(session), "", FIREFOX).await.status
}

async fn stored() -> Vec<(String, String, i32, String, String)> {
    rullst::db::sqlx::query_as("SELECT token_hash, public_id, user_id, browser, platform FROM rullst_auth_sessions ORDER BY created_at, token_hash")
        .fetch_all(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap()
}

/// The session references of the "Your devices" page, in its order.
fn references(page: &str) -> Vec<String> {
    page.split("name=\"session\" value=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn generated_session_registry_contract() {
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() { migration.up().await.unwrap(); }
    let hash = rullst::auth::hash_password(PASSWORD).unwrap();
    rullst::db::sqlx::query("INSERT INTO users (id,name,email,password_hash,created_at,updated_at) VALUES (?,?,?,?,?,?)")
        .bind(1).bind("Member").bind(MEMBER).bind(hash).bind("2026-11-15T00:00:00+00:00").bind("2026-11-15T00:00:00+00:00")
        .execute(rullst::orm::Orm::pool().unwrap()).await.unwrap();
    // `Server` adds CSRF and the secure headers to the router; so does this.
    let router = rullst::security::apply_security_baseline(
        crate::router().unwrap().into_axum(),
        rullst::config::RullstConfig::global().security.clone(),
        rullst::config::Environment::Test,
    )
    .unwrap();
    let app = &router;

    // Two sign-ins are two registered sessions; only digests are stored.
    let laptop = sign_in(app, FIREFOX).await;
    let phone = sign_in(app, SAFARI).await;
    assert!(laptop.starts_with("v3.") && phone.starts_with("v3."));
    assert_eq!(dashboard(app, &laptop).await, StatusCode::OK);
    assert_eq!(dashboard(app, &phone).await, StatusCode::OK);
    let rows = stored().await;
    assert_eq!(rows.len(), 2);
    for (token_hash, public_id, user_id, _, _) in &rows {
        assert_eq!((token_hash.len(), public_id.len(), *user_id), (64, 22, 1));
        assert!(!laptop.contains(token_hash.as_str()) && !phone.contains(token_hash.as_str()));
    }
    let mut devices: Vec<(&str, &str)> = rows.iter().map(|row| (row.3.as_str(), row.4.as_str())).collect();
    devices.sort_unstable();
    assert_eq!(devices, vec![("firefox", "linux"), ("safari", "ios")]);
    // A stateless epoch cookie is not a session of this application.
    let key = rullst::auth::get_app_key().unwrap();
    let stateless = rullst::auth::encrypt_session_with_epoch(1, 0, rullst::auth::DEFAULT_SESSION_MAX_AGE, &key).unwrap();
    assert_eq!(dashboard(app, &stateless).await, StatusCode::SEE_OTHER);

    // Your devices: both, this one marked, served privately.
    let page = call(app, "GET", "/sessions", Some(&laptop), "", FIREFOX).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("Firefox on Linux") && page.body.contains("Safari on iOS"));
    assert_eq!(page.body.matches("This device").count(), 1);
    assert_eq!(references(&page.body).len(), 1);
    assert_eq!(call(app, "GET", "/sessions", None, "", FIREFOX).await.status, StatusCode::SEE_OTHER);

    // A Server-Sent Events stream of a session ends once the session signs
    // out in this process, without waiting for the revalidation interval.
    let streaming = sign_in(app, FIREFOX).await;
    let now = rullst::auth::session_registry::unix_now().unwrap();
    let session = sessions().await.unwrap().authenticate(&streaming, &key, now).await.unwrap();
    let channels = rullst::realtime::BroadcastManager::new();
    let sse = rullst::realtime::RealtimeSse::new(rullst::realtime::RealtimeSseConfig::new());
    let stream = sse.channel_revalidated(&channels, "dashboard", revalidation(&session).await.unwrap());
    let ended = call(app, "POST", "/logout", Some(&streaming), &format!("_token={CSRF}"), FIREFOX).await;
    assert_eq!(ended.status, StatusCode::SEE_OTHER);
    let body = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rullst::web::axum::body::to_bytes(stream.into_body(), 1 << 16),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(&body[..], b"retry: 1000\n\n");
    assert_eq!(sse.open_connections(), 0);

    // Signing out ends every copy of the cookie, not only this browser's.
    let copied = phone.clone();
    let signed_out = call(app, "POST", "/logout", Some(&phone), &format!("_token={CSRF}"), SAFARI).await;
    assert_eq!((signed_out.status, signed_out.location.as_str()), (StatusCode::SEE_OTHER, "/login"));
    assert_eq!(signed_out.session.as_deref(), Some(""));
    assert_eq!(dashboard(app, &copied).await, StatusCode::SEE_OTHER);
    assert_eq!(dashboard(app, &laptop).await, StatusCode::OK);
    assert_eq!(stored().await.len(), 1);

    // Signing out one device from another.
    let tablet = sign_in(app, SAFARI).await;
    let page = call(app, "GET", "/sessions", Some(&laptop), "", FIREFOX).await;
    let listed = references(&page.body);
    assert_eq!(listed.len(), 1);
    let revoked = call(app, "POST", "/sessions/revoke", Some(&laptop), &format!("_token={CSRF}&session={}", listed[0]), FIREFOX).await;
    assert_eq!((revoked.status, revoked.location.as_str()), (StatusCode::SEE_OTHER, "/sessions?signed_out=one"));
    assert_eq!(dashboard(app, &tablet).await, StatusCode::SEE_OTHER);
    assert_eq!(dashboard(app, &laptop).await, StatusCode::OK);
    assert!(call(app, "GET", "/sessions?signed_out=one", Some(&laptop), "", FIREFOX).await.body.contains("That device was signed out."));
    // A malformed reference is refused; an unknown one revokes nothing.
    assert_eq!(call(app, "POST", "/sessions/revoke", Some(&laptop), &format!("_token={CSRF}&session=bad"), FIREFOX).await.status, StatusCode::BAD_REQUEST);
    call(app, "POST", "/sessions/revoke", Some(&laptop), &format!("_token={CSRF}&session=AAAAAAAAAAAAAAAAAAAAAA"), FIREFOX).await;
    assert_eq!(dashboard(app, &laptop).await, StatusCode::OK);

    // Sign out of every other device keeps this one.
    let first = sign_in(app, SAFARI).await;
    let second = sign_in(app, FIREFOX).await;
    let others = call(app, "POST", "/sessions/revoke-others", Some(&laptop), &format!("_token={CSRF}"), FIREFOX).await;
    assert_eq!(others.location, "/sessions?signed_out=others");
    for revoked in [&first, &second] {
        assert_eq!(dashboard(app, revoked).await, StatusCode::SEE_OTHER);
    }
    assert_eq!(dashboard(app, &laptop).await, StatusCode::OK);
    assert_eq!(stored().await.len(), 1);

    // The session epoch still signs out every device, and its rows go too.
    let phone = sign_in(app, SAFARI).await;
    let everywhere = call(app, "POST", "/logout/everywhere", Some(&phone), &format!("_token={CSRF}"), SAFARI).await;
    assert_eq!(everywhere.location, "/login");
    for revoked in [&laptop, &phone] {
        assert_eq!(dashboard(app, revoked).await, StatusCode::SEE_OTHER);
    }
    assert!(stored().await.is_empty());
    assert_eq!(User::find(1).await.unwrap().unwrap().session_epoch, 1);
    // A full-row save of a copy loaded before the bump keeps the new epoch.
    let mut stale = User::find(1).await.unwrap().unwrap();
    let fresh = sign_in(app, FIREFOX).await;
    User::revoke_sessions(1).await.unwrap();
    stale.name = "Renamed".into();
    stale.save().await.unwrap();
    assert_eq!(User::find(1).await.unwrap().unwrap().session_epoch, 2);
    assert_eq!(dashboard(app, &fresh).await, StatusCode::SEE_OTHER);
    assert!(User::query().where_eq("is_admin", true).get().await.unwrap().is_empty());
    // A cookie revoked meanwhile is turned away before it can act again.
    assert_eq!(call(app, "POST", "/logout/everywhere", Some(&fresh), &format!("_token={CSRF}"), FIREFOX).await.status, StatusCode::SEE_OTHER);
    assert_eq!(User::find(1).await.unwrap().unwrap().session_epoch, 2);

    // An account deleted with SQL (a retention job) takes its sessions with
    // it: `user_id` references `users` with `ON DELETE CASCADE`.
    let last = sign_in(app, SAFARI).await;
    assert_eq!(stored().await.len(), 1);
    let orphan = rullst::db::sqlx::query("INSERT INTO rullst_auth_sessions (token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform) VALUES (?, ?, 2, 0, 0, 0, 'firefox', 'linux')")
        .bind("0".repeat(64)).bind("A".repeat(22))
        .execute(rullst::orm::Orm::pool().unwrap()).await;
    assert!(orphan.is_err(), "a session of an account that does not exist is refused");
    rullst::db::sqlx::query("DELETE FROM users WHERE id = ?").bind(1)
        .execute(rullst::orm::Orm::pool().unwrap()).await.unwrap();
    assert!(stored().await.is_empty());
    assert_eq!(dashboard(app, &last).await, StatusCode::SEE_OTHER);
}
