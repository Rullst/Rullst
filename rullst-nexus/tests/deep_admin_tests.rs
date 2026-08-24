#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
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

    let app = admin.build();

    // 1. Root / dashboard
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Table list view
    let req = Request::builder()
        .uri("/table/categories")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. New record form page
    let req = Request::builder()
        .uri("/table/categories/new")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Security dashboard in Nexus
    let req = Request::builder()
        .uri("/security")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Telemetry dashboard in Nexus
    let req = Request::builder()
        .uri("/telemetry")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. AI Chat assistant in Nexus
    let req = Request::builder().uri("/chat").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
