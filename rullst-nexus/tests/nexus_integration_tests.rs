mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use axum::http::StatusCode;
use rullst_nexus::*;
use tower::ServiceExt;

fn local_test_router(nexus: Nexus) -> axum::Router {
    authenticated_test_router(nexus)
}

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

struct MissingLifecycleModel;
impl NexusModel for MissingLifecycleModel {
    fn nexus_table() -> &'static str {
        "nexus_lifecycle_missing"
    }
    fn nexus_label() -> &'static str {
        "Unavailable records"
    }
    fn nexus_icon() -> &'static str {
        "database"
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

struct ComplexModel;
impl NexusModel for ComplexModel {
    fn nexus_table() -> &'static str {
        "complex_records"
    }
    fn nexus_label() -> &'static str {
        "Complex Records"
    }
    fn nexus_icon() -> &'static str {
        "⚙️"
    }
    fn nexus_pk() -> &'static str {
        "id"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("description", "Description", FieldKind::Textarea),
            FieldMeta::new("email", "Email Address", FieldKind::Email),
            FieldMeta::new("website", "Website URL", FieldKind::Url),
            FieldMeta::new("price", "Price", FieldKind::Number),
            FieldMeta::new("secret", "Password", FieldKind::Password),
            FieldMeta::new("created_date", "Date", FieldKind::Date),
            FieldMeta::new("updated_time", "Timestamp", FieldKind::DateTime),
            FieldMeta::new("is_published", "Published", FieldKind::Boolean),
            FieldMeta::new("metadata", "JSON Metadata", FieldKind::Json),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["active", "pending", "archived"],
                },
            ),
            FieldMeta::new(
                "category_id",
                "Category",
                FieldKind::ForeignKey {
                    table: "categories",
                    label_col: "name",
                },
            ),
        ]
    }
}

#[tokio::test]
async fn test_nexus_dashboard_and_views() {
    let nexus = Nexus::new()
        .with_brand("Enterprise Test")
        .register::<UserModel>()
        .register::<ComplexModel>();

    let app = local_test_router(nexus);

    let routes = [
        "/",
        "/security",
        "/telemetry",
        "/chat",
        "/table/users",
        "/table/users/new",
        "/table/users/1/edit",
        "/table/users/search?q=alice",
        "/table/complex_records",
        "/table/complex_records/new",
        "/table/complex_records/1/edit",
    ];

    for route in routes {
        let req = local_request()
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
async fn test_nexus_crud_unavailable_storage_fails_closed() {
    let nexus = Nexus::new().register::<MissingLifecycleModel>();
    let app = local_test_router(nexus);

    let csrf_token = "valid_csrf_token_for_test_12345";

    // 1. POST create record
    let form_body = "username=testuser&is_active=true";
    let req = local_request()
        .method("POST")
        .uri("/table/nexus_lifecycle_missing")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(form_body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 2. PUT update record
    let update_body = "username=updated_user&is_active=false";
    let req = local_request()
        .method("PUT")
        .uri("/table/nexus_lifecycle_missing/1")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(update_body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 3. DELETE record
    let req = local_request()
        .method("DELETE")
        .uri("/table/nexus_lifecycle_missing/1")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 4. An empty batch remains an intentional, side-effect-free redirect.
    let batch_body = "action=delete";
    let req = local_request()
        .method("POST")
        .uri("/table/nexus_lifecycle_missing/batch")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf_token))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::from(batch_body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn test_nexus_ai_chat_queries_and_responses() {
    let nexus = Nexus::new().register::<UserModel>();
    let app = local_test_router(nexus);

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
        let req = local_request()
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
    let app = local_test_router(nexus);

    let req = local_request()
        .uri("/table/users")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = local_request()
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

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
#[tokio::test]
async fn test_nexus_with_sqlite_db_backed_crud() {
    use rullst_orm::_sqlx::Row;

    rullst_orm::Orm::init("sqlite:file:memdb_nexus_shared?mode=memory&cache=shared")
        .await
        .expect("initialize isolated Nexus SQLite pool");

    let pool = rullst_core::db::safe_pool().expect("Nexus SQLite pool remains initialized");
    rullst_orm::_sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
    )
    .execute(pool)
    .await
    .expect("create Nexus users fixture table");
    rullst_orm::_sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .expect("reset Nexus users fixtures");
    rullst_orm::_sqlx::query(
        "INSERT INTO users (id, username, is_active) VALUES \
         (1, 'alice', 1), (2, 'bob', 0), (100, 'batch', 1), (101, 'remove', 1)",
    )
    .execute(pool)
    .await
    .expect("insert deterministic Nexus users fixtures");

    let nexus = Nexus::new()
        .with_brand("Nexus DB Suite")
        .register::<UserModel>()
        .register::<ComplexModel>();
    let app = local_test_router(nexus);

    let csrf = "valid_test_csrf_token";

    // 1. Table rows render with populated database
    let req = local_request()
        .uri("/table/users?page=1&sort_by=username&order=asc")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Search query matching 'alice'
    let req = local_request()
        .uri("/table/users/search?q=alice")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. New record form rendering
    let req = local_request()
        .uri("/table/users/new")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Edit record form rendering
    let req = local_request()
        .uri("/table/users/1/edit")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. POST create record
    let req = local_request()
        .method("POST")
        .uri("/table/users")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::from("username=charlie&is_active=1"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("create response body");
    assert_eq!(
        status,
        StatusCode::OK,
        "create returned {status}: {}",
        String::from_utf8_lossy(&body)
    );

    // 6. PUT update record
    let req = local_request()
        .method("PUT")
        .uri("/table/users/1")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::from("username=alice_updated&is_active=0"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. DELETE record
    let req = local_request()
        .method("DELETE")
        .uri("/table/users/2")
        .header("Cookie", format!("rullst_csrf={}", csrf))
        .header("X-CSRF-Token", csrf)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    {
        // 8. Batch deactivate a model that explicitly exposes a writable is_active flag.
        let req = local_request()
            .method("POST")
            .uri("/table/users/batch")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Cookie", format!("rullst_csrf={}", csrf))
            .header("X-CSRF-Token", csrf)
            .body(Body::from("action=deactivate&selected_ids=100"))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("batch deactivate response body");
        assert!(
            status.is_redirection(),
            "batch deactivate returned {status}: {}",
            String::from_utf8_lossy(&body)
        );

        let pool = rullst_core::db::safe_pool().expect("test pool remains initialized");
        let row = rullst_orm::_sqlx::query("SELECT is_active FROM users WHERE id = 100")
            .fetch_one(pool)
            .await
            .expect("deactivated fixture remains present");
        assert_eq!(row.get::<i64, _>("is_active"), 0);

        // 9. Batch delete remains bound and removes only the selected record.
        let req = local_request()
            .method("POST")
            .uri("/table/users/batch")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Cookie", format!("rullst_csrf={}", csrf))
            .header("X-CSRF-Token", csrf)
            .body(Body::from("action=delete&selected_ids=101"))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert!(res.status().is_redirection());

        let row = rullst_orm::_sqlx::query("SELECT COUNT(*) AS total FROM users WHERE id = 101")
            .fetch_one(pool)
            .await
            .expect("batch delete count query");
        assert_eq!(row.get::<i64, _>("total"), 0);
    }
}
