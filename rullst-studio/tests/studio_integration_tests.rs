use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_studio::Studio;
use rullst_studio::data_browser::IntoStudioPort;
use tower::ServiceExt;

#[tokio::test]
async fn test_studio_core_routes() {
    let app = Studio::new().into_router();

    let routes = [
        "/",
        "/studio",
        "/studio/radar",
        "/studio/capital",
        "/studio/security",
        "/studio/traces",
        "/studio/migrations",
        "/studio/ai",
        "/env",
        "/features",
        "/er",
        "/security/stats",
        "/requests",
        "/tools/migrations",
        "/tools/ai",
        "/tools/security",
        "/tools/radar",
        "/tools/capital",
        "/tools/revenue",
        "/tools/traces",
        "/api/radar",
        "/api/revenue",
        "/api/traces",
    ];

    for route in routes {
        let req = Request::builder()
            .uri(route)
            .body(Body::empty())
            .expect("valid request");

        let res = app.clone().oneshot(req).await.expect("handler executed");
        assert!(
            res.status().is_success() || res.status().is_redirection(),
            "Route {} returned status {:?}",
            route,
            res.status()
        );
    }
}

#[tokio::test]
async fn test_into_studio_port_conversion() {
    assert_eq!(5555u16.into_port(), 5555);
    assert_eq!("8080".into_port(), 8080);
    assert_eq!("".into_port(), 5555);
    assert_eq!("invalid".into_port(), 5555);
    assert_eq!(String::from("9000").into_port(), 9000);
    assert_eq!(Some(3000u16).into_port(), 3000);
    assert_eq!(None::<u16>.into_port(), 5555);
}

#[tokio::test]
async fn test_studio_features_endpoints() {
    let app = Studio::new().into_router();

    let req = Request::builder()
        .uri("/features")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("Feature Flags") || body_str.contains("studio"));
}

#[tokio::test]
async fn test_studio_env_endpoints() {
    let app = Studio::new().into_router();

    let req = Request::builder().uri("/env").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_er_diagram() {
    let app = Studio::new().into_router();

    let req = Request::builder().uri("/er").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_with_horizon_queue() {
    let queue = rullst_core::Queue::sqlite(":memory:").await.unwrap();
    let app = Studio::new().with_horizon(queue).into_router();

    let req = Request::builder().uri("/jobs").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success() || res.status().is_redirection());
}

#[tokio::test]
async fn test_studio_with_openapi_playground() {
    use utoipa::OpenApi;

    #[derive(OpenApi)]
    #[openapi(paths(), components(schemas()))]
    struct ApiDoc;

    let app = Studio::new().with_openapi(ApiDoc::openapi()).into_router();

    let req = Request::builder().uri("/api").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success() || res.status().is_redirection());
}

#[tokio::test]
async fn test_studio_api_json_endpoints() {
    let app = Studio::new().into_router();

    let endpoints = ["/api/radar", "/api/revenue", "/api/traces"];

    for ep in endpoints {
        let req = Request::builder().uri(ep).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        assert!(!body_str.is_empty());
    }
}

#[tokio::test]
async fn test_studio_logger_requests_flow() {
    let app = Studio::new().into_router();

    // 1. Perform an arbitrary request to trigger logger middleware
    let req = Request::builder()
        .uri("/studio")
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // 2. Query /requests
    let req = Request::builder()
        .uri("/requests")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_studio_table_browser_and_schema_inspection() {
    let _ = rullst_orm::Orm::init("sqlite::memory:").await;

    if let Some(pool) = rullst_core::db::safe_pool() {
        let _ = rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS studio_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                email TEXT NOT NULL
            )"
        ).execute(pool).await;

        let _ = rullst_orm::_sqlx::query(
            "INSERT INTO studio_users (username, email) VALUES ('alice', 'alice@rullst.dev'), ('bob', 'bob@rullst.dev')"
        ).execute(pool).await;
    }

    let app = Studio::new().into_router();

    // 1. Standard HTML table view
    let req = Request::builder()
        .uri("/tables/studio_users")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

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

    // 4. ER diagram generation with schema populated
    let req = Request::builder()
        .uri("/er")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("erDiagram") || body_str.contains("studio_users"));

    // 5. Feature Flags Toggling
    let req = Request::builder()
        .method("POST")
        .uri("/features/toggle/dark_mode")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success() || res.status().is_redirection());
}
