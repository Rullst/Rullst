//! Stored values pass through the real SQLite query and Nexus cell renderer.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_nexus::{FieldKind, FieldMeta, RegistryEntry, crud::views::render_table_rows};
use rullst_orm::{_sqlx as sqlx, Orm};

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
        FieldKind::Password,
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
