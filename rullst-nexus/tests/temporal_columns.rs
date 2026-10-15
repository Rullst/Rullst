//! Date, time, date-time and UTC fields end to end on SQLite: Nexus parses
//! each per its kind, stores the canonical text, shows it in the matching
//! picker and answers malformed input with a validation error.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

use axum::{body::Body, http::StatusCode};
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use rullst_orm::{_sqlx as sqlx, Orm, RullstPool};
use support::{authenticated_test_router, local_request};
use tower::ServiceExt;

const CSRF: &str = "temporal_columns_csrf_fixture";

struct Appointment;

impl NexusModel for Appointment {
    fn nexus_table() -> &'static str {
        "appointments"
    }
    fn nexus_label() -> &'static str {
        "Appointments"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("due_on", "Due on", FieldKind::Date),
            FieldMeta::new("opens_at", "Opens at", FieldKind::Time),
            FieldMeta::new("starts_at", "Starts at", FieldKind::DateTime),
            FieldMeta::new("paid_at", "Paid at", FieldKind::DateTimeUtc),
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

type Row = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

async fn stored(pool: &RullstPool) -> Row {
    sqlx::query_as("SELECT due_on, opens_at, starts_at, paid_at FROM appointments WHERE id = 1")
        .fetch_one(pool)
        .await
        .expect("read appointment")
}

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

#[tokio::test]
async fn date_and_time_fields_round_trip_on_sqlite() {
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated single-connection database");
    let pool = Orm::try_pool().expect("initialized test pool");
    sqlx::query(
        "CREATE TABLE appointments (id INTEGER PRIMARY KEY, due_on TEXT, opens_at TEXT, \
         starts_at TEXT, paid_at TEXT)",
    )
    .execute(pool)
    .await
    .expect("create fixture table");
    let app = authenticated_test_router(Nexus::new().register::<Appointment>());

    // What the browser pickers submit; the UTC field has no offset.
    let (status, body) = send(
        &app,
        "POST",
        "/table/appointments",
        "due_on=2026-10-15&opens_at=09%3A30&starts_at=2026-10-15T09%3A30\
         &paid_at=2026-10-15T12%3A30",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        stored(pool).await,
        (
            text("2026-10-15"),
            text("09:30:00"),
            text("2026-10-15 09:30:00"),
            text("2026-10-15T12:30:00+00:00")
        )
    );

    let (status, form) = send(&app, "GET", "/table/appointments/record/1/edit", "").await;
    assert_eq!(status, StatusCode::OK);
    for expected in [
        "type=\"date\" name=\"due_on\" value=\"2026-10-15\"",
        "type=\"time\" name=\"opens_at\" value=\"09:30:00\"",
        "type=\"datetime-local\" name=\"starts_at\" value=\"2026-10-15T09:30:00\"",
        "type=\"datetime-local\" name=\"paid_at\" value=\"2026-10-15T12:30:00\"",
        "Paid at (UTC)",
    ] {
        assert!(form.contains(expected), "{expected}: {form}");
    }

    // An offset is converted to UTC; a fraction is kept.
    let (status, body) = send(
        &app,
        "PUT",
        "/table/appointments/record/1",
        "paid_at=2026-10-15T09%3A30%3A00.250-03%3A00&opens_at=18%3A45%3A10.5",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, opens_at, _, paid_at) = stored(pool).await;
    assert_eq!(opens_at, text("18:45:10.500"));
    assert_eq!(paid_at, text("2026-10-15T12:30:00.250+00:00"));

    for (body, field) in [
        ("due_on=2026-02-30", "due_on"),
        ("opens_at=25%3A00", "opens_at"),
        ("starts_at=2026-10-15T09%3A61", "starts_at"),
        ("paid_at=2026-10-15T09%3A30%2B25%3A00", "paid_at"),
    ] {
        let (status, message) = send(&app, "PUT", "/table/appointments/record/1", body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(message.contains(&format!("field `{field}`")), "{message}");
    }

    // Emptied, every field stores NULL.
    let (status, body) = send(
        &app,
        "PUT",
        "/table/appointments/record/1",
        "due_on=&opens_at=&starts_at=&paid_at=",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(stored(pool).await, (None, None, None, None));
}
