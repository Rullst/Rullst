#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use axum::http::StatusCode;
use rullst_nexus::*;
use tower::ServiceExt;

struct CategoryModel;
impl NexusModel for CategoryModel {
    fn nexus_table() -> &'static str {
        "categories"
    }
    fn nexus_label() -> &'static str {
        "Categories"
    }
    fn nexus_icon() -> &'static str {
        "🏷️"
    }
    fn nexus_pk() -> &'static str {
        "id"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("name", "Category Name", FieldKind::Text),
            FieldMeta::new("slug", "URL Slug", FieldKind::Text),
            FieldMeta::new("description", "Description", FieldKind::Textarea),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
        ]
    }
}

#[tokio::test]
async fn test_nexus_admin_builder_and_extended_routes() {
    let admin = Nexus::new()
        .with_brand("Acme Admin Suite")
        .register::<CategoryModel>();

    let app = authenticated_test_router(admin);

    // 1. Root / dashboard
    let req = local_request().uri("/").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Table list view
    let req = local_request()
        .uri("/table/categories")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. New record form page
    let req = local_request()
        .uri("/table/categories/new")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Security dashboard in Nexus
    let req = local_request()
        .uri("/security")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Telemetry dashboard in Nexus
    let req = local_request()
        .uri("/telemetry")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. AI Chat assistant in Nexus
    let req = local_request().uri("/chat").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
