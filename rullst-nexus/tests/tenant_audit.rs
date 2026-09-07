#![cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]

mod support;
use support::{TEST_ADMIN_USERNAME, authenticated_test_router, local_request};

use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
};
use rullst_core::security::TenantContext;
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusModel, create_nexus_audit_table, recent_nexus_audits,
};
use rullst_orm::_sqlx::Row;
use tower::ServiceExt;

const CSRF: &str = "tenant_nexus_csrf_fixture";

struct TenantRecord;

impl NexusModel for TenantRecord {
    fn nexus_table() -> &'static str {
        "nexus_tenant_records"
    }

    fn nexus_label() -> &'static str {
        "Tenant Records"
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
            FieldMeta::new("active", "Active", FieldKind::Boolean),
        ]
    }
}

fn tenant_router(tenant_id: &str, required_audit: bool) -> axum::Router {
    let mut nexus = Nexus::new().register::<TenantRecord>();
    if required_audit {
        nexus = nexus.with_required_audit();
    }
    let tenant = TenantContext::try_new(tenant_id).expect("valid test tenant");
    authenticated_test_router(nexus).layer(Extension(tenant))
}

fn mutation_request(method: &str, uri: &str, body: impl Into<Body>) -> Request<Body> {
    local_request()
        .method(method)
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", format!("rullst_csrf={CSRF}"))
        .header("x-csrf-token", CSRF)
        .header("x-request-id", "nexus-test-request-1")
        .body(body.into())
        .expect("valid mutation request")
}

async fn response_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("bounded response body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 response")
}

