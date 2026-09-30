#![cfg(all(
    debug_assertions,
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
//! Studio database selection in an isolated process. This binary contains a
//! single test because it changes the process working directory.

use axum::{
    body::{Body, to_bytes},
    extract::{ConnectInfo, Request as AxumRequest},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use tower::ServiceExt;

async fn inject_loopback(mut request: AxumRequest, next: Next) -> Response {
    request.headers_mut().insert(
        header::HOST,
        axum::http::HeaderValue::from_static("127.0.0.1:5555"),
    );
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42000"
            .parse::<SocketAddr>()
            .expect("loopback test peer"),
    ));
    next.run(request).await
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("valid Studio request"),
        )
        .await
        .expect("Studio response");
    let status = response.status();
    let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("bounded Studio body");
    (
        status,
        String::from_utf8(body.to_vec()).expect("UTF-8 Studio body"),
    )
}

#[tokio::test]
async fn studio_views_use_the_shared_resolver_and_never_invent_sqlite() {
    if std::env::var_os("DATABASE_URL").is_some() {
        eprintln!("skipping: the process environment already selects a database");
        return;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let project = std::env::temp_dir().join(format!(
        "rullst-studio-resolution-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&project).expect("temporary Studio project");
    std::env::set_current_dir(&project).expect("enter temporary Studio project");

    let app = Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router")
        .layer(axum::middleware::from_fn(inject_loopback));

    // Without configuration, database views report it and create nothing.
    let (status, dashboard) = get(&app, "/studio").await;
    assert!(rullst_core::db::safe_pool().is_none());
    assert!(!project.join("db.sqlite").exists());
    assert_eq!(status, StatusCode::OK);
    assert!(dashboard.contains("No database is configured for Rullst Studio"));
    let (status, _) = get(&app, "/studio/tables/users").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    for uri in ["/studio/features", "/studio/er"] {
        let (status, page) = get(&app, uri).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            page.contains("No database is configured for Rullst Studio"),
            "{uri}"
        );
    }
    assert!(rullst_core::db::safe_pool().is_none());
    assert!(!project.join("db.sqlite").exists());

    // `.env` selects the same database that Server and Artisan would use.
    std::fs::write(
        project.join(".env"),
        "DATABASE_URL=sqlite://studio-dotenv.db\n",
    )
    .expect("write project .env");
    // Feature flags, like every database view, initialize the pool themselves.
    let (status, flags) = get(&app, "/studio/features").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!flags.contains("No database is configured"));
    assert!(flags.contains("CREATE TABLE rullst_feature_flags"));
    assert_eq!(rullst_core::db::safe_driver(), Some("sqlite"));
    let (status, diagram) = get(&app, "/studio/er").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!diagram.contains("Schema unavailable"));
    let (status, dashboard) = get(&app, "/studio").await;
    assert_eq!(status, StatusCode::OK);
    assert!(dashboard.contains("Rullst Studio Control Center"));
    assert_eq!(rullst_core::db::safe_driver(), Some("sqlite"));
    assert!(project.join("studio-dotenv.db").exists());
    assert!(!project.join("db.sqlite").exists());

    let _ = std::env::set_current_dir(std::env::temp_dir());
    let _ = std::fs::remove_dir_all(&project);
}
