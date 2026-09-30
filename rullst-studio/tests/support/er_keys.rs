//! ER diagram key checks shared by the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

/// A column in both a primary and a foreign key is listed once, and a
/// composite foreign key pairs its columns by key position.
pub(super) async fn exercise_composite_keys(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) {
    let quote = |name: String| {
        if driver == "mysql" {
            format!("`{name}`")
        } else {
            format!("\"{name}\"")
        }
    };
    let referenced = quote(format!("{table}_er_ref"));
    let referencing = quote(format!("{table}_er_link"));
    for statement in [
        format!(
            "CREATE TABLE {referenced} (ref_x BIGINT NOT NULL, ref_y BIGINT NOT NULL, \
             PRIMARY KEY (ref_x, ref_y))"
        ),
        format!(
            "CREATE TABLE {referencing} (link_a BIGINT NOT NULL, link_b BIGINT NOT NULL, \
             PRIMARY KEY (link_a, link_b), \
             FOREIGN KEY (link_a, link_b) REFERENCES {referenced} (ref_x, ref_y))"
        ),
    ] {
        let mut builder = rullst_orm::_sqlx::QueryBuilder::new(statement);
        fixture(&mut builder, driver)
            .execute(pool)
            .await
            .expect("create the composite-key ER tables");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/er")
                .body(Body::empty())
                .expect("valid Studio ER request"),
        )
        .await
        .expect("Studio ER response");
    assert_eq!(response.status(), StatusCode::OK);
    let diagram = response_text(response).await;
    let lines = diagram.lines().map(str::trim).collect::<Vec<_>>();
    for column in ["link_a", "link_b"] {
        let key_line = format!(" {column} PK");
        let plain_line = format!(" {column}");
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.ends_with(&key_line))
                .count(),
            1,
            "{column} must be listed once as a key column"
        );
        assert!(
            !lines.iter().any(|line| line.ends_with(&plain_line)),
            "{column} must not be listed again without its key marker"
        );
    }
    assert!(diagram.contains("link_a_to_ref_x"));
    assert!(diagram.contains("link_b_to_ref_y"));
    assert!(!diagram.contains("link_a_to_ref_y"));
    assert!(!diagram.contains("link_b_to_ref_x"));
}
