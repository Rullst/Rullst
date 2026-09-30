//! Stored values pass through the real SQLite query and Nexus renderers.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

use axum::{body::Body, http::StatusCode};
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusAuditPolicy, NexusModel, NexusState, RegistryEntry,
    crud::views::{render_record_form, render_table_rows},
};
use rullst_orm::{_sqlx as sqlx, _sqlx::Row, Orm, RullstPool};
use std::sync::Arc;
use support::{authenticated_test_router, local_request};
use tower::ServiceExt;

#[tokio::test]
async fn stored_cells_remain_text_including_entity_prefixes() {
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated single-connection database");
    let pool = Orm::try_pool().expect("initialized test pool");
    sqlx::query(
        "CREATE TABLE nexus_cell_fixture (id INTEGER PRIMARY KEY, value TEXT, \
         enabled INTEGER NOT NULL, quantity INTEGER NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create synthetic table");

    // Preserve the existing SQLx decoding distinction: the concrete SQLite
    // driver reads NULL as empty text; the Any driver takes the fallback.
    let null_display = if cfg!(feature = "strict-sqlite") {
        ""
    } else {
        "-"
    };
    let cases = [
        (
            Some("&#60;<strong data-nexus-probe=\"untrusted\">probe</strong>"),
            "&amp;#60;&lt;strong data-nexus-probe=&quot;untrusted&quot;&gt;probe&lt;/strong&gt;",
        ),
        (
            Some("&#x3c;<em data-nexus-probe='hex'>hex</em>"),
            "&amp;#x3c;&lt;em data-nexus-probe=&#x27;hex&#x27;&gt;hex&lt;/em&gt;",
        ),
        (Some("&#9989; literal entity"), "&amp;#9989; literal entity"),
        (Some("&#"), "&amp;#"),
        (
            Some("<b>plain</b> & \"quoted\""),
            "&lt;b&gt;plain&lt;/b&gt; &amp; &quot;quoted&quot;",
        ),
        (Some("Olá ✅ <世界>"), "Olá ✅ &lt;世界&gt;"),
        (Some(""), ""),
        (None, null_display),
    ];
    let kinds = [
        FieldKind::Text,
        FieldKind::Textarea,
        FieldKind::Email,
        FieldKind::Url,
        FieldKind::Date,
        FieldKind::DateTime,
        FieldKind::Json,
        FieldKind::Enum {
            options: vec!["pending"],
        },
    ];
    let mut browser_cases = Vec::new();
    for kind in kinds {
        let entry = RegistryEntry {
            table: "nexus_cell_fixture",
            label: "Cell fixture",
            icon: "",
            pk: "id",
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new("value", "Value", kind.clone()),
                FieldMeta::new("enabled", "Enabled", FieldKind::Boolean),
                FieldMeta::new("quantity", "Quantity", FieldKind::Number),
            ],
        };
        for (index, (value, expected)) in cases.iter().enumerate() {
            let enabled = index % 2 == 0;
            sqlx::query("DELETE FROM nexus_cell_fixture")
                .execute(pool)
                .await
                .expect("reset synthetic row");
            sqlx::query(
                "INSERT INTO nexus_cell_fixture (id, value, enabled, quantity) \
                 VALUES (1, ?, ?, 42)",
            )
            .bind(*value)
            .bind(i64::from(enabled))
            .execute(pool)
            .await
            .expect("bind stored test value");

            let html = render_table_rows(&entry, "", 1, None, None, None).await;
            assert!(
                html.contains("data-nexus-row-id=\"1\""),
                "must render a real row: {html}"
            );
            assert!(
                html.contains(&format!("<td class=\"nexus-td\">{expected}</td>")),
                "{kind:?}, case {index}: stored value must remain escaped text: {html}"
            );
            assert!(!html.contains("<strong data-nexus-probe"));
            assert!(!html.contains("<em data-nexus-probe"));
            assert!(html.contains("<td class=\"nexus-td\">42</td>"));
            // Accept either encoding here so the regression distinguishes unsafe
            // stored text from the implementation's boolean icon representation.
            let indicator = if enabled { "Yes" } else { "No" };
            assert!(html.contains(&format!(" {indicator}</td>")));
            browser_cases.push(serde_json::json!({
                "html": html, "text": value.unwrap_or(null_display), "enabled": enabled,
            }));
        }
    }
    check_browser(&browser_cases);
    password_values_are_never_rendered(pool).await;
    untouched_values_round_trip_through_an_edit(pool).await;
}

