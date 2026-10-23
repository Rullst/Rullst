//! Nexus CRUD against real PostgreSQL and MySQL servers (Docker matrix).
//!
//! Typed columns, text keys, search case and the exact tenant scope all depend
//! on the server's type system and collations, which SQLite cannot show.

mod audit;
mod composite;
mod edges;
mod temporal;

use crate::support::{authenticated_test_router, local_request};
use axum::{Extension, body::Body, http::StatusCode};
use rullst_core::security::TenantContext;
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use rullst_orm::{_sqlx as sqlx, Orm, RullstPool};
use tower::ServiceExt;

const CSRF: &str = "nexus_matrix_csrf_fixture";

pub fn handle_container_start_error(provider: &str, error: impl std::fmt::Display) {
    if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
        panic!("{provider} testcontainer is required but failed to start: {error}");
    }
    eprintln!("skipping {provider} Nexus matrix: {error}");
}

/// Tenant-scoped rows with integer, floating-point, native Boolean,
/// `Blueprint::boolean` (INTEGER) and relation columns.
struct Product;

impl NexusModel for Product {
    fn nexus_table() -> &'static str {
        "nexus_matrix_products"
    }
    fn nexus_label() -> &'static str {
        "Products"
    }
    fn nexus_tenant_column() -> Option<&'static str> {
        Some("tenant_id")
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("tenant_id", "Tenant", FieldKind::Text)
                .readonly()
                .hidden(),
            FieldMeta::new("name", "Name", FieldKind::Text),
            FieldMeta::new("price", "Price", FieldKind::Number),
            FieldMeta::new("ratio", "Ratio", FieldKind::Number),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
            FieldMeta::new("featured", "Featured", FieldKind::Boolean),
            FieldMeta::new("archived", "Archived", FieldKind::Boolean),
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

/// A text primary key whose values look numeric.
struct Sku;

impl NexusModel for Sku {
    fn nexus_table() -> &'static str {
        "nexus_matrix_skus"
    }
    fn nexus_label() -> &'static str {
        "SKUs"
    }
    fn nexus_pk() -> &'static str {
        "sku"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("sku", "SKU", FieldKind::Text).readonly(),
            FieldMeta::new("label", "Label", FieldKind::Text),
        ]
    }
}

fn app() -> axum::Router {
    let nexus = Nexus::new()
        .register::<Product>()
        .register::<Sku>()
        .register::<composite::Progress>()
        .register::<temporal::Appointment>();
    authenticated_test_router(nexus).layer(Extension(
        TenantContext::try_new("acme").expect("valid tenant"),
    ))
}

async fn send(app: &axum::Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            local_request()
                .method(method)
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("rullst_csrf={CSRF}"))
                .header("x-csrf-token", CSRF)
                .body(Body::from(body.to_owned()))
                .expect("valid request"),
        )
        .await
        .expect("Nexus response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn execute(pool: &RullstPool, sql: &str) {
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(pool)
        .await
        .unwrap_or_else(|error| panic!("fixture statement failed: {sql}: {error}"));
}

/// Reads `columns` of the product called `name` as text, joined by `|`.
async fn product(pool: &RullstPool, driver: &str, name: &str, columns: &[&str]) -> String {
    let text = if driver == "postgres" { "TEXT" } else { "CHAR" };
    let select = columns
        .iter()
        .map(|column| format!("COALESCE(CAST({column} AS {text}), 'NULL')"))
        .collect::<Vec<_>>()
        .join(", '|', ");
    let sql = format!("SELECT CONCAT({select}) FROM nexus_matrix_products WHERE name = '{name}'");
    let (value,): (String,) = sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("read product {name}: {error}"));
    value
}

fn boolean(driver: &str, value: bool) -> &'static str {
    match (driver, value) {
        ("postgres", true) => "true",
        ("postgres", false) => "false",
        (_, true) => "1",
        (_, false) => "0",
    }
}

