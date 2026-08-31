#![cfg(not(miri))]
// The suite exercises both the Nexus facade and direct ORM-backed fixtures.
// Keep it out of isolated `nexus` feature checks where the facade intentionally
// does not expose its optional `rullst-orm` dependency.
#![cfg(all(feature = "nexus", feature = "orm"))]
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use base64::Engine;
use rullst::nexus::{FieldKind, FieldMeta, Nexus, NexusModel, NexusVerifiedTls};
use rullst::testing::TestApp;
use std::net::SocketAddr;

struct TestUser;

impl NexusModel for TestUser {
    fn nexus_table() -> &'static str {
        "nexus_users"
    }

    fn nexus_label() -> &'static str {
        "Nexus Users"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta {
                name: "id",
                label: "ID",
                kind: FieldKind::Number,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "name",
                label: "Name",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "email",
                label: "Email",
                kind: FieldKind::Email,
                hidden: false,
                readonly: false,
            },
        ]
    }
}

struct TestPost;

impl NexusModel for TestPost {
    fn nexus_table() -> &'static str {
        "nexus_posts"
    }

    fn nexus_label() -> &'static str {
        "Nexus Posts"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta {
                name: "id",
                label: "ID",
                kind: FieldKind::Number,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "title",
                label: "Title",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "user_id",
                label: "Author",
                kind: FieldKind::ForeignKey {
                    table: "nexus_users",
                    label_col: "name",
                },
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "content",
                label: "Content",
                kind: FieldKind::Textarea,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "is_published",
                label: "Published",
                kind: FieldKind::Boolean,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "created_at",
                label: "Created",
                kind: FieldKind::DateTime,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "avatar",
                label: "Avatar",
                kind: FieldKind::Url,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "metadata",
                label: "Metadata",
                kind: FieldKind::Json,
                hidden: false,
                readonly: false,
            },
        ]
    }
}

static INIT_DB: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

fn trusted_nexus_app(nexus: Nexus) -> TestApp {
    let peer = "192.0.2.50:443"
        .parse::<SocketAddr>()
        .expect("valid Nexus test peer");
    let router = nexus
        .try_build()
        .expect("valid Nexus configuration")
        .layer(axum::Extension(
            NexusVerifiedTls::from_trusted_tls_termination(),
        ))
        .layer(axum::Extension(axum::extract::ConnectInfo(peer)));
    TestApp::new(router)
}

async fn init_test_db() {
    INIT_DB.get_or_init(|| async {
        let db_path = "sqlite:file:nexus_test.db?mode=rwc";
        if let Err(e) = rullst_orm::Orm::init(db_path).await
            && !e.to_string().contains("already been initialized")
        {
            panic!("Orm::init failed: {:?}", e);
        }
        let pool = rullst::db::safe_pool().expect("pool should be initialized");

        // Clean up tables just in case
        let _ = sqlx::query("DROP TABLE IF EXISTS nexus_users").execute(pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS nexus_posts").execute(pool).await;

        // Create table
        sqlx::query("CREATE TABLE nexus_users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE nexus_posts (id INTEGER PRIMARY KEY, title TEXT, user_id INTEGER, content TEXT, is_published INTEGER, created_at TEXT, avatar TEXT, metadata TEXT);")
            .execute(pool)
            .await
            .unwrap();

        // Insert data
        sqlx::query("INSERT INTO nexus_users (name, email) VALUES ('Alice', 'alice@example.com'), ('Bob', 'bob@example.com');")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO nexus_posts (title, user_id, is_published, content) VALUES ('First Post', 1, 1, 'Hello World');")
            .execute(pool)
            .await
            .unwrap();
    }).await;
}

#[tokio::test]
async fn test_nexus_full_flow() {
    init_test_db().await;

    // Secure panel with auth
    let nexus = Nexus::new()
        .register::<TestUser>()
        .with_brand("Nexus Custom App")
        .with_auth("admin", "unique-test-secret-42");

    let app = trusted_nexus_app(nexus);

    // 1. UNAUTHORIZED access without auth headers
    let res_unauth = app.get("/").await;
    res_unauth.assert_status(401);

    // 2. AUTHORIZED dashboard
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("admin:unique-test-secret-42")
    );
    let res_dash = app.get("/").header("Authorization", &auth_header).await;
    res_dash.assert_status(200);
    res_dash.assert_see("Dashboard");
    res_dash.assert_see("Nexus Users");

    // 3. Table list view
    let res_table = app
        .get("/table/nexus_users")
        .header("Authorization", &auth_header)
        .await;
    res_table.assert_status(200);
    res_table.assert_see("Alice");
    res_table.assert_see("bob@example.com");
    res_table.assert_see("data-nexus-row-id=\"1\"");
    res_table.assert_see("data-nexus-row-id=\"2\"");

    // 4. Search view
    let res_search = app
        .get("/table/nexus_users/search?q=Alice")
        .header("Authorization", &auth_header)
        .await;
    res_search.assert_status(200);
    res_search.assert_see("Alice");
    res_search.assert_see("data-nexus-row-id=\"1\"");
    res_search.assert_dont_see("Bob");

    // 5. New Form rendering
    let res_form = app
        .get("/table/nexus_users/new")
        .header("Authorization", &auth_header)
        .await;
    res_form.assert_status(200);
    res_form.assert_see("New Nexus User");

    // 6. Create record POST
    let form_data = [("name", "Charlie"), ("email", "charlie@example.com")];
    let res_create = app
        .post("/table/nexus_users")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&form_data)
        .await;
    res_create.assert_status(200);
    res_create.assert_see("created successfully");

    // Verify it was created
    let res_check = app
        .get("/table/nexus_users")
        .header("Authorization", &auth_header)
        .await;
    res_check.assert_see("Charlie");

    // 7. Edit Form rendering
    let res_edit = app
        .get("/table/nexus_users/2/edit")
        .header("Authorization", &auth_header)
        .await;
    res_edit.assert_status(200);
    res_edit.assert_see("Save Record");

    // 8. Update record PUT
    let update_data = [("name", "Bobby"), ("email", "bobby@example.com")];
    let res_update = app
        .put("/table/nexus_users/2")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&update_data)
        .await;
    res_update.assert_status(200);
    res_update.assert_see("updated successfully");

    // 9. Delete record DELETE
    let res_delete = app
        .delete("/table/nexus_users/2")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .await;
    res_delete.assert_status(200);
    res_delete.assert_see("deleted");

    // 10. AI Chat Page
    let res_chat = app.get("/chat").header("Authorization", &auth_header).await;
    res_chat.assert_status(200);
    res_chat.assert_see("AI Query Assistant");
}

