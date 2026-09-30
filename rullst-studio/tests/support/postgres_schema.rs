//! PostgreSQL schema resolution check for the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

/// Studio inspects `public`; its data statements must not follow `search_path`
/// (by default `"$user", public`) to a same-named table in the role's schema.
/// Run last: the role schema created here shadows unqualified names.
pub(super) async fn exercise_search_path_shadow(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    table: &str,
) {
    let shadow = format!("{table}_shadow");
    let mut role_schema = rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new(
        "SELECT CAST(current_user AS VARCHAR)",
    );
    let role: String = fixture(&mut role_schema, "postgres")
        .fetch_one(pool)
        .await
        .and_then(|row| rullst_orm::_sqlx::Row::try_get(&row, 0))
        .expect("read the matrix role");
    assert!(
        role.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "matrix role must be a plain identifier"
    );
    for statement in [
        format!("CREATE SCHEMA \"{role}\""),
        format!(
            "CREATE TABLE public.\"{shadow}\" (id BIGINT PRIMARY KEY, label VARCHAR(32) NOT NULL)"
        ),
        format!(
            "CREATE TABLE \"{role}\".\"{shadow}\" (id BIGINT PRIMARY KEY, label VARCHAR(32) NOT NULL)"
        ),
        format!("INSERT INTO public.\"{shadow}\" VALUES (1, 'public-row'), (2, 'public-two')"),
        format!("INSERT INTO \"{role}\".\"{shadow}\" VALUES (1, 'role-row'), (2, 'role-two')"),
    ] {
        let mut builder = rullst_orm::_sqlx::QueryBuilder::new(statement);
        fixture(&mut builder, "postgres")
            .execute(pool)
            .await
            .expect("prepare the search_path shadow tables");
    }

    let view = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{shadow}"))
                .body(Body::empty())
                .expect("valid shadow table request"),
        )
        .await
        .expect("Studio shadow table response");
    assert_eq!(view.status(), StatusCode::OK);
    let view = response_text(view).await;
    assert!(view.contains("public-row"), "Studio read another schema");
    assert!(!view.contains("role-row"), "Studio read another schema");

    for (suffix, body) in [
        (
            "update",
            "column=label&value=public-edited&pk_id=1".to_string(),
        ),
        ("delete", format!("confirm=DELETE+{shadow}&pk_id=2")),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/studio/tables/{shadow}/rows/{suffix}"))
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .expect("valid shadow mutation request"),
            )
            .await
            .expect("Studio shadow mutation response");
        assert!(response.status().is_redirection(), "{suffix}");
    }

    let shadow = shadow.as_str();
    let rows = |schema: String| async move {
        let mut select = rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new(
            format!("SELECT label FROM \"{schema}\".\"{shadow}\" ORDER BY id"),
        );
        fixture(&mut select, "postgres")
            .fetch_all(pool)
            .await
            .expect("read shadow rows")
            .iter()
            .map(|row| rullst_orm::_sqlx::Row::try_get::<String, _>(row, 0))
            .collect::<Result<Vec<_>, _>>()
            .expect("decode shadow labels")
    };
    assert_eq!(rows("public".to_string()).await, ["public-edited"]);
    assert_eq!(rows(role).await, ["role-row", "role-two"]);
}
