//! Table-view byte bounds shared by the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

/// Mirrors Studio's display and search bounds.
const MAX_DISPLAY_CHARS: usize = 256;
const MAX_SEARCH_BYTES: usize = 256;

/// Large cells are cut by the database before they reach Studio, and search
/// terms are bounded before they are bound once per column.
pub(super) async fn exercise_bounded_table_view(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) {
    let wide_table = format!("{table}_wide");
    let quoted_table = if driver == "mysql" {
        format!("`{wide_table}`")
    } else {
        format!("\"{wide_table}\"")
    };
    let mut create =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create
        .push(&quoted_table)
        .push(" (id BIGINT PRIMARY KEY, body TEXT NOT NULL)");
    fixture(&mut create, driver)
        .execute(pool)
        .await
        .expect("create Studio wide-cell table");
    let long_body = format!("{}tail-marker", "a".repeat(8 * 1024));
    let mut insert =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
    insert
        .push(&quoted_table)
        .push(" (id, body) VALUES (")
        .push_bind(1_i64)
        .push(", ")
        .push_bind(long_body)
        .push(")");
    fixture(&mut insert, driver)
        .execute(pool)
        .await
        .expect("insert Studio wide-cell row");

    let get = |uri: String| async move {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("hx-request", "true")
                    .body(Body::empty())
                    .expect("valid bounded table request"),
            )
            .await
            .expect("Studio bounded table response");
        let status = response.status();
        (status, response_text(response).await)
    };

    let (status, view) = get(format!("/studio/tables/{wide_table}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!view.contains("tail-marker"));
    assert!(view.contains(&format!("{}…", "a".repeat(MAX_DISPLAY_CHARS))));
    assert!(!view.contains(&"a".repeat(MAX_DISPLAY_CHARS + 1)));

    let bounded_search = "a".repeat(MAX_SEARCH_BYTES);
    let (status, view) = get(format!(
        "/studio/tables/{wide_table}?search={bounded_search}"
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(view.contains("of <strong>1</strong> records"));

    let (status, _) = get(format!(
        "/studio/tables/{wide_table}?search={bounded_search}a"
    ))
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
