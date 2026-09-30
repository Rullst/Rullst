//! MySQL/MariaDB unsigned-integer checks for the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

/// MariaDB reports `bigint(20) unsigned` where MySQL 8 reports `bigint
/// unsigned`. Both must stay read-only instead of binding a signed codec.
pub(super) async fn exercise_unsigned_columns(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    table: &str,
) {
    let key_table = format!("{table}_unsigned_key");
    let value_table = format!("{table}_unsigned_value");
    for statement in [
        format!(
            "CREATE TABLE `{key_table}` (id BIGINT UNSIGNED PRIMARY KEY, \
             label VARCHAR(32) NOT NULL)"
        ),
        format!("INSERT INTO `{key_table}` (id, label) VALUES (1, 'before')"),
        format!(
            "CREATE TABLE `{value_table}` (id BIGINT PRIMARY KEY, \
             qty INT UNSIGNED NOT NULL)"
        ),
        format!("INSERT INTO `{value_table}` (id, qty) VALUES (1, 5)"),
    ] {
        let mut builder = rullst_orm::_sqlx::QueryBuilder::new(statement);
        fixture(&mut builder, "mysql")
            .execute(pool)
            .await
            .expect("prepare the unsigned tables");
    }

    let post = |uri: String, body: &'static str| async move {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("valid unsigned mutation request"),
            )
            .await
            .expect("Studio unsigned mutation response")
            .status()
    };
    assert_eq!(
        post(
            format!("/studio/tables/{key_table}/rows/update"),
            "column=label&value=after&pk_id=1"
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(
            format!("/studio/tables/{value_table}/rows/update"),
            "column=qty&value=-5&pk_id=1"
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let view = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{key_table}"))
                .body(Body::empty())
                .expect("valid unsigned table request"),
        )
        .await
        .expect("Studio unsigned table response");
    assert_eq!(view.status(), StatusCode::OK);
    let view = response_text(view).await;
    assert!(view.contains("Read-only: row changes need a complete primary key"));
    assert!(!view.contains("/rows/delete"));

    let mut select = rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new(format!(
        "SELECT label FROM `{key_table}` WHERE id = 1"
    ));
    let label: String = fixture(&mut select, "mysql")
        .fetch_one(pool)
        .await
        .and_then(|row| rullst_orm::_sqlx::Row::try_get(&row, 0))
        .expect("read the unsigned-key row");
    assert_eq!(label, "before");
}