pub async fn exercise(database_url: &str, driver: &str) {
    Orm::init(database_url)
        .await
        .expect("initialize the matrix database");
    let pool = Orm::try_pool().expect("matrix pool");
    // MySQL's BOOLEAN is TINYINT(1), which the `sqlx::Any` driver cannot
    // decode; PostgreSQL keeps a native BOOLEAN column.
    let (serial, text, float, native_boolean) = if driver == "postgres" {
        ("SERIAL PRIMARY KEY", "TEXT", "DOUBLE PRECISION", "BOOLEAN")
    } else {
        (
            "INT AUTO_INCREMENT PRIMARY KEY",
            "VARCHAR(255)",
            "DOUBLE",
            "INTEGER",
        )
    };
    execute(
        pool,
        &format!(
            "CREATE TABLE nexus_matrix_products (id {serial}, tenant_id VARCHAR(64) NOT NULL, \
             name {text} NOT NULL, price INTEGER, ratio {float}, is_active INTEGER NOT NULL, \
             featured {native_boolean} NOT NULL, category_id INTEGER, \
             archived BIGINT NOT NULL DEFAULT 0)"
        ),
    )
    .await;
    execute(
        pool,
        &format!("CREATE TABLE nexus_matrix_skus (sku VARCHAR(64) PRIMARY KEY, label {text})"),
    )
    .await;
    execute(
        pool,
        "INSERT INTO nexus_matrix_products \
         (tenant_id, name, price, ratio, is_active, featured, category_id) VALUES \
         ('acme', 'own-row', 1, 0.5, 1, TRUE, NULL), \
         ('Acme', 'foreign-row', 2, 1.5, 1, TRUE, NULL)",
    )
    .await;
    execute(
        pool,
        "INSERT INTO nexus_matrix_skus (sku, label) VALUES ('1001', 'original')",
    )
    .await;
    composite::create_table(pool).await;
    temporal::create_table(pool, driver).await;
    let app = app();

    typed_columns_round_trip(&app, pool, driver).await;
    tenant_scope_is_exact(&app, pool, driver).await;
    text_keys_are_not_bound_as_integers(&app).await;
    composite::composite_keys_round_trip(&app, pool, driver).await;
    temporal::native_date_time_columns_round_trip(&app, pool, driver).await;
    audit::required_audit_round_trip(pool, driver).await;
    edges::edge_cases(pool, driver).await;
}

