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

    // A search keeps only matching rows, in the page and in its count:
    // `row-2` and `row-20` to `row-29`.
    let searched = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{paged_table}?search=row-2"))
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid paged search request"),
        )
        .await
        .expect("Studio paged search response");
    assert_eq!(searched.status(), StatusCode::OK);
    let searched = response_text(searched).await;
    assert!(
        searched.contains("of <strong>11</strong> records"),
        "{searched}"
    );
    assert!(!searched.contains("name=\"pk_id\" value=\"3\""));

    use rullst_studio::data_browser::db::count_table_rows;
    assert_eq!(count_table_rows(&paged_table, None).await.ok(), Some(30));
    assert_eq!(
        count_table_rows(&paged_table, Some("row-2")).await.ok(),
        Some(11)
    );
    assert_eq!(
        count_table_rows(&paged_table, Some(&"a".repeat(256)))
            .await
            .ok(),
        Some(0)
    );
    assert!(
        count_table_rows(&paged_table, Some(&"a".repeat(257)))
            .await
            .is_err()
    );
    assert!(
        count_table_rows(&format!("{paged_table};"), None)
            .await
            .is_err()
    );

    // Without a search every row counts, even one whose only column is NULL.
    let nullable_table = format!("{table}_nullable");
    let quoted_nullable = if driver == "mysql" {
        format!("`{nullable_table}`")
    } else {
        format!("\"{nullable_table}\"")
    };
    for statement in [
        format!("CREATE TABLE {quoted_nullable} (note VARCHAR(32) NULL)"),
        format!("INSERT INTO {quoted_nullable} (note) VALUES (NULL)"),
    ] {
        let mut builder = rullst_orm::_sqlx::QueryBuilder::new(statement);
        fixture(&mut builder, driver)
            .execute(pool)
            .await
            .expect("prepare the nullable table");
    }
    let nullable = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{nullable_table}"))
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid nullable table request"),
        )
        .await
        .expect("Studio nullable table response");
    assert_eq!(nullable.status(), StatusCode::OK);
    let nullable = response_text(nullable).await;
    assert!(
        nullable.contains("of <strong>1</strong> records"),
        "{nullable}"
    );
    assert!(nullable.contains("text-slate-600 font-mono italic\">NULL</td>"));
}
