//! Page-order check shared by the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

/// Pages follow the primary key: an edited row stays on its page. Without an
/// `ORDER BY`, PostgreSQL returns the new version of an updated row last, so
/// page 1 would lose it and show the first row of page 2 instead.
pub(super) async fn exercise_stable_paging(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) {
    let paged_table = format!("{table}_paged");
    let quoted_table = if driver == "mysql" {
        format!("`{paged_table}`")
    } else {
        format!("\"{paged_table}\"")
    };
    let values = (1..=30)
        .map(|id| format!("({id}, 'row-{id}')"))
        .collect::<Vec<_>>()
        .join(", ");
    for statement in [
        format!("CREATE TABLE {quoted_table} (id BIGINT PRIMARY KEY, label VARCHAR(32) NOT NULL)"),
        format!("INSERT INTO {quoted_table} (id, label) VALUES {values}"),
    ] {
        let mut builder = rullst_orm::_sqlx::QueryBuilder::new(statement);
        fixture(&mut builder, driver)
            .execute(pool)
            .await
            .expect("prepare the paged table");
    }

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/studio/tables/{paged_table}/rows/update"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("column=label&value=row-1-edited&pk_id=1"))
                .expect("valid paged update request"),
        )
        .await
        .expect("Studio paged update response");
    assert!(update.status().is_redirection());

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{paged_table}?page=1"))
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid paged table request"),
        )
        .await
        .expect("Studio paged table response");
    assert_eq!(page.status(), StatusCode::OK);
    let page = response_text(page).await;
    assert!(page.contains("row-1-edited"), "the edited row left page 1");
    assert!(page.contains("name=\"pk_id\" value=\"25\""));
    assert!(!page.contains("name=\"pk_id\" value=\"26\""));
}
