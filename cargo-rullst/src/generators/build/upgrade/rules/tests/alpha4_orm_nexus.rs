//! 13.0.0-alpha.4: `#[derive(Orm)]` implements `FromRow`, Rullst crates no
//! longer enable SQLx's macros, and the facade's `nexus` leaves out the AI
//! assistant.

use super::super::files::manifest_findings;
use super::*;

fn scan(source: &str, sqlx_macros: bool) -> Vec<(&'static str, usize)> {
    let facts = WorkspaceFacts {
        sqlx_macros,
        ..WorkspaceFacts::default()
    };
    scan_with(source, "models.rs", &facts)
}

fn manifest(text: &str) -> (Vec<(&'static str, usize)>, WorkspaceFacts) {
    let plan = ManifestUpgradePlan {
        path: PathBuf::from("Cargo.toml"),
        original: text.to_string(),
        updated: text.to_string(),
        is_package: true,
        matched: 1,
        source_majors: BTreeSet::from([13]),
        source_versions: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    };
    let (findings, facts) = manifest_findings(&[plan]);
    let found = findings
        .iter()
        .map(|finding| (finding.rule.code, finding.line))
        .collect();
    (found, facts)
}

#[test]
fn models_deriving_from_row_beside_orm_must_change() {
    let source = r#"
use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "posts")]
pub struct Post { pub id: i32 }

#[derive(rullst::db::Orm, sqlx::FromRow)]
pub struct Tag { pub id: i32 }

#[derive(Orm)]
#[derive(rullst_orm::FromRow)]
pub struct Note { pub id: i32 }

#[derive(Debug, Orm)]
pub struct Clean { pub id: i32 }
"#;
    for sqlx_macros in [false, true] {
        assert_eq!(
            scan(source, sqlx_macros),
            [
                ("V13-ORM-FROM-ROW", 4),
                ("V13-ORM-FROM-ROW", 8),
                ("V13-ORM-FROM-ROW", 12)
            ]
        );
    }
}

#[test]
fn custom_from_row_models_and_plain_structs_need_the_sqlx_derive_feature() {
    let source = r#"
#[derive(Orm, sqlx::FromRow)]
#[orm(table = "users", custom_from_row)]
pub struct User { pub id: i32 }

#[derive(Debug, sqlx::FromRow)]
struct Invoice { id: i64 }

#[derive(Clone, sqlx::Type)]
enum Status { Active }

// Not SQLx's derive: an application's own `Type` derive.
#[derive(Type)]
enum Kind { A }
"#;
    assert_eq!(
        scan(source, false),
        [
            ("V13-SQLX-MACROS", 2),
            ("V13-SQLX-MACROS", 6),
            ("V13-SQLX-MACROS", 9)
        ]
    );
    // A manifest that enables SQLx's macros keeps them compiling.
    assert_eq!(scan(source, true), []);
}

#[test]
fn sqlx_query_macros_need_the_sqlx_macros_feature() {
    let source = r#"
async fn load(pool: &SqlitePool) {
    let row = sqlx::query!("SELECT id FROM users").fetch_one(pool).await;
    let typed = rullst::db::sqlx::query_as!(User, "SELECT id FROM users");
    sqlx::migrate!("./migrations");
    let plain = rullst::db::sqlx::query("SELECT 1");
    let other = query!("not SQLx's");
    let text = "sqlx::query!(...)";
}
"#;
    assert_eq!(
        scan(source, false),
        [
            ("V13-SQLX-MACROS", 3),
            ("V13-SQLX-MACROS", 4),
            ("V13-SQLX-MACROS", 5)
        ]
    );
    assert_eq!(scan(source, true), []);
}

#[test]
fn manifests_record_whether_they_enable_sqlx_macros() {
    for (text, enabled) in [
        ("[dependencies]\nsqlx = \"0.9\"\n", true),
        (
            "[dependencies]\nsqlx = { version = \"0.9\", features = [\"sqlite\"] }\n",
            true,
        ),
        (
            "[dependencies]\nsqlx = { version = \"0.9\", default-features = false, features = [\"derive\"] }\n",
            true,
        ),
        (
            "[dev-dependencies]\nsqlx = { version = \"0.9\", default-features = false, features = [\"macros\"] }\n",
            true,
        ),
        (
            "[dependencies]\nsqlx = { version = \"0.9\", default-features = false, features = [\"runtime-tokio\", \"tls-rustls\", \"sqlite\"] }\n",
            false,
        ),
        (
            "[dependencies]\nrullst = { version = \"13\", default-features = false }\n",
            false,
        ),
    ] {
        let (_, facts) = manifest(text);
        assert_eq!(facts.sqlx_macros, enabled, "{text}");
    }
}

#[test]
fn nexus_dependencies_without_the_assistant_are_reviewed() {
    for (text, line) in [
        (
            "[dependencies]\nrullst = { version = \"13\", default-features = false, features = [\"strict-sqlite\", \"nexus\"] }\n",
            2,
        ),
        (
            "[package]\nname = \"app\"\n\n[dependencies]\nrullst = { version = \"13\", default-features = false, features = [\"orm\", \"nexus-tz\"] }\n",
            5,
        ),
        (
            "[dependencies]\nrullst-nexus = { version = \"13\", default-features = false, features = [\"strict-sqlite\"] }\n",
            2,
        ),
    ] {
        assert_eq!(manifest(text).0, [("V13-NEXUS-AI-FEATURE", line)], "{text}");
    }
    for text in [
        "[dependencies]\nrullst = { version = \"13\", default-features = false, features = [\"nexus\", \"nexus-ai\", \"strict-sqlite\"] }\n",
        "[dependencies]\nrullst = { version = \"13\", default-features = false, features = [\"orm\", \"strict-sqlite\"] }\n",
        "[dependencies]\nrullst-nexus = \"13\"\n",
        "[dependencies]\nrullst-nexus = { version = \"13\", default-features = false, features = [\"ai\", \"strict-sqlite\"] }\n",
    ] {
        assert_eq!(manifest(text).0, [], "{text}");
    }
}