/// NEXUS-05 and NX2-04: numbers, relations and both Boolean column shapes
/// are written through create, update, the edit form and batch deactivate.
async fn typed_columns_round_trip(app: &axum::Router, pool: &RullstPool, driver: &str) {
    let columns = [
        "price",
        "ratio",
        "is_active",
        "featured",
        "category_id",
        "tenant_id",
    ];
    let (status, body) = send(
        app,
        "POST",
        "/table/nexus_matrix_products",
        "name=created&price=42&ratio=2.5&is_active=1&featured=1&category_id=7",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver} create: {body}");
    assert_eq!(
        product(pool, driver, "created", &columns).await,
        format!("42|2.5|1|{}|7|acme", boolean(driver, true))
    );

    let (id,): (i32,) =
        sqlx::query_as("SELECT id FROM nexus_matrix_products WHERE name = 'created'")
            .fetch_one(pool)
            .await
            .expect("created product id");
    let uri = format!("/table/nexus_matrix_products/{id}");
    let (status, body) = send(
        app,
        "PUT",
        &uri,
        "price=43&ratio=&featured=0&is_active=0&category_id=",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{driver} update: {body}");
    assert_eq!(
        product(pool, driver, "created", &columns).await,
        format!("43|NULL|0|{}|NULL|acme", boolean(driver, false))
    );

    let (status, form) = send(app, "GET", &format!("{uri}/edit"), "").await;
    assert_eq!(status, StatusCode::OK, "{driver} edit form");
    assert!(
        form.contains("name=\"price\" value=\"43\""),
        "{driver}: {form}"
    );
    assert!(
        form.contains("name=\"ratio\" value=\"\" class=\"nexus-input\" placeholder=\"NULL\""),
        "{driver}: {form}"
    );

    let (status, _) = send(app, "PUT", &uri, "is_active=1&archived=1").await;
    assert_eq!(status, StatusCode::OK);
    // Each Boolean column shape reads back for the edit form and the list:
    // INTEGER, BIGINT and native BOOLEAN.
    let (_, form) = send(app, "GET", &format!("{uri}/edit"), "").await;
    for (field, checked) in [("is_active", true), ("archived", true), ("featured", false)] {
        assert_eq!(
            form.contains(&format!("name=\"{field}\" value=\"1\" checked")),
            checked,
            "{driver} {field}: {form}"
        );
    }
    let row = listed_row(app, id).await;
    assert_eq!(row.matches("✅ Yes").count(), 2, "{driver}: {row}");
    assert_eq!(row.matches("❌ No").count(), 1, "{driver}: {row}");
    assert!(
        row.contains("<td class=\"nexus-td\">43</td>"),
        "{driver}: {row}"
    );
    assert_eq!(
        row.matches("<td class=\"nexus-td nexus-muted\">NULL</td>")
            .count(),
        2,
        "{driver}: {row}"
    );
    let (status, body) = send(
        app,
        "POST",
        "/table/nexus_matrix_products/batch",
        &format!("action=deactivate&selected_ids={id}"),
    )
    .await;
    assert!(status.is_redirection(), "{driver} batch: {status} {body}");
    assert_eq!(product(pool, driver, "created", &["is_active"]).await, "0");
    let row = listed_row(app, id).await;
    assert_eq!(row.matches("✅ Yes").count(), 1, "{driver}: {row}");
    assert_eq!(row.matches("❌ No").count(), 2, "{driver}: {row}");
}

/// The `<tr>` of the listed product `id`.
async fn listed_row(app: &axum::Router, id: i32) -> String {
    let (status, list) = send(app, "GET", "/table/nexus_matrix_products", "").await;
    assert_eq!(status, StatusCode::OK);
    let start = list
        .find(&format!("data-nexus-row-id=\"{id}\""))
        .unwrap_or_else(|| panic!("row {id} is listed: {list}"));
    let end = list[start..].find("</tr>").expect("row end") + start;
    list[start..end].to_string()
}

/// NEXUS-08 and NX2-11: `Acme` is another tenant than `acme`, even under a
/// case-insensitive MySQL collation, and search ignores case on PostgreSQL.
async fn tenant_scope_is_exact(app: &axum::Router, pool: &RullstPool, driver: &str) {
    let (status, list) = send(app, "GET", "/table/nexus_matrix_products", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(list.contains("own-row"), "{driver}: {list}");
    assert!(!list.contains("foreign-row"), "{driver}: {list}");

    let (_, found) = send(app, "GET", "/table/nexus_matrix_products/search?q=OWN", "").await;
    assert!(found.contains("own-row"), "{driver}: {found}");
    assert!(!found.contains("foreign-row"), "{driver}: {found}");

    let (foreign,): (i32,) =
        sqlx::query_as("SELECT id FROM nexus_matrix_products WHERE name = 'foreign-row'")
            .fetch_one(pool)
            .await
            .expect("foreign product id");
    let uri = format!("/table/nexus_matrix_products/{foreign}");
    let (status, form) = send(app, "GET", &format!("{uri}/edit"), "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{driver}: {form}");
    assert_eq!(
        send(app, "PUT", &uri, "price=99").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(send(app, "DELETE", &uri, "").await.0, StatusCode::NOT_FOUND);
    let batch = format!("action=delete&selected_ids={foreign}");
    assert_eq!(
        send(app, "POST", "/table/nexus_matrix_products/batch", &batch)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        product(pool, driver, "foreign-row", &["price", "tenant_id"]).await,
        "2|Acme"
    );
}

/// NEXUS-05: a text key such as `1001` is compared as text, not BIGINT.
async fn text_keys_are_not_bound_as_integers(app: &axum::Router) {
    let (status, form) = send(app, "GET", "/table/nexus_matrix_skus/1001/edit", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(form.contains("value=\"original\""), "{form}");
    let (status, body) = send(app, "PUT", "/table/nexus_matrix_skus/1001", "label=renamed").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = send(app, "DELETE", "/table/nexus_matrix_skus/1001", "").await;
    assert_eq!(status, StatusCode::OK);
}