#[tokio::test]
async fn test_nexus_foreign_key_and_kinds() {
    init_test_db().await;

    let nexus = Nexus::new()
        .register::<TestUser>()
        .register::<TestPost>()
        .with_auth("admin", "unique-test-secret-42");

    let app = trusted_nexus_app(nexus);
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("admin:unique-test-secret-42")
    );

    // 1. Table list view for nexus_posts (should render foreign key relationship)
    let res_table = app
        .get("/table/nexus_posts")
        .header("Authorization", &auth_header)
        .await;
    res_table.assert_status(200);
    res_table.assert_see("First Post");

    // 2. New Form rendering (should query foreign keys to build select dropdown)
    let res_form = app
        .get("/table/nexus_posts/new")
        .header("Authorization", &auth_header)
        .await;
    res_form.assert_status(200);
    res_form.assert_see("New Nexus Post");
    // Should have field for Author user_id
    res_form.assert_see("name=\"user_id\"");
    // Should have textarea for LongText
    res_form.assert_see("<textarea");
    // Should have checkbox for Boolean
    res_form.assert_see("type=\"checkbox\"");
    // Should have Save button
    res_form.assert_see("Save Record");

    // 3. Edit Form rendering
    let res_edit = app
        .get("/table/nexus_posts/1/edit")
        .header("Authorization", &auth_header)
        .await;
    res_edit.assert_status(200);
    res_edit.assert_see("Save Record");
    res_edit.assert_see("Hello World");

    let form_data = [
        ("title", "Second Post"),
        ("user_id", "2"),
        ("content", "Testing content"),
        ("is_published", "on"),
        ("metadata", "{}"),
    ];
    let res_create = app
        .post("/table/nexus_posts")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&form_data)
        .await;
    res_create.assert_status(200);

    let chat_data = [("message", "Show all users")];
    let res_chat = app
        .post("/chat/query")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&chat_data)
        .await;
    res_chat.assert_status(200);
}

static NEXUS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_nexus_invalid_tables_and_errors() {
    let _guard = NEXUS_LOCK.lock().await;
    init_test_db().await;

    let nexus = Nexus::new()
        .register::<TestUser>()
        .with_auth("admin", "unique-test-secret-42");

    let app = trusted_nexus_app(nexus);
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("admin:unique-test-secret-42")
    );

    // 1. Invalid Table Read
    let res = app
        .get("/table/invalid_table")
        .header("Authorization", &auth_header)
        .await;
    res.assert_see("Table not found");

    // 2. Invalid Table Search
    let res = app
        .get("/table/invalid_table/search?q=1")
        .header("Authorization", &auth_header)
        .await;
    res.assert_see("Table not found");

    // 3. Invalid Table Edit Form
    let res = app
        .get("/table/invalid_table/1/edit")
        .header("Authorization", &auth_header)
        .await;
    res.assert_see("Table not found");

    // 4. Invalid Table PUT
    let res = app
        .put("/table/invalid_table/1")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&[("name", "Bob")])
        .await;
    res.assert_status(404);

    // 5. Invalid Table DELETE
    let res = app
        .delete("/table/invalid_table/1")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .await;
    res.assert_status(404);

    // 6. DB Error Simulation (Create without required fields, or missing table)
    let res = app
        .post("/table/invalid_table")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&[("name", "Bob")])
        .await;
    res.assert_status(404);

    // 7. Chat Query Testing
    let res = app
        .post("/chat/query")
        .header("Authorization", &auth_header)
        .header("cookie", "rullst_csrf=test_csrf")
        .header("x-csrf-token", "test_csrf")
        .form(&[("message", "Show all users")])
        .await;
    // We expect it to hit the endpoint (though the AI config might be offline and return an error HTML block)
    res.assert_status(200);
}
