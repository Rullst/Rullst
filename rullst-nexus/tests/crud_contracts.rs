//! Browser- and key-shaped CRUD contracts against a real SQLite database.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

use axum::{body::Body, http::StatusCode};
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use rullst_orm::{_sqlx as sqlx, Orm, RullstPool};
use support::{authenticated_test_router, local_request};
use tower::ServiceExt;

/// A slug-keyed model: record keys are arbitrary text.
struct Page;

impl NexusModel for Page {
    fn nexus_table() -> &'static str {
        "nexus_pages"
    }

    fn nexus_label() -> &'static str {
        "Pages"
    }

    fn nexus_pk() -> &'static str {
        "slug"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("slug", "Slug", FieldKind::Text),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ]
    }
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 response")
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            local_request()
                .uri(uri)
                .body(Body::empty())
                .expect("valid GET request"),
        )
        .await
        .expect("GET response");
    (response.status(), body_text(response).await)
}

#[tokio::test]
async fn crud_contracts_hold_on_sqlite() {
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated single-connection database");
    let pool = Orm::try_pool().expect("initialized test pool");
    sqlx::query("CREATE TABLE nexus_pages (slug TEXT PRIMARY KEY, title TEXT NOT NULL)")
        .execute(pool)
        .await
        .expect("create page table");
    let app = authenticated_test_router(Nexus::new().register::<Page>());

    search_matches_wildcards_literally(&app, pool).await;
}

/// `%` and `_` typed into the search box are literal characters (NX2-11).
async fn search_matches_wildcards_literally(app: &axum::Router, pool: &RullstPool) {
    sqlx::query(
        "INSERT INTO nexus_pages (slug, title) VALUES \
         ('sale', '50% off'), ('stock', '500 units'), \
         ('under', 'a_b'), ('plain', 'axb'), ('person', 'Alice')",
    )
    .execute(pool)
    .await
    .expect("insert search fixtures");

    let (status, html) = get(app, "/table/nexus_pages/search?q=50%25").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("50% off"));
    assert!(!html.contains("500 units"));

    let (_, html) = get(app, "/table/nexus_pages/search?q=a_b").await;
    assert!(html.contains("a_b"));
    assert!(!html.contains("axb"));

    let (_, html) = get(app, "/table/nexus_pages/search?q=alice").await;
    assert!(html.contains("Alice"));

    sqlx::query("DELETE FROM nexus_pages")
        .execute(pool)
        .await
        .expect("reset search fixtures");
}
