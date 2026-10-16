//! Real generated SaaS users table, model, middleware and handlers: bumping the
//! session epoch ends every earlier login cookie, including pre-epoch ones.
use super::*;
use rullst::server::Body;
use tower::ServiceExt;

async fn send(app: &rullst::web::axum::Router, method: &str, uri: &str, session: &str) -> Response {
    let request = Request::builder().method(method).uri(uri)
        .header("cookie", format!("rullst_session={session}"))
        .body(Body::empty()).unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn stored_epoch() -> i64 {
    User::find(1).await.unwrap().unwrap().session_epoch
}

#[tokio::test]
async fn generated_session_epoch_contract() {
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() { migration.up().await.unwrap(); }
    rullst::db::sqlx::query("INSERT INTO users (id,name,email,created_at,updated_at) VALUES (?,?,?,?,?)")
        .bind(1).bind("Member").bind("member@example.test").bind("2026-10-14T00:00:00+00:00").bind("2026-10-14T00:00:00+00:00")
        .execute(rullst::orm::Orm::pool().unwrap()).await.unwrap();
    assert_eq!(stored_epoch().await, 0);
    let key = rullst::auth::get_app_key().unwrap();
    let issue = |epoch| rullst::auth::encrypt_session_with_epoch(1, epoch, rullst::auth::DEFAULT_SESSION_MAX_AGE, &key).unwrap();
    let router = crate::router().unwrap();
    let app = router.as_axum();
    let laptop = issue(0);
    let legacy = rullst::auth::encrypt_session(1, &key).unwrap();
    for cookie in [&laptop, &legacy] {
        assert_eq!(send(app, "GET", "/dashboard", cookie).await.status(), StatusCode::OK);
    }

    // A full-row save of a copy loaded before the bump keeps the new epoch.
    let mut stale = User::find(1).await.unwrap().unwrap();
    User::revoke_sessions(1).await.unwrap();
    stale.name = "Renamed".into();
    stale.save().await.unwrap();
    assert_eq!(stored_epoch().await, 1);
    for cookie in [&laptop, &legacy, &issue(2)] {
        assert_eq!(send(app, "GET", "/dashboard", cookie).await.status(), StatusCode::SEE_OTHER);
    }
    let phone = issue(1);
    assert_eq!(send(app, "GET", "/dashboard", &phone).await.status(), StatusCode::OK);

    // The generated handler bumps the epoch and clears this browser's cookie.
    let everywhere = rullst::web::axum::Router::new()
        .route("/logout/everywhere", rullst::routing::post(crate::controllers::auth_controller::logout_everywhere))
        .layer(rullst::server::from_fn(auth_middleware));
    let signed_out = send(&everywhere, "POST", "/logout/everywhere", &phone).await;
    assert_eq!(signed_out.status(), StatusCode::SEE_OTHER);
    assert_eq!(signed_out.headers()["location"], "/login");
    assert!(signed_out.headers()["set-cookie"].to_str().unwrap().starts_with("rullst_session=; "));
    assert_eq!(stored_epoch().await, 2);
    assert_eq!(send(app, "GET", "/dashboard", &phone).await.status(), StatusCode::SEE_OTHER);
    // A revoked cookie is turned away before it can bump the epoch again.
    assert_eq!(send(&everywhere, "POST", "/logout/everywhere", &phone).await.status(), StatusCode::SEE_OTHER);
    assert_eq!(stored_epoch().await, 2);
}
