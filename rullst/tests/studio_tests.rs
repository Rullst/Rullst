#![cfg(not(miri))]
#![cfg(feature = "studio")]

use axum::{Router, routing::get};
use rullst::studio::{handle_dashboard, handle_table};
use rullst::testing::TestApp;

static INIT_DB: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn init_test_db() {
    INIT_DB.get_or_init(|| async {
        let db_path = "sqlite:file:studio_integration_test_db?mode=memory&cache=shared";
        let _ = rullst_orm::Orm::init(db_path).await;
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

fn build_studio_router() -> Router {
    Router::new()
        .route("/", get(handle_dashboard))
        .route("/tables/{table_name}", get(handle_table))
}

#[tokio::test]
async fn test_studio_dashboard() {
    init_test_db().await;
    let app = TestApp::new(build_studio_router());

    let response = app.get("/").await;
    response.assert_status(200);
    response.assert_see("Rullst Studio");
    response.assert_see("Welcome to Rullst Studio");
    response.assert_see("Database Type");
    response.assert_see("Total Tables");
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
    response.assert_status(200); // Handled error returns 200 with error message
    response.assert_see("Table 'nonexistent_table' not found.");
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
