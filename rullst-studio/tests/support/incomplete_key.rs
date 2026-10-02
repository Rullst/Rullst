//! Composite-key regression shared by the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

fn quote_identifier(driver: &str, identifier: &str) -> String {
    if driver == "mysql" {
        format!("`{identifier}`")
    } else {
        format!("\"{identifier}\"")
    }
}

/// TM-STUDIO-06: a composite key whose second column is outside Studio's
/// ASCII identifier boundary must not be reduced to its first column, which
/// would match (and change) every row sharing that prefix.
pub(super) async fn exercise_incomplete_primary_key(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) {
    let key_table = format!("{table}_key");
    let quoted_table = quote_identifier(driver, &key_table);
    let key_column = quote_identifier(driver, "c\u{f3}digo");
    let mut create =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create
        .push(&quoted_table)
        .push(" (branch BIGINT NOT NULL, ")
        .push(&key_column)
        .push(" VARCHAR(32) NOT NULL, qty BIGINT NOT NULL, PRIMARY KEY (branch, ")
        .push(&key_column)
        .push("))");
    fixture(&mut create, driver)
        .execute(pool)
        .await
        .expect("create Studio incomplete-key table");
    for (code, quantity) in [("A", 10_i64), ("B", 20_i64)] {
        let mut insert =
            rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
        insert
            .push(&quoted_table)
            .push(" (branch, ")
            .push(&key_column)
            .push(", qty) VALUES (")
            .push_bind(1_i64)
            .push(", ")
            .push_bind(code)
            .push(", ")
            .push_bind(quantity)
            .push(")");
        fixture(&mut insert, driver)
            .execute(pool)
            .await
            .expect("insert Studio incomplete-key row");
    }

    let unchanged_rows = || async {
        let mut count = rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT COUNT(*) FROM ",
        );
        count
            .push(&quoted_table)
            .push(" WHERE (qty = ")
            .push_bind(10_i64)
            .push(" OR qty = ")
            .push_bind(20_i64)
            .push(")");
        fixture(&mut count, driver)
            .fetch_one(pool)
            .await
            .and_then(|row| rullst_orm::_sqlx::Row::try_get::<i64, _>(&row, 0))
            .expect("read Studio incomplete-key rows")
    };

    // Attempt the writes first so that a regression reports the changed rows.
    for (suffix, body) in [
        ("delete", format!("confirm=DELETE+{key_table}&pk_branch=1")),
        ("update", "column=qty&value=99&pk_branch=1".to_string()),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/studio/tables/{key_table}/rows/{suffix}"))
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("valid incomplete-key mutation request"),
            )
            .await
            .expect("Studio incomplete-key mutation response");
        assert_eq!(unchanged_rows().await, 2, "{suffix} changed a key prefix");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    let view = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{key_table}"))
                .body(Body::empty())
                .expect("valid incomplete-key table request"),
        )
        .await
        .expect("Studio incomplete-key table response");
    assert_eq!(view.status(), StatusCode::OK);
    let view = response_text(view).await;
    assert!(view.contains("Read-only: row changes need a complete primary key"));
    assert!(!view.contains("/rows/delete"));
    assert!(!view.contains("Choose column"));
}
