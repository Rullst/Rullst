//! The tenant boundary of scoped models, without a database: every build,
//! including the all-feature one, checks that a missing tenant context fails
//! closed before any query.

mod support;
use support::{authenticated_test_router, local_request};

use axum::{
    Extension,
    body::Body,
    http::{Response, StatusCode},
};
use rullst_core::security::TenantContext;
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use tower::ServiceExt;

struct ScopedRecord;

impl NexusModel for ScopedRecord {
    fn nexus_table() -> &'static str {
        "boundary_scoped_records"
    }
    fn nexus_label() -> &'static str {
        "Scoped records"
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
        ]
    }
}

struct SharedRecord;

impl NexusModel for SharedRecord {
    fn nexus_table() -> &'static str {
        "boundary_shared_records"
    }
    fn nexus_label() -> &'static str {
        "Shared records"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ]
    }
}

fn router() -> axum::Router {
    authenticated_test_router(
        Nexus::new()
            .register::<ScopedRecord>()
            .register::<SharedRecord>(),
    )
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, String) {
    let response: Response<_> = app
        .oneshot(
            local_request()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Nexus response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("bounded body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn scoped_models_refuse_requests_without_a_tenant_context() {
    for uri in [
        "/table/boundary_scoped_records",
        "/table/boundary_scoped_records/search?q=title",
        "/table/boundary_scoped_records/new",
    ] {
        let (status, body) = get(router(), uri).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(
            body, "This model requires authenticated tenant context.",
            "{uri}"
        );
    }
    // An unscoped model needs no tenant.
    let (status, _) = get(router(), "/table/boundary_shared_records/search?q=x").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn the_search_fragment_renders_rows_for_a_tenant() {
    let app = router().layer(Extension(
        TenantContext::try_new("acme").expect("valid tenant"),
    ));
    let (status, body) = get(app, "/table/boundary_scoped_records/search?q=title").await;
    assert_eq!(status, StatusCode::OK);
    // No pool is initialized in this test binary.
    assert!(body.contains("Database not initialized"), "{body}");
    assert!(body.starts_with("<tr>"), "{body}");
}
