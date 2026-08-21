use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use rullst_studio::*;
use tower::ServiceExt;

#[tokio::test]
async fn test_studio_core_routes() {
    let app = Studio::new().into_router();

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
        "/tools/migrations",
        "/tools/ai",
        "/tools/security",
        "/tools/telemetry",
        "/tools/revenue",
        "/tools/traces",
    ];

    for path in routes {
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "Failed on route: {}", path);
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

    let app = Studio::new().into_router();

    // 1. Standard HTML table view
    let req = Request::builder()
        .uri("/tables/studio_users")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("alice") || body_str.contains("studio_users"));

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
    assert_eq!(res.status(), StatusCode::OK);

    // 5. ER diagram generation with schema populated
    let req = Request::builder().uri("/er").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("erDiagram") || body_str.contains("studio_users"));

    // 6. Feature Flags
    let req = Request::builder()
        .uri("/features")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/features/toggle/dark_mode")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success() || res.status().is_redirection());

    // 7. Migration Manager Handlers
    use rullst_studio::migration_manager::*;
    let html = render_migration_manager_html("<div>Tables</div>");
    assert!(html.contains("Database Tools"));

    let res = handle_run_migrations().await.into_response();
    assert_eq!(res.status(), StatusCode::OK);

    let res = handle_rollback_migrations().await.into_response();
    assert_eq!(res.status(), StatusCode::OK);

    let res = handle_run_seeders().await.into_response();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_ai_playground_and_providers() {
    use rullst_studio::ai_playground::*;

    // Render HTML
    let html = render_ai_playground_html();
    assert!(html.contains("AI & RAG Playground"));

    // Fallback when no keys
    let req = axum::Json(PromptRequest {
        prompt: "Hello AI".to_string(),
        system_context: Some("System ctx".to_string()),
    });
    let res = handle_ai_prompt(req).await.into_response();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_security_radar_and_telemetry() {
    let app = Studio::new().into_router();

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
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_horizon_jobs_and_purge() {
    let queue = rullst_core::Queue::sqlite(":memory:").await.unwrap();
    let app = Studio::new().with_horizon(queue).into_router();

    let req = Request::builder().uri("/jobs").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/jobs/jobs-table")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/jobs/retry/job_123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/jobs/purge")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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
    let app = Studio::new().into_router();

    let req = Request::builder().uri("/env").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_logger_requests_flow() {
    let app = Studio::new().into_router();

    let req = Request::builder()
        .uri("/studio")
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .uri("/requests")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_api_json_endpoints() {
    let app = Studio::new().into_router();

    let endpoints = ["/api/radar", "/api/revenue", "/api/traces"];

    for ep in endpoints {
        let req = Request::builder().uri(ep).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
