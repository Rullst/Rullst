use axum::{
    body::Body,
    extract::{ConnectInfo, Request as AxumRequest},
    http::{Request, StatusCode},
    middleware::Next,
};
use rullst_studio::*;
use std::net::SocketAddr;
use tower::ServiceExt;

async fn inject_loopback(mut request: AxumRequest, next: Next) -> axum::response::Response {
    request.headers_mut().insert(
        axum::http::header::HOST,
        axum::http::HeaderValue::from_static("127.0.0.1:5555"),
    );
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) {
        request.headers_mut().insert(
            axum::http::header::ORIGIN,
            axum::http::HeaderValue::from_static("http://127.0.0.1:5555"),
        );
    }
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42000"
            .parse::<SocketAddr>()
            .expect("loopback test peer"),
    ));
    next.run(request).await
}

fn local_studio(studio: Studio) -> axum::Router {
    studio
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router")
        .layer(axum::middleware::from_fn(inject_loopback))
}

#[tokio::test]
async fn test_studio_core_routes() {
    let app = local_studio(Studio::new());

    let routes = [
        "/",
        "/studio",
        "/migrations",
        "/studio/migrations",
        "/ai",
        "/studio/ai",
        "/security",
        "/studio/security",
        "/radar",
        "/studio/radar",
        "/capital",
        "/studio/capital",
        "/traces",
        "/studio/traces",
        "/studio/env",
        "/studio/features",
        "/studio/er",
        "/studio/requests",
        "/studio/cache",
    ];

    for path in routes {
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "Failed on route: {}", path);
    }

    for legacy_path in [
        "/tools/migrations",
        "/tools/ai",
        "/tools/security",
        "/tools/telemetry",
        "/tools/revenue",
        "/tools/traces",
        "/studio/tools/radar",
        "/env",
        "/features",
        "/er",
        "/requests",
    ] {
        let req = Request::builder()
            .uri(legacy_path)
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "legacy route remained mounted: {legacy_path}"
        );
    }
}

#[tokio::test]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
async fn test_studio_table_browser_and_schema_inspection() {
    let db_url = "sqlite:file:studio_shared_db?mode=memory&cache=shared";
    let _ = rullst_orm::Orm::init(db_url).await;

    if let Some(pool) = rullst_core::db::safe_pool() {
        let _ = rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS studio_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                age INTEGER,
                is_active BOOLEAN DEFAULT 1,
                bio TEXT
            )",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                price REAL NOT NULL,
                in_stock INTEGER DEFAULT 10
            )",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL REFERENCES studio_users(id),
                product_id INTEGER NOT NULL REFERENCES products(id),
                total REAL NOT NULL
            )",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "INSERT INTO studio_users (username, email, age, is_active, bio) VALUES
             ('alice', 'alice@rullst.dev', 30, 1, 'Engineer'),
             ('bob', 'bob@rullst.dev', 25, 0, NULL),
             ('carol', 'carol@rullst.dev', 28, 1, 'Designer')",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "INSERT INTO products (title, price, in_stock) VALUES
             ('Laptop', 1200.50, 5),
             ('Mouse', 25.00, 50)",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "INSERT INTO orders (user_id, product_id, total) VALUES (1, 1, 1200.50)",
        )
        .execute(pool)
        .await;
    }

    let app = local_studio(Studio::new());

    // 1. Standard HTML table view
    let req = Request::builder()
        .uri("/tables/studio_users")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "{body_str}");
    assert!(body_str.contains("alice") || body_str.contains("studio_users"));
    assert!(body_str.contains("Actions"), "{body_str}");
    assert!(body_str.contains("/rows/update"), "{body_str}");
    assert!(body_str.contains("/rows/delete"), "{body_str}");

    // 2. Search query and pagination
    let req = Request::builder()
        .uri("/tables/studio_users?page=1&search=alice")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. HTMX partial request
    let req = Request::builder()
        .uri("/tables/studio_users")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Unknown table 404 handler
    let req = Request::builder()
        .uri("/tables/non_existent_table")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 5. ER diagram generation with schema populated
    let req = Request::builder()
        .uri("/studio/er")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("erDiagram"));
    assert!(body_str.contains("studio_users"), "{body_str}");
    assert!(body_str.contains("orders"), "{body_str}");
    assert!(!body_str.contains("Schema unavailable"), "{body_str}");

    // 6. Feature Flags
    let req = Request::builder()
        .uri("/studio/features")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let pool = rullst_core::db::safe_pool().expect("Studio SQLite pool");
    rullst_orm::_sqlx::query(
        "INSERT OR REPLACE INTO rullst_feature_flags \
         (name, enabled, rollout_percentage, variants) VALUES (?, ?, ?, ?)",
    )
    .bind("dark_mode")
    .bind(0_i32)
    .bind(100_i32)
    .bind("[]")
    .execute(pool)
    .await
    .expect("insert deterministic feature flag fixture");

    use rullst_core::FeatureDriver;
    let feature_driver = rullst_core::DbFeatureDriver::with_ttl(std::time::Duration::from_secs(60));
    assert_eq!(
        feature_driver.enabled_for("dark_mode", "user-42").await,
        Some(false)
    );

    let req = Request::builder()
        .method("POST")
        .uri("/studio/features/toggle/dark_mode")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(axum::http::header::LOCATION).unwrap(),
        "/studio/features"
    );
    assert_eq!(
        feature_driver.enabled_for("dark_mode", "user-42").await,
        Some(true),
        "Studio toggle must invalidate already-warm in-process drivers"
    );

    // 7. Migration Manager Handlers
    use rullst_studio::migration_manager::*;
    let html = render_migration_manager_html("<div>Tables</div>");
    assert!(html.contains("Database schema tools"));

    let res = handle_run_migrations().await;
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);

    let res = handle_rollback_migrations().await;
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);

    let res = handle_run_seeders().await;
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_studio_ai_playground_and_providers() {
    use rullst_studio::ai_playground::*;

    let html = render_ai_playground_html();
    assert!(html.contains("AI integration"));
    assert!(html.contains("No AI client is connected"));
    assert!(!html.contains("Connection successful"));
}

