//! Composite primary keys end to end on SQLite: list, edit, update, delete,
//! batch and create address a record by every key column.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

use axum::{body::Body, http::StatusCode};
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusModel, create_nexus_audit_table, recent_nexus_audits,
};
use rullst_orm::{_sqlx as sqlx, Orm, RullstPool};
use support::{authenticated_test_router, local_request};
use tower::ServiceExt;

const CSRF: &str = "composite_key_csrf_fixture";

/// `lesson_progress (user_id, lesson_id)`: an integer and a text key column.
struct LessonProgress;

impl NexusModel for LessonProgress {
    fn nexus_table() -> &'static str {
        "lesson_progress"
    }
    fn nexus_label() -> &'static str {
        "Lesson progress"
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

async fn send(app: &axum::Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            local_request()
                .method(method)
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("rullst_csrf={CSRF}"))
                .header("x-csrf-token", CSRF)
                .body(Body::from(body.to_owned()))
                .expect("valid request"),
        )
        .await
        .expect("Nexus response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn percents(pool: &RullstPool) -> Vec<(i64, String, i64)> {
    sqlx::query_as("SELECT user_id, lesson_id, percent FROM lesson_progress ORDER BY 1, 2")
        .fetch_all(pool)
        .await
        .expect("read progress rows")
}

async fn audited(action: &str) -> Vec<Option<String>> {
    recent_nexus_audits(100, None)
        .await
        .expect("load audit records")
        .into_iter()
        .filter(|audit| audit.action == action)
        .map(|audit| audit.record_key)
        .collect()
}

#[tokio::test]
async fn composite_keys_address_one_record_on_sqlite() {
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated single-connection database");
    let pool = Orm::try_pool().expect("initialized test pool");
    sqlx::query(
        "CREATE TABLE lesson_progress (user_id INTEGER NOT NULL, lesson_id TEXT NOT NULL, \
         percent INTEGER NOT NULL, PRIMARY KEY (user_id, lesson_id))",
    )
    .execute(pool)
    .await
    .expect("create fixture table");
    // The same user and the same lesson appear in other keys: only the pair
    // names a record. `a,b` needs its separator encoded.
    sqlx::query(
        "INSERT INTO lesson_progress (user_id, lesson_id, percent) VALUES \
         (1, 'intro', 10), (1, 'a,b', 20), (2, 'intro', 30)",
    )
    .execute(pool)
    .await
    .expect("insert fixture rows");
    create_nexus_audit_table()
        .await
        .expect("install Nexus audit schema");
    let app = authenticated_test_router(
        Nexus::new()
            .register::<LessonProgress>()
            .with_required_audit(),
    );

    // The list addresses each row by both columns.
    let (status, list) = send(&app, "GET", "/table/lesson_progress", "").await;
    assert_eq!(status, StatusCode::OK);
    for address in ["1,intro", "1,a%2Cb", "2,intro"] {
        assert!(
            list.contains(&format!("data-nexus-row-id=\"{address}\"")),
            "{address}: {list}"
        );
    }
    assert!(list.contains("/nexus/table/lesson_progress/record/1%2Ca%252Cb/edit"));

    // The edit form loads exactly that record, with both key columns locked.
    let (status, form) = send(
        &app,
        "GET",
        "/table/lesson_progress/record/1%2Ca%252Cb/edit",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(form.contains("value=\"20\""), "{form}");
    assert!(!form.contains("name=\"user_id\"") && !form.contains("name=\"lesson_id\""));

    let (status, body) = send(
        &app,
        "PUT",
        "/table/lesson_progress/record/1%2Ca%252Cb",
        "percent=25",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        percents(pool).await,
        [
            (1, "a,b".to_owned(), 25),
            (1, "intro".to_owned(), 10),
            (2, "intro".to_owned(), 30)
        ]
    );
    assert_eq!(audited("update").await, [Some("1,a%2Cb".to_owned())]);

    // A wrong arity or a non-canonical spelling names no record.
    for address in ["1", "1%2Cintro%2Cx", "01%2Cintro", "1%2Ca%2Cb"] {
        let uri = format!("/table/lesson_progress/record/{address}");
        let (status, _) = send(&app, "PUT", &uri, "percent=99").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{address}");
        let (status, _) = send(&app, "GET", &format!("{uri}/edit"), "").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{address}");
    }
    let (status, _) = send(
        &app,
        "DELETE",
        "/table/lesson_progress/record/3%2Cintro",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(
        &app,
        "DELETE",
        "/table/lesson_progress/record/1%2Cintro",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(audited("delete").await, [Some("1,intro".to_owned())]);

    // A new record is entered with both key values and audited by them.
    let (status, body) = send(
        &app,
        "POST",
        "/table/lesson_progress",
        "user_id=3&lesson_id=wrap%20up&percent=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(audited("create").await, [Some("3,wrap%20up".to_owned())]);

    // The batch form posts the addresses its checkboxes carry.
    let (status, body) = send(
        &app,
        "POST",
        "/table/lesson_progress/batch",
        "action=delete&selected_ids=2%2Cintro&selected_ids=3%2Cwrap%2520up",
    )
    .await;
    assert!(status.is_redirection(), "{status} {body}");
    assert_eq!(percents(pool).await, [(1, "a,b".to_owned(), 25)]);
    let (status, _) = send(
        &app,
        "POST",
        "/table/lesson_progress/batch",
        "action=delete&selected_ids=1",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