fn fixture_entry(value_kind: FieldKind) -> RegistryEntry {
    RegistryEntry {
        table: "nexus_cell_fixture",
        label: "Cell fixture",
        icon: "",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("value", "Value", value_kind),
            FieldMeta::new("enabled", "Enabled", FieldKind::Boolean),
            FieldMeta::new("quantity", "Quantity", FieldKind::Number),
        ],
    }
}

/// A Password column shows a fixed mask in the list and an empty input in the
/// edit form: the stored value never reaches the HTML.
async fn password_values_are_never_rendered(pool: &RullstPool) {
    let stored = "stored-credential-sample";
    sqlx::query("DELETE FROM nexus_cell_fixture")
        .execute(pool)
        .await
        .expect("reset synthetic row");
    sqlx::query(
        "INSERT INTO nexus_cell_fixture (id, value, enabled, quantity) VALUES (1, ?, 1, 42)",
    )
    .bind(stored)
    .execute(pool)
    .await
    .expect("insert stored value");
    let entry = fixture_entry(FieldKind::Password);

    let list = render_table_rows(&entry, "", 1, None, None, None).await;
    assert!(list.contains("data-nexus-row-id=\"1\""));
    let mask = format!("<td class=\"nexus-td\">{}</td>", "\u{2022}".repeat(8));
    assert!(list.contains(&mask));
    assert!(!list.contains(stored));

    let state = NexusState {
        registry: Arc::new(vec![entry.clone()]),
        brand: Arc::new("Nexus".to_owned()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    let form = render_record_form(&state, &entry, Some("1"), None).await;
    // The quantity proves that the edit form really read the stored row.
    assert!(form.contains("name=\"quantity\" value=\"42\""));
    assert!(form.contains("type=\"password\" name=\"value\" value=\"\""));
    assert!(!form.contains(stored));
}

fn check_browser(cases: &[serde_json::Value]) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    match std::env::var("RULLST_UI_BROWSER_TESTS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("0") => return,
        Ok("1") => {}
        other => panic!("invalid RULLST_UI_BROWSER_TESTS: {other:?}"),
    }
    let script = std::env::var_os("RULLST_NEXUS_RENDER_BROWSER_SCRIPT")
        .expect("explicit renderer browser script when browser checks are enabled");
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Node and Chromium available for browser checks");
    child
        .stdin
        .take()
        .expect("browser fixture stdin")
        .write_all(&serde_json::to_vec(cases).expect("serialize synthetic fixtures"))
        .expect("send real renderer output to browser");
    assert!(child.wait().expect("browser exit status").success());
}

struct RoundTrip;

impl NexusModel for RoundTrip {
    fn nexus_table() -> &'static str {
        "nexus_round_trip"
    }

    fn nexus_label() -> &'static str {
        "Round trip"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("views", "Views", FieldKind::Number),
            FieldMeta::new("published_at", "Published", FieldKind::DateTime),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["draft", "published"],
                },
            ),
            FieldMeta::new("meta", "Meta", FieldKind::Json),
        ]
    }
}

type StoredRow = (String, String, String, String, String, String);

