//! Composite primary keys on PostgreSQL and MySQL: an integer and a text
//! column, each bound with its own type.

use super::{execute, send};
use axum::http::StatusCode;
use rullst_nexus::{FieldKind, FieldMeta, NexusModel};
use rullst_orm::{_sqlx as sqlx, RullstPool};

pub struct Progress;

impl NexusModel for Progress {
    fn nexus_table() -> &'static str {
        "nexus_matrix_progress"
    }
    fn nexus_label() -> &'static str {
        "Progress"
    }
    fn nexus_pk_columns() -> Vec<&'static str> {
        vec!["user_id", "lesson_id"]
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("user_id", "Learner", FieldKind::Number),
            FieldMeta::new("lesson_id", "Lesson", FieldKind::Text),
            FieldMeta::new("percent", "Percent", FieldKind::Number),
        ]
    }
}

pub async fn create_table(pool: &RullstPool) {
    execute(
        pool,
        "CREATE TABLE nexus_matrix_progress (user_id INTEGER NOT NULL, \
         lesson_id VARCHAR(64) NOT NULL, percent INTEGER NOT NULL, \
         PRIMARY KEY (user_id, lesson_id))",
    )
    .await;
    execute(
        pool,
        "INSERT INTO nexus_matrix_progress (user_id, lesson_id, percent) VALUES \
         (1, 'intro', 10), (1, 'a,b', 20), (2, 'intro', 30)",
    )
    .await;
}

async fn percent(pool: &RullstPool, user_id: i32, lesson_id: &str) -> Option<i32> {
    let sql = format!(
        "SELECT percent FROM nexus_matrix_progress \
         WHERE user_id = {user_id} AND lesson_id = '{lesson_id}'"
    );
    sqlx::query_as::<_, (i32,)>(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_optional(pool)
        .await
        .expect("read progress")
        .map(|(percent,)| percent)
}

/// NEXUS-27: only the pair names a record, through every CRUD route.
pub async fn composite_keys_round_trip(app: &axum::Router, pool: &RullstPool, driver: &str) {
    let (status, list) = send(app, "GET", "/table/nexus_matrix_progress", "").await;
    assert_eq!(status, StatusCode::OK, "{driver}");
    assert!(
        list.contains("data-nexus-row-id=\"1,a%2Cb\""),
        "{driver}: {list}"
    );

    let record = "/table/nexus_matrix_progress/record/1%2Ca%252Cb";
    let (status, form) = send(app, "GET", &format!("{record}/edit"), "").await;
    assert_eq!(status, StatusCode::OK, "{driver}");
    assert!(form.contains("value=\"20\""), "{driver}: {form}");
    let (status, body) = send(app, "PUT", record, "percent=25").await;
    assert_eq!(status, StatusCode::OK, "{driver}: {body}");
    assert_eq!(percent(pool, 1, "a,b").await, Some(25), "{driver}");
    assert_eq!(percent(pool, 1, "intro").await, Some(10), "{driver}");
    assert_eq!(percent(pool, 2, "intro").await, Some(30), "{driver}");

    let (status, body) = send(
        app,
        "POST",
        "/table/nexus_matrix_progress",
        "user_id=3&lesson_id=wrap&percent=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver}: {body}");
    let (status, body) = send(
        app,
        "POST",
        "/table/nexus_matrix_progress/batch",
        "action=delete&selected_ids=2%2Cintro&selected_ids=3%2Cwrap",
    )
    .await;
    assert!(status.is_redirection(), "{driver}: {status} {body}");
    assert_eq!(percent(pool, 2, "intro").await, None, "{driver}");
    assert_eq!(percent(pool, 3, "wrap").await, None, "{driver}");

    let (status, _) = send(
        app,
        "DELETE",
        "/table/nexus_matrix_progress/record/1%2Cintro",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver}");
    assert_eq!(percent(pool, 1, "intro").await, None, "{driver}");
    assert_eq!(percent(pool, 1, "a,b").await, Some(25), "{driver}");
}
