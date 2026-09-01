#![cfg(not(miri))]
// The suite exercises both the Studio facade and direct ORM-backed fixtures.
// Keep it out of isolated `studio` feature checks where the facade intentionally
// does not expose its optional `rullst-orm` dependency.
#![cfg(all(feature = "studio", feature = "orm"))]
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst::studio::{LocalStudioAccess, Studio};
use rullst::testing::TestApp;

static INIT_DB: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn init_test_db() {
    INIT_DB.get_or_init(|| async {
        let db_path = "sqlite:file:studio_test.db?mode=rwc";
        if let Err(e) = rullst_orm::Orm::init(db_path).await
            && !e.to_string().contains("already been initialized")
        {
            panic!("Orm::init failed: {:?}", e);
        }
        let pool = rullst::db::safe_pool().expect("pool should be initialized");

        // Clean up tables just in case
        let _ = sqlx::query("DROP TABLE IF EXISTS studio_users").execute(pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS studio_posts").execute(pool).await;

        // Create tables
        sqlx::query("CREATE TABLE studio_users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE studio_posts (id INTEGER PRIMARY KEY, title TEXT, content TEXT);")
            .execute(pool)
            .await
            .unwrap();

        // Insert data
        sqlx::query("INSERT INTO studio_users (name, email) VALUES ('Alice', 'alice@example.com'), ('Bob', 'bob@example.com'), ('Charlie', 'charlie@example.com');")
            .execute(pool)
            .await
            .unwrap();
    }).await;
}

fn build_studio_router() -> axum::Router {
    Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router")
        .layer(axum::middleware::from_fn(
            |mut request: axum::extract::Request, next: axum::middleware::Next| async move {
                request.headers_mut().insert(
                    axum::http::header::HOST,
                    axum::http::HeaderValue::from_static("127.0.0.1:5555"),
                );
                request.extensions_mut().insert(axum::extract::ConnectInfo(
                    "127.0.0.1:42000"
                        .parse::<std::net::SocketAddr>()
                        .expect("loopback test peer"),
                ));
                next.run(request).await
            },
        ))
}

#[tokio::test]
async fn test_studio_dashboard() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/").await;
    response.assert_status(200);
    response.assert_see("Rullst Studio");
}

#[tokio::test]
async fn test_studio_table_details_full_page() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/tables/studio_users").await;
    response.assert_status(200);
    response.assert_see("studio_users");
    response.assert_see("Alice");
    response.assert_see("bob@example.com");
    response.assert_see("Search records...");
    response.assert_see("PK"); // id is PK
}

#[tokio::test]
async fn test_studio_table_details_htmx() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    // With HTMX header, it should return a partial HTML (no layout/header)
    let response = app
        .get("/tables/studio_users")
        .header("hx-request", "true")
        .await;

    response.assert_status(200);
    response.assert_see("studio_users");
    response.assert_see("Alice");
    response.assert_dont_see("Rullst Studio | Database Inspector"); // layout should not be rendered
}

#[tokio::test]
async fn test_studio_table_not_found() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/tables/nonexistent_table").await;
    response.assert_status(404);
    response.assert_see("The requested table is unavailable or uses an unsupported identifier.");
}

#[tokio::test]
async fn test_studio_table_search() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/tables/studio_users?search=Alice").await;
    response.assert_status(200);
    response.assert_see("Alice");
    response.assert_dont_see("Bob");
}

#[tokio::test]
async fn test_studio_table_empty() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/tables/studio_posts").await;
    response.assert_status(200);
    response.assert_see("No records found inside this table.");
}

#[tokio::test]
async fn test_studio_all_tool_endpoints() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    // Test prefixed routes (/studio/*)
    let endpoints = vec![
        "/studio",
        "/studio/migrations",
        "/studio/ai",
        "/studio/radar",
        "/studio/capital",
        "/studio/security",
        "/studio/traces",
        "/migrations",
        "/ai",
        "/radar",
        "/capital",
        "/security",
        "/traces",
    ];

    for ep in endpoints {
        let resp = app.get(ep).await;
        resp.assert_status(200);

        // Test with HTMX header
        let htmx_resp = app.get(ep).header("hx-request", "true").await;
        htmx_resp.assert_status(200);
    }
}
