use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_nexus::*;
use tower::ServiceExt;

struct UserModel;
impl NexusModel for UserModel {
    fn nexus_table() -> &'static str {
        "users"
    }
    fn nexus_label() -> &'static str {
        "Users"
    }
    fn nexus_icon() -> &'static str {
        "👥"
    }
    fn nexus_pk() -> &'static str {
        "id"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("username", "Username", FieldKind::Text),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
        ]
    }
}

#[tokio::test]
async fn test_nexus_dashboard_and_views() {
    let nexus = Nexus::new()
        .with_brand("Enterprise Test")
        .register::<UserModel>();

    let app = nexus.build();

    let routes = [
        "/",
        "/security",
        "/telemetry",
        "/chat",
        "/table/users",
        "/table/users/new",
        "/table/users/search?q=alice",
    ];

    for route in routes {
        let req = Request::builder()
            .uri(route)
            .body(Body::empty())
            .expect("valid request");

        let res = app.clone().oneshot(req).await.expect("handler executed");
        assert!(
            res.status().is_success() || res.status().is_redirection(),
            "Route {} returned status {:?}",
            route,
            res.status()
        );
    }
}

#[tokio::test]
async fn test_nexus_ai_chat_query() {
    let nexus = Nexus::new().register::<UserModel>();
    let app = nexus.build();

    let csrf_token = "valid_csrf_token_for_test_12345";
    let form_body = "message=How+many+users+are+in+the+system%3F";

    let req = Request::builder()
        .method("POST")
        .uri("/chat/query")
        .header(axum::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(form_body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(!body_str.is_empty());
}

#[test]
fn test_field_meta_and_kinds() {
    let text_field = FieldMeta::new("email", "Email", FieldKind::Email);
    assert_eq!(text_field.name, "email");
    assert_eq!(text_field.label, "Email");
    assert_eq!(text_field.kind, FieldKind::Email);

    let num_field = FieldMeta::new("balance", "Balance", FieldKind::Number).readonly();
    assert_eq!(num_field.label, "Balance");
    assert!(num_field.readonly);
}

#[test]
fn test_sanitize_identifier_multibyte() {
    let input = "table_name_123!@#_test";
    let clean = sanitize_identifier(input);
    assert_eq!(clean, "table_name_123_test");
    assert!(clean.len() <= 64);
}
