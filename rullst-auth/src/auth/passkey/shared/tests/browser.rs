//! Real HTTP and virtual-authenticator fixture, not a generated application.
use super::*;
use crate::auth::passkey::PublicKeyCredential;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::io::Write;
use subtle::ConstantTimeEq;

#[derive(Clone)]
struct App {
    start: SharedPasskeyAuth<PostgresCeremonyStore>,
    finish: SharedPasskeyAuth<PostgresCeremonyStore>,
    database: sqlx::PgPool,
    cookie: String,
    origin: String,
    csrf: String,
}
fn authorize(app: &App, headers: &HeaderMap) -> Result<PasskeyBinding, StatusCode> {
    for (name, expected) in [
        ("cookie", format!("passkey_fixture={}", app.cookie)),
        ("origin", app.origin.clone()),
        ("x-csrf-token", app.csrf.clone()),
    ] {
        if headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| v.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() != 1)
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    PasskeyBinding::new(
        "browser-tenant",
        "browser-account",
        &app.cookie,
        vec![7; 32],
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)
}
fn rejected(_: Error) -> StatusCode {
    StatusCode::UNAUTHORIZED
}
async fn register_start(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let binding = authorize(&app, &headers)?;
    let (options, expected) = app
        .start
        .start_register(&binding, "browser-user", "Browser user")
        .await
        .map_err(rejected)?;
    Ok(Json(
        serde_json::json!({"options":options,"expected":expected}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    expected: String,
    credential: RegisterPublicKeyCredential,
}
async fn register_finish(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Registration>,
) -> Result<StatusCode, StatusCode> {
    let binding = authorize(&app, &headers)?;
    let key = app
        .finish
        .finish_register(&binding, &body.credential, &body.expected)
        .await
        .map_err(rejected)?;
    sqlx::query("INSERT INTO rullst_passkey_ceremony.fixture_devices(subject,credential,public_key,counter) VALUES ('browser-account',$1,$2,$3)")
        .bind(key.credential_id).bind(key.public_key).bind(i64::from(key.sign_count)).execute(&app.database).await.map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn credential(app: &App) -> Result<crate::auth::passkey::Passkey, StatusCode> {
    let (id,public,count):(Vec<u8>,Vec<u8>,i64)=sqlx::query_as("SELECT credential,public_key,counter FROM rullst_passkey_ceremony.fixture_devices WHERE subject='browser-account' AND NOT revoked")
        .fetch_one(&app.database).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(crate::auth::passkey::Passkey {
        credential_id: id,
        public_key: public,
        sign_count: u32::try_from(count).map_err(|_| StatusCode::UNAUTHORIZED)?,
    })
}
async fn authenticate_start(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let binding = authorize(&app, &headers)?;
    let key = credential(&app).await?;
    let (options, expected) = app
        .start
        .start_authenticate(&binding, &[key])
        .await
        .map_err(rejected)?;
    Ok(Json(
        serde_json::json!({"options":options,"expected":expected}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Assertion {
    expected: String,
    credential: PublicKeyCredential,
    user_handle: Option<String>,
}
async fn authenticate_finish(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Assertion>,
) -> Result<StatusCode, StatusCode> {
    let binding = authorize(&app, &headers)?;
    let key = credential(&app).await?;
    let previous = key.sign_count;
    let verified = app
        .finish
        .finish_authenticate(
            &binding,
            &body.credential,
            &body.expected,
            key,
            body.user_handle.as_deref(),
        )
        .await
        .map_err(rejected)?;
    let changed=sqlx::query("UPDATE rullst_passkey_ceremony.fixture_devices SET counter=$1 WHERE subject='browser-account' AND credential=$2 AND counter=$3 AND NOT revoked")
        .bind(i64::from(verified.sign_count)).bind(verified.credential_id).bind(i64::from(previous)).execute(&app.database).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    if changed.rows_affected() != 1 {
        return Err(StatusCode::CONFLICT);
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn page(State(app): State<App>) -> Response {
    let html = format!(
        r#"<!doctype html><html lang="en"><title>Shared passkey fixture</title>
<button id="register">Register passkey</button><button id="authenticate">Authenticate</button><button id="replay">Replay assertion</button><output id="status">ready</output>
<script nonce="{}">const csrf = {:?};
{}
</script></html>"#,
        app.csrf,
        app.csrf,
        include_str!("browser_client.js")
    );
    let mut response = Html(html).into_response();
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response.headers_mut().insert("content-security-policy",format!("default-src 'none'; script-src 'nonce-{}'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'",app.csrf).parse().unwrap());
    response
}

pub(super) async fn run(url: &str) {
    if std::env::var("RULLST_PASSKEY_BROWSER_TESTS").as_deref() != Ok("1") {
        return;
    }
    let (raw, a, b, _) = reset(url).await;
    a.close().await;
    b.close().await;
    sqlx::query("DROP SCHEMA rullst_passkey_ceremony CASCADE")
        .execute(&raw)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://localhost:{}", listener.local_addr().unwrap().port());
    let config = CeremonyStoreConfig::new("browser-epoch", 8, 60).unwrap();
    let a = PostgresCeremonyStore::initialize(url, config.clone())
        .await
        .unwrap();
    let b = PostgresCeremonyStore::connect(url, config).await.unwrap();
    sqlx::query("CREATE TABLE rullst_passkey_ceremony.fixture_devices(subject TEXT PRIMARY KEY,credential BYTEA NOT NULL UNIQUE,public_key BYTEA NOT NULL,counter BIGINT NOT NULL,revoked BOOLEAN NOT NULL DEFAULT FALSE)").execute(&raw).await.unwrap();
    let config = PasskeyConfig::new("Browser fixture", "localhost", &origin)
        .with_max_pending_challenges(8)
        .with_challenge_ttl_seconds(60);
    let cookie = crate::auth::passkey::service::generate_challenge();
    let csrf = crate::auth::passkey::service::generate_challenge();
    let app = App {
        start: SharedPasskeyAuth::new(&config, a.clone()).unwrap(),
        finish: SharedPasskeyAuth::new(&config, b.clone()).unwrap(),
        database: raw.clone(),
        cookie: cookie.clone(),
        origin: origin.clone(),
        csrf,
    };
    let router = Router::new()
        .route("/", get(page))
        .route("/a/register", post(register_start))
        .route("/b/register", post(register_finish))
        .route("/a/authenticate", post(authenticate_start))
        .route("/b/authenticate", post(authenticate_finish))
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024))
        .with_state(app);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".github/passkey-browser-smoke.mjs");
    let result = tokio::task::spawn_blocking(move || {
        let mut child = std::process::Command::new("node")
            .arg(script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                serde_json::json!({"origin":origin,"cookie":cookie})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap();
    let _ = stop.send(());
    server.await.unwrap();
    a.close().await;
    b.close().await;
    raw.close().await;
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
