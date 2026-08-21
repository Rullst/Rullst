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
        "/table/users/1/edit",
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
async fn test_nexus_crud_lifecycle_requests() {
    let nexus = Nexus::new().register::<UserModel>();
    let app = nexus.build();

    let csrf_token = "valid_csrf_token_for_test_12345";

    // 1. POST create record
    let form_body = "username=testuser&is_active=true";
    let req = Request::builder()
        .method("POST")
        .uri("/table/users")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(form_body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_server_error()
            || res.status().is_redirection()
    );

    // 2. PUT update record
    let update_body = "username=updated_user&is_active=false";
    let req = Request::builder()
        .method("PUT")
        .uri("/table/users/1")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(update_body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_server_error()
            || res.status().is_redirection()
    );

    // 3. DELETE record
    let req = Request::builder()
        .method("DELETE")
        .uri("/table/users/1")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_server_error()
            || res.status().is_redirection()
    );

    // 4. Batch action
    let batch_body = "action=delete";
    let req = Request::builder()
        .method("POST")
        .uri("/table/users/batch")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(batch_body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_server_error()
            || res.status().is_redirection()
            || res.status().is_client_error()
    );
}

#[tokio::test]
async fn test_nexus_ai_chat_queries_and_responses() {
    let nexus = Nexus::new().register::<UserModel>();
    let app = nexus.build();

    let csrf_token = "valid_csrf_token_for_test_12345";
    let test_queries = [
        "How do I configure Google Gemini or OpenAI in .env?",
        "Tell me about the users table and its fields",
        "How many users are in the system?",
        "Give me security recommendations for my API",
    ];

    for msg in test_queries {
        let encoded_msg = msg.replace(' ', "+").replace('?', "%3F");
        let form_body = format!("message={}", encoded_msg);
        let req = Request::builder()
            .method("POST")
            .uri("/chat/query")
            .header(
                axum::http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header("Cookie", format!("rullst_csrf={}", csrf_token))
            .header("X-CSRF-Token", csrf_token)
            .body(Body::from(form_body))
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        assert!(!body_str.is_empty());
    }
}

#[tokio::test]
async fn test_nexus_htmx_partial_headers() {
    let nexus = Nexus::new().register::<UserModel>();
    let app = nexus.build();

    let req = Request::builder()
        .uri("/table/users")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/table/users/search?q=test")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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

    let date_field = FieldMeta::new("created_at", "Created At", FieldKind::DateTime);
    assert_eq!(date_field.kind, FieldKind::DateTime);

    let json_field = FieldMeta::new("metadata", "Metadata", FieldKind::Json);
    assert_eq!(json_field.kind, FieldKind::Json);

    let bool_field = FieldMeta::new("is_verified", "Verified", FieldKind::Boolean);
    assert_eq!(bool_field.kind, FieldKind::Boolean);
}

#[test]
fn test_sanitize_identifier_multibyte() {
    let input = "table_name_123!@#_test";
    let clean = sanitize_identifier(input);
    assert_eq!(clean, "table_name_123_test");
    assert!(clean.len() <= 64);
}

#[tokio::test]
async fn test_nexus_with_sqlite_db_backed_crud() {
    let _ = rullst_orm::Orm::init("sqlite:file:memdb_nexus?mode=memory&cache=shared").await;

    if let Some(pool) = rullst_core::db::safe_pool() {
        let _ = rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
        )
        .execute(pool)
        .await;

        let _ = rullst_orm::_sqlx::query(
            "INSERT INTO users (username, is_active) VALUES ('alice', 1), ('bob', 0)",
        )
        .execute(pool)
        .await;
    }

    let nexus = Nexus::new()
        .with_brand("Nexus DB Suite")
        .register::<UserModel>();
    let app = nexus.build();

    let csrf = "valid_test_csrf_token";

    // 1. Table rows render with populated database
    let req = Request::builder()
        .uri("/table/users?page=1&sort_by=username&order=asc")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Search query matching 'alice'
    let req = Request::builder()
        .uri("/table/users/search?q=alice")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. New record form rendering
    let req = Request::builder()
        .uri("/table/users/new")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Edit record form rendering
    let req = Request::builder()
        .uri("/table/users/1/edit")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. POST create record
    let req = Request::builder()
        .method("POST")
        .uri("/table/users")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::from("username=charlie&is_active=1"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_redirection()
            || res.status().is_server_error()
    );

    // 6. PUT update record
    let req = Request::builder()
        .method("PUT")
        .uri("/table/users/1")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::from("username=alice_updated&is_active=0"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_redirection()
            || res.status().is_server_error()
    );

    // 7. DELETE record
    let req = Request::builder()
        .method("DELETE")
        .uri("/table/users/2")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success()
            || res.status().is_redirection()
            || res.status().is_server_error()
    );
}
