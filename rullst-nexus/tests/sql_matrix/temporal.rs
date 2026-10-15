//! Native DATE, TIME, TIMESTAMP and TIMESTAMPTZ (MySQL: DATETIME holding UTC)
//! columns written through Nexus forms, including NULL and invalid input.

use super::{execute, send};
use axum::http::StatusCode;
use rullst_nexus::{FieldKind, FieldMeta, NexusModel};
use rullst_orm::{_sqlx as sqlx, RullstPool};

pub struct Appointment;

impl NexusModel for Appointment {
    fn nexus_table() -> &'static str {
        "nexus_matrix_appointments"
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

pub async fn create_table(pool: &RullstPool, driver: &str) {
    let sql = if driver == "postgres" {
        "CREATE TABLE nexus_matrix_appointments (id SERIAL PRIMARY KEY, due_on DATE, \
         opens_at TIME, starts_at TIMESTAMP, paid_at TIMESTAMPTZ)"
    } else {
        "CREATE TABLE nexus_matrix_appointments (id INT AUTO_INCREMENT PRIMARY KEY, \
         due_on DATE, opens_at TIME(6), starts_at DATETIME(6), paid_at DATETIME(6))"
    };
    execute(pool, sql).await;
}

/// How many rows hold exactly these values, compared as native values.
async fn matching(pool: &RullstPool, driver: &str, paid_at: &str, opens_at: &str) -> i64 {
    let paid = if driver == "postgres" {
        format!("TIMESTAMPTZ '{paid_at}+00'")
    } else {
        format!("'{paid_at}'")
    };
    let typed = |kind: &str, value: &str| {
        if driver == "postgres" {
            format!("{kind} '{value}'")
        } else {
            format!("'{value}'")
        }
    };
    let sql = format!(
        "SELECT COUNT(*) FROM nexus_matrix_appointments WHERE due_on = {} \
         AND opens_at = {} AND starts_at = {} AND paid_at = {paid}",
        typed("DATE", "2026-10-15"),
        typed("TIME", opens_at),
        typed("TIMESTAMP", "2026-10-15 09:30:00"),
    );
    let (count,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("{driver} count appointments: {error}"));
    count
}

/// The coordinator's alpha.2 item: native date/time columns accept Nexus
/// form values, bound with their own types.
pub async fn native_date_time_columns_round_trip(
    app: &axum::Router,
    pool: &RullstPool,
    driver: &str,
) {
    let (status, body) = send(
        app,
        "POST",
        "/table/nexus_matrix_appointments",
        "due_on=2026-10-15&opens_at=09%3A30&starts_at=2026-10-15T09%3A30\
         &paid_at=2026-10-15T12%3A30",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver} create: {body}");
    assert_eq!(
        matching(pool, driver, "2026-10-15 12:30:00", "09:30:00").await,
        1,
        "{driver}"
    );

    let (id,): (i32,) = sqlx::query_as("SELECT id FROM nexus_matrix_appointments")
        .fetch_one(pool)
        .await
        .expect("appointment id");
    let record = format!("/table/nexus_matrix_appointments/record/{id}");
    let (status, form) = send(app, "GET", &format!("{record}/edit"), "").await;
    assert_eq!(status, StatusCode::OK, "{driver}");
    for expected in [
        "type=\"date\" name=\"due_on\" value=\"2026-10-15\"",
        "type=\"time\" name=\"opens_at\" value=\"09:30:00\"",
        "type=\"datetime-local\" name=\"paid_at\" value=\"2026-10-15T12:30:00\"",
    ] {
        assert!(form.contains(expected), "{driver} {expected}: {form}");
    }

    // An offset is converted to UTC before it is stored.
    let (status, body) = send(
        app,
        "PUT",
        &record,
        "paid_at=2026-10-15T07%3A30-03%3A00&opens_at=10%3A15%3A30",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver} update: {body}");
    assert_eq!(
        matching(pool, driver, "2026-10-15 10:30:00", "10:15:30").await,
        1,
        "{driver}"
    );

    let (status, body) = send(app, "PUT", &record, "due_on=2026-02-30").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{driver}: {body}");

    // NULL reaches native columns too (PostgreSQL needs the cast for it).
    let (status, body) = send(app, "PUT", &record, "due_on=&opens_at=&starts_at=&paid_at=").await;
    assert_eq!(status, StatusCode::OK, "{driver} null: {body}");
    let (nulls,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM nexus_matrix_appointments WHERE due_on IS NULL \
         AND opens_at IS NULL AND starts_at IS NULL AND paid_at IS NULL",
    )
    .fetch_one(pool)
    .await
    .expect("count null appointments");
    assert_eq!(nulls, 1, "{driver}");
}
