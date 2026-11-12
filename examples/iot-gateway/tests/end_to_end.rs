//! Two devices post through the production security baseline, the dashboard
//! shows both, and a revoked token is refused while the other device keeps
//! streaming.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rullst::config::{Environment, SecurityConfig};
use rullst::db::schema::Migration;
use rullst::iot::SensorTelemetry;
use rullst::security::apply_security_baseline_with_machine_endpoints;
use rullst::web::axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use rullst_iot_gateway_example::registry::{CreateIotDevicesTable, SqlDeviceDirectory};
use rullst_iot_gateway_example::server::{TELEMETRY_PATH, app};
use tower::ServiceExt;

fn telemetry(device: &str, seq: f64) -> Request<Body> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let body = serde_json::to_string(&vec![
        SensorTelemetry::new(device, "temperature_c_uncalibrated", 7.1, now),
        SensorTelemetry::new(device, "sequence", seq, now),
    ])
    .unwrap();
    Request::post(TELEMETRY_PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-rullst-firmware", "rullst-uno-sensor-2")
        .body(Body::from(body))
        .unwrap()
}

async fn send(app: &Router, mut request: Request<Body>, token: &str) -> StatusCode {
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    app.clone().oneshot(request).await.unwrap().status()
}

/// The database the compiled ORM pool speaks: "SQLite" or "Any" normally,
/// "PostgreSQL" or "MySQL" when Cargo feature unification (`cargo test
/// --workspace --all-features`) enables a strict driver. These tests need
/// SQLite and skip otherwise; `cargo test -p rullst-iot-gateway-example`
/// runs them.
fn sqlite_compiled() -> bool {
    fn name<DB: rullst_orm::sqlx::Database>(
        _: Option<&rullst_orm::sqlx::Pool<DB>>,
    ) -> &'static str {
        DB::NAME
    }
    let compiled = name(None::<&rullst_orm::RullstPool>);
    let usable = matches!(compiled, "SQLite" | "Any");
    if !usable {
        eprintln!("skipping: this example needs SQLite, but the ORM is compiled for {compiled}");
    }
    usable
}

#[tokio::test]
async fn two_devices_stream_and_a_revoked_token_is_refused() {
    if !sqlite_compiled() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("iot.db").display());
    rullst_orm::Orm::init(&url).await.unwrap();
    CreateIotDevicesTable.up().await.unwrap();
    let registry = SqlDeviceDirectory::new();
    let (_, token_a) = registry.register("uno-a", "Greenhouse").await.unwrap();
    let (_, token_b) = registry.register("uno-b", "Workshop").await.unwrap();

    let built = app(None).unwrap();
    let dashboard = built.dashboard.clone();
    let router = apply_security_baseline_with_machine_endpoints(
        built.router.into_axum(),
        SecurityConfig::default(),
        Environment::Production,
        built.policy,
    )
    .unwrap();

    for seq in 0..3 {
        assert_eq!(
            send(&router, telemetry("uno-a", seq.into()), token_a.expose()).await,
            StatusCode::ACCEPTED
        );
        assert_eq!(
            send(&router, telemetry("uno-b", seq.into()), token_b.expose()).await,
            StatusCode::ACCEPTED
        );
    }
    // A board's token cannot post another board's readings.
    assert_eq!(
        send(&router, telemetry("uno-b", 9.0), token_a.expose()).await,
        StatusCode::FORBIDDEN
    );

    // The receiver's channel feeds the dashboard; both devices appear.
    let cards = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let devices = registry.list().await.unwrap();
            let cards = dashboard.fragments(&devices).0.into_string();
            if cards.matches("3 batches since the server started").count() == 2 {
                return cards;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the dashboard shows both devices");
    assert_eq!(cards.matches("badge badge-active").count(), 2, "{cards}");
    assert!(cards.contains("Firmware rullst-uno-sensor-2."), "{cards}");

    // Revocation applies to the next request; the other board continues.
    assert!(registry.revoke("uno-b").await.unwrap());
    assert_eq!(
        send(&router, telemetry("uno-b", 3.0), token_b.expose()).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&router, telemetry("uno-a", 3.0), token_a.expose()).await,
        StatusCode::ACCEPTED
    );
    let devices = registry.list().await.unwrap();
    let cards = dashboard.fragments(&devices).0.into_string();
    assert!(cards.contains("badge badge-revoked"), "{cards}");

    // The page subscribes with the htmx SSE extension under the strict CSP.
    let page = router
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let csp = page.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(csp.contains("script-src 'self' 'nonce-"), "{csp}");
    let html =
        String::from_utf8(to_bytes(page.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
    for expected in [
        r#"hx-ext="sse""#,
        r#"sse-connect="/events""#,
        r#"sse-swap="devices""#,
        r#"src="/static/htmx-ext-sse-2.2.4.js""#,
        r#"data-device="uno-a""#,
        r#"data-device="uno-b""#,
    ] {
        assert!(html.contains(expected), "{expected}");
    }
    let events = router
        .oneshot(Request::get("/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(events.headers()[header::CONTENT_TYPE], "text/event-stream");
}
