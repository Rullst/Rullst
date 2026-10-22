//! Required audit on a real server: every mutation commits with its summary
//! row, and an unavailable audit table rolls the mutation back.

use super::{CSRF, execute};
use crate::support::{TEST_ADMIN_USERNAME, authenticated_test_router, local_request};
use axum::{Extension, body::Body, http::StatusCode};
use rullst_core::security::TenantContext;
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusModel, create_nexus_audit_table, recent_nexus_audits,
};
use rullst_orm::{_sqlx as sqlx, RullstPool};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

struct AuditedNote;

impl NexusModel for AuditedNote {
    fn nexus_table() -> &'static str {
        "nexus_matrix_audited_notes"
    }
    fn nexus_label() -> &'static str {
        "Audited notes"
    }
    fn nexus_tenant_column() -> Option<&'static str> {
        Some("tenant_id")
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("tenant_id", "Tenant", FieldKind::Text)
                .hidden()
                .readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
        ]
    }
}

async fn send(app: &axum::Router, method: &str, uri: &str, body: &str) -> StatusCode {
    app.clone()
        .oneshot(
            local_request()
                .method(method)
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("rullst_csrf={CSRF}"))
                .header("x-csrf-token", CSRF)
                .header("x-request-id", "matrix-audit-request")
                .body(Body::from(body.to_owned()))
                .expect("valid request"),
        )
        .await
        .expect("Nexus response")
        .status()
}

async fn title(pool: &RullstPool, id: i32) -> String {
    let sql = format!("SELECT title FROM nexus_matrix_audited_notes WHERE id = {id}");
    let (title,): (String,) = sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_one(pool)
        .await
        .expect("audited note title");
    title
}

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after 1970")
            .as_millis(),
    )
    .expect("millisecond clock")
}

pub async fn required_audit_round_trip(pool: &RullstPool, driver: &str) {
    let serial = if driver == "postgres" {
        "SERIAL PRIMARY KEY"
    } else {
        "INT AUTO_INCREMENT PRIMARY KEY"
    };
    execute(
        pool,
        &format!(
            "CREATE TABLE nexus_matrix_audited_notes (id {serial}, \
             tenant_id VARCHAR(64) NOT NULL, title VARCHAR(255) NOT NULL, \
             is_active INTEGER NOT NULL)"
        ),
    )
    .await;
    create_nexus_audit_table()
        .await
        .expect("install the Nexus audit table");
    let app =
        authenticated_test_router(Nexus::new().register::<AuditedNote>().with_required_audit())
            .layer(Extension(
                TenantContext::try_new("acme").expect("valid tenant"),
            ));
    let before = now_ms();

    let created = send(
        &app,
        "POST",
        "/table/nexus_matrix_audited_notes",
        "title=first&is_active=1",
    )
    .await;
    assert_eq!(created, StatusCode::OK, "{driver} audited create");
    let (id,): (i32,) =
        sqlx::query_as("SELECT id FROM nexus_matrix_audited_notes WHERE title = 'first'")
            .fetch_one(pool)
            .await
            .expect("created note id");
    let uri = format!("/table/nexus_matrix_audited_notes/{id}");
    assert_eq!(
        send(&app, "PUT", &uri, "title=second").await,
        StatusCode::OK
    );
    let batch = send(
        &app,
        "POST",
        "/table/nexus_matrix_audited_notes/batch",
        &format!("action=deactivate&selected_ids={id}"),
    )
    .await;
    assert!(batch.is_redirection(), "{driver} audited batch: {batch}");
    let after = now_ms();

    let audits = recent_nexus_audits(20, Some("acme"))
        .await
        .expect("read the audit trail");
    let mut actions = audits
        .iter()
        .map(|audit| audit.action.as_str())
        .collect::<Vec<_>>();
    actions.sort_unstable();
    assert_eq!(
        actions,
        ["batch_deactivate", "create", "update"],
        "{driver}"
    );
    let key = id.to_string();
    for audit in &audits {
        assert_eq!(audit.actor_id, TEST_ADMIN_USERNAME);
        assert_eq!(audit.tenant_id.as_deref(), Some("acme"));
        assert_eq!(audit.table_name, "nexus_matrix_audited_notes");
        assert_eq!(audit.record_count, 1);
        assert_eq!(audit.outcome, "committed");
        assert_eq!(audit.format_version, 1);
        assert_eq!(
            audit.correlation_id.as_deref(),
            Some("matrix-audit-request")
        );
        assert!(
            (before..=after).contains(&audit.occurred_at_ms),
            "{driver}: {} outside {before}..={after}",
            audit.occurred_at_ms
        );
        // A batch records its count, not its keys.
        let expected_key = (audit.action == "update").then_some(key.as_str());
        if audit.action != "create" {
            assert_eq!(audit.record_key.as_deref(), expected_key, "{driver}");
        }
    }

    // Without its audit table a required-audit mutation is refused and its
    // change rolled back.
    execute(pool, "DROP TABLE rullst_nexus_audits").await;
    assert_eq!(
        send(&app, "PUT", &uri, "title=never-committed").await,
        StatusCode::SERVICE_UNAVAILABLE,
        "{driver}"
    );
    assert_eq!(title(pool, id).await, "second", "{driver}");
}