async fn stored_row(pool: &RullstPool) -> StoredRow {
    let row = sqlx::query(
        "SELECT title, typeof(views) AS views_type, COALESCE(CAST(views AS TEXT), '') AS views, \
         published_at, COALESCE(status, 'NULL') AS status, typeof(meta) AS meta_type \
         FROM nexus_round_trip WHERE id = 1",
    )
    .fetch_one(pool)
    .await
    .expect("round-trip row");
    (
        row.get("title"),
        row.get("views_type"),
        row.get("views"),
        row.get("published_at"),
        row.get("status"),
        row.get("meta_type"),
    )
}

async fn put(app: &axum::Router, body: &'static str) -> StatusCode {
    let csrf = "round_trip_csrf_fixture";
    app.clone()
        .oneshot(
            local_request()
                .method("PUT")
                .uri("/table/nexus_round_trip/1")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("rullst_csrf={csrf}"))
                .header("x-csrf-token", csrf)
                .body(Body::from(body))
                .expect("update request"),
        )
        .await
        .expect("update response")
        .status()
}

/// Editing one field must not rewrite NULL, an offset date-time or an
/// unregistered enum value that the form cannot represent (NEXUS-04).
async fn untouched_values_round_trip_through_an_edit(pool: &RullstPool) {
    sqlx::query(
        "CREATE TABLE nexus_round_trip (id INTEGER PRIMARY KEY, title TEXT NOT NULL, \
         views INTEGER, published_at TEXT, status TEXT, meta TEXT)",
    )
    .execute(pool)
    .await
    .expect("create round-trip table");
    sqlx::query(
        "INSERT INTO nexus_round_trip VALUES \
         (1, 'Old', NULL, '2026-01-01T10:00:00+00:00', 'legacy', NULL)",
    )
    .execute(pool)
    .await
    .expect("insert round-trip row");
    let app = authenticated_test_router(Nexus::new().register::<RoundTrip>());

    let form = app
        .clone()
        .oneshot(
            local_request()
                .uri("/table/nexus_round_trip/1/edit")
                .body(Body::empty())
                .expect("edit form request"),
        )
        .await
        .expect("edit form response");
    let form = axum::body::to_bytes(form.into_body(), 256 * 1024)
        .await
        .expect("bounded form");
    let form = String::from_utf8(form.to_vec()).expect("UTF-8 form");
    assert!(form.contains("data-nexus-mode=\"edit\""));
    assert!(form.contains(
        "type=\"number\" name=\"views\" value=\"\" class=\"nexus-input\" placeholder=\"NULL\""
    ));
    assert!(
        form.contains("type=\"text\" name=\"published_at\" value=\"2026-01-01T10:00:00+00:00\"")
    );
    assert!(form.contains("<option value=\"legacy\" selected disabled>"));
    assert!(form.contains("placeholder=\"NULL\">\n</textarea>"));

    // nexus.js sends only the edited field.
    assert_eq!(put(&app, "title=New").await, StatusCode::OK);
    let expected: StoredRow = (
        "New".into(),
        "null".into(),
        String::new(),
        "2026-01-01T10:00:00+00:00".into(),
        "legacy".into(),
        "null".into(),
    );
    assert_eq!(stored_row(pool).await, expected);

    // An explicitly emptied typed field becomes NULL, never ''.
    assert_eq!(put(&app, "views=7").await, StatusCode::OK);
    assert_eq!(stored_row(pool).await.2, "7");
    assert_eq!(put(&app, "views=&status=&meta=").await, StatusCode::OK);
    let (_, views_type, _, _, status, meta_type) = stored_row(pool).await;
    assert_eq!(
        (views_type.as_str(), status.as_str(), meta_type.as_str()),
        ("null", "NULL", "null")
    );

    // An edited offset date-time is accepted and stored as typed.
    assert_eq!(
        put(&app, "published_at=2026-02-03T04:05:06Z").await,
        StatusCode::OK
    );
    assert_eq!(stored_row(pool).await.3, "2026-02-03T04:05:06Z");
}