#[tokio::test]
// TM-NEXUS-02, TM-NEXUS-06
async fn tenant_scope_and_required_audit_are_enforced_atomically() {
    rullst_orm::Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("initialize isolated Nexus test database");
    let pool = rullst_orm::Orm::try_pool().expect("Nexus test pool initialized");
    rullst_orm::_sqlx::query(
        "CREATE TABLE nexus_tenant_records (\
         id INTEGER PRIMARY KEY AUTOINCREMENT, tenant_id TEXT NOT NULL, \
         title TEXT NOT NULL, active INTEGER NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create tenant record table");
    rullst_orm::_sqlx::query(
        "INSERT INTO nexus_tenant_records (tenant_id, title, active) \
         VALUES ('tenant-a', 'alpha-private', 1), ('tenant-b', 'beta-private', 1)",
    )
    .execute(pool)
    .await
    .expect("insert cross-tenant fixtures");
    create_nexus_audit_table()
        .await
        .expect("install Nexus audit schema");
    rullst_orm::_sqlx::query(
        "INSERT INTO rullst_nexus_audits (actor_id, tenant_id, table_name, action, \
         record_key, record_count, outcome, correlation_id, occurred_at_ms, format_version) \
         VALUES ('foreign-admin', 'tenant-b', 'nexus_tenant_records', 'update', \
         '2', 1, 'committed', 'foreign-request', 1, 1)",
    )
    .execute(pool)
    .await
    .expect("insert foreign audit fixture");

    let app = tenant_router("tenant-a", true);
    let listing = app
        .clone()
        .oneshot(
            local_request()
                .uri("/table/nexus_tenant_records")
                .body(Body::empty())
                .expect("valid list request"),
        )
        .await
        .expect("list response");
    assert_eq!(listing.status(), StatusCode::OK);
    let listing = response_text(listing).await;
    assert!(listing.contains("alpha-private"));
    assert!(!listing.contains("beta-private"));

    let cross_tenant_update = app
        .clone()
        .oneshot(mutation_request(
            "PUT",
            "/table/nexus_tenant_records/2",
            "title=stolen&active=1",
        ))
        .await
        .expect("cross-tenant update response");
    assert_eq!(cross_tenant_update.status(), StatusCode::NOT_FOUND);

    let create = app
        .clone()
        .oneshot(mutation_request(
            "POST",
            "/table/nexus_tenant_records",
            "title=created-for-a&active=1",
        ))
        .await
        .expect("tenant create response");
    assert_eq!(create.status(), StatusCode::OK);
    let created: (i64,) = rullst_orm::_sqlx::query_as(
        "SELECT id FROM nexus_tenant_records \
         WHERE tenant_id = 'tenant-a' AND title = 'created-for-a'",
    )
    .fetch_one(pool)
    .await
    .expect("created tenant row");

    let spoofed_tenant = app
        .clone()
        .oneshot(mutation_request(
            "POST",
            "/table/nexus_tenant_records",
            "tenant_id=tenant-b&title=spoofed&active=1",
        ))
        .await
        .expect("spoofed tenant create response");
    assert_eq!(spoofed_tenant.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let update = app
        .clone()
        .oneshot(mutation_request(
            "PUT",
            "/table/nexus_tenant_records/1",
            "title=alpha-updated&active=1",
        ))
        .await
        .expect("owned update response");
    assert_eq!(update.status(), StatusCode::OK);

    let batch = app
        .clone()
        .oneshot(mutation_request(
            "POST",
            "/table/nexus_tenant_records/batch",
            format!("action=deactivate&selected_ids={}", created.0),
        ))
        .await
        .expect("owned batch response");
    assert!(batch.status().is_redirection());

    let delete = app
        .clone()
        .oneshot(mutation_request(
            "DELETE",
            &format!("/table/nexus_tenant_records/{}", created.0),
            Body::empty(),
        ))
        .await
        .expect("owned delete response");
    assert_eq!(delete.status(), StatusCode::OK);

    let audits = recent_nexus_audits(20, Some("tenant-a"))
        .await
        .expect("load tenant audit records");
    assert!(audits.iter().any(|audit| {
        audit.action == "create"
            && audit.actor_id == TEST_ADMIN_USERNAME
            && audit.correlation_id.as_deref() == Some("nexus-test-request-1")
    }));
    assert!(
        audits
            .iter()
            .any(|audit| { audit.action == "update" && audit.record_key.as_deref() == Some("1") })
    );
    assert!(
        audits
            .iter()
            .any(|audit| audit.action == "batch_deactivate" && audit.record_count == 1)
    );
    let created_id = created.0.to_string();
    assert!(audits.iter().any(|audit| {
        audit.action == "delete" && audit.record_key.as_deref() == Some(created_id.as_str())
    }));
    assert!(audits.iter().all(|audit| {
        audit.tenant_id.as_deref() == Some("tenant-a")
            && audit.outcome == "committed"
            && audit.format_version == 1
    }));

    let cross_tenant_delete = app
        .clone()
        .oneshot(mutation_request(
            "DELETE",
            "/table/nexus_tenant_records/2",
            Body::empty(),
        ))
        .await
        .expect("cross-tenant delete response");
    assert_eq!(cross_tenant_delete.status(), StatusCode::NOT_FOUND);

    let cross_tenant_batch = app
        .clone()
        .oneshot(mutation_request(
            "POST",
            "/table/nexus_tenant_records/batch",
            "action=delete&selected_ids=2",
        ))
        .await
        .expect("cross-tenant batch response");
    assert_eq!(cross_tenant_batch.status(), StatusCode::NOT_FOUND);
    let foreign = rullst_orm::_sqlx::query(
        "SELECT title FROM nexus_tenant_records WHERE id = 2 AND tenant_id = 'tenant-b'",
    )
    .fetch_one(pool)
    .await
    .expect("foreign tenant fixture survives");
    assert_eq!(foreign.get::<String, _>("title"), "beta-private");

    rullst_orm::_sqlx::query("DROP TABLE rullst_nexus_audits")
        .execute(pool)
        .await
        .expect("simulate unavailable audit storage");
    let denied = app
        .oneshot(mutation_request(
            "PUT",
            "/table/nexus_tenant_records/1",
            "title=must-roll-back&active=1",
        ))
        .await
        .expect("fail-closed audit response");
    assert_eq!(denied.status(), StatusCode::SERVICE_UNAVAILABLE);
    let denied_body = response_text(denied).await;
    assert!(!denied_body.to_ascii_lowercase().contains("no such table"));
    let current: (String,) = rullst_orm::_sqlx::query_as(
        "SELECT title FROM nexus_tenant_records WHERE id = 1 AND tenant_id = 'tenant-a'",
    )
    .fetch_one(pool)
    .await
    .expect("read rolled-back row");
    assert_eq!(current.0, "alpha-updated");
}

#[tokio::test]
async fn scoped_model_denies_requests_without_trusted_tenant_context() {
    let app = authenticated_test_router(Nexus::new().register::<TenantRecord>());
    let response = app
        .oneshot(
            local_request()
                .uri("/table/nexus_tenant_records")
                .body(Body::empty())
                .expect("valid tenantless request"),
        )
        .await
        .expect("tenantless response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
