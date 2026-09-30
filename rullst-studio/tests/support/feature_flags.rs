//! Feature-flag page and toggle checks shared by the Studio mutation matrix.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

pub(super) async fn exercise_feature_flags(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
) {
    let feature_flags = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/features")
                .body(Body::empty())
                .expect("valid Studio feature-flags request"),
        )
        .await
        .expect("Studio feature-flags response");
    assert_eq!(feature_flags.status(), StatusCode::OK);
    // A safe GET must not create the table: the application owns its schema.
    let missing_flags = rullst_orm::_sqlx::query("SELECT COUNT(*) FROM rullst_feature_flags")
        .fetch_one(pool)
        .await;
    assert!(missing_flags.is_err());
    let feature_flags = response_text(feature_flags).await;
    assert!(feature_flags.contains("table is unavailable"));
    assert!(feature_flags.contains("CREATE TABLE rullst_feature_flags"));

    let flag_schema = if driver == "sqlite" {
        "CREATE TABLE rullst_feature_flags (name TEXT PRIMARY KEY, \
         enabled INTEGER NOT NULL DEFAULT 0, rollout_percentage INTEGER, variants TEXT)"
    } else {
        "CREATE TABLE rullst_feature_flags (name VARCHAR(255) PRIMARY KEY, \
         enabled BOOLEAN NOT NULL DEFAULT false, rollout_percentage INTEGER, variants TEXT)"
    };
    rullst_orm::_sqlx::query(flag_schema)
        .execute(pool)
        .await
        .expect("create Studio matrix feature-flag table");

    let mut insert_flag = rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new(
        "INSERT INTO rullst_feature_flags \
             (name, enabled, rollout_percentage, variants) VALUES (",
    );
    insert_flag
        .push_bind("academy_beta")
        .push(", ")
        .push_bind(false)
        .push(", ")
        .push_bind(50_i32)
        .push(", ")
        .push_bind("<script>unsafe</script>")
        .push(")");
    fixture(&mut insert_flag, driver)
        .execute(pool)
        .await
        .expect("insert Studio matrix feature flag");

    let feature_flags = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/features")
                .body(Body::empty())
                .expect("valid populated feature-flags request"),
        )
        .await
        .expect("Studio populated feature-flags response");
    assert_eq!(feature_flags.status(), StatusCode::OK);
    let feature_flags = response_text(feature_flags).await;
    assert!(feature_flags.contains("academy_beta"));
    assert!(feature_flags.contains("&lt;script&gt;unsafe&lt;/script&gt;"));
    assert!(!feature_flags.contains("<script>unsafe</script>"));

    let toggle_flag = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/studio/features/toggle/academy_beta")
                .body(Body::empty())
                .expect("valid feature-flag toggle request"),
        )
        .await
        .expect("Studio feature-flag toggle response");
    assert!(toggle_flag.status().is_redirection());

    let missing_flag = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/studio/features/toggle/not_present")
                .body(Body::empty())
                .expect("valid missing feature-flag request"),
        )
        .await
        .expect("Studio missing feature-flag response");
    assert_eq!(missing_flag.status(), StatusCode::NOT_FOUND);
}