#[tokio::test]
async fn test_studio_security_radar_and_telemetry() {
    let app = local_studio(Studio::new());

    let req = Request::builder()
        .uri("/security/stats")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/security/stats/stats")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_studio_horizon_jobs_and_purge() {
    let queue = rullst_core::Queue::sqlite(":memory:").await.unwrap();
    let app = local_studio(Studio::new().with_horizon(queue));

    let req = Request::builder()
        .uri("/studio/jobs")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/studio/jobs/jobs-table")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/studio/jobs/retry/job_123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);

    let req = Request::builder()
        .method("POST")
        .uri("/studio/jobs/purge")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(axum::http::header::LOCATION).unwrap(),
        "/studio/jobs"
    );
}

#[tokio::test]
async fn test_studio_openapi_uses_canonical_routes() {
    let app = local_studio(Studio::new().with_openapi(utoipa::openapi::OpenApi::default()));

    let redirect = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/api")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(redirect.status().is_redirection());

    for path in ["/studio/api/", "/studio/api/openapi.json"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "failed route: {path}");
    }

    let legacy = app
        .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(legacy.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_studio_db_helpers_and_serializers() {
    use rullst_studio::data_browser::db::*;

    assert_eq!(escape_html_attr("<hello>"), "&lt;hello&gt;");
    assert_eq!(quote_table_name("postgres", "users"), "\"users\"");
    assert_eq!(quote_table_name("mysql", "users"), "`users`");
    assert_eq!(quote_table_name("sqlite", "users"), "\"users\"");

    assert!(build_fetch_tables_query("postgres").contains("information_schema"));
    assert!(build_fetch_tables_query("mysql").contains("information_schema"));
    assert!(build_fetch_tables_query("sqlite").contains("sqlite_master"));

    assert!(build_schema_query("postgres", "users").contains("information_schema.columns"));
    assert!(build_schema_query("mysql", "users").contains("information_schema.columns"));
    assert!(build_schema_query("sqlite", "users").contains("PRAGMA table_info"));

    assert_eq!(resolve_db_url("custom_url"), "custom_url");
}

#[tokio::test]
async fn test_studio_env_viewer_endpoint() {
    let app = local_studio(Studio::new());

    let req = Request::builder()
        .uri("/studio/env")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_logger_requests_flow() {
    let app = local_studio(Studio::new());

    let req = Request::builder()
        .uri("/studio")
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .uri("/studio/requests")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_api_json_endpoints() {
    let app = local_studio(Studio::new());

    let endpoints = ["/api/radar", "/api/revenue", "/api/traces"];

    for ep in endpoints {
        let req = Request::builder().uri(ep).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
