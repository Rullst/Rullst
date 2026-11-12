//! 13.0.0-alpha.5 removed the MongoDB and DuckDB adapters
//! (`V13-ORM-ADAPTERS-REMOVED`): their names in Rust sources and their
//! features in Cargo manifests.

use super::super::files::manifest_findings;
use super::files::plan;
use super::*;

const CODE: &str = "V13-ORM-ADAPTERS-REMOVED";

fn lines_of(source: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| *code == CODE)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn removed_adapter_names_must_change_at_every_location() {
    let source = r#"
use rullst::orm::polyglot::{CollectionName, MongoDbStore};
use rullst_orm::polyglot::{AnalyticsRepository, DuckDbStore, QueryLimit};
use rullst::orm::polyglot::mongodb;
async fn f(error: PolyglotError) -> Result<(), PolyglotError> {
    let store = MongoDbStore::<Event>::connect_or_mock("", "app").await?;
    let analytics = DuckDbStore::open("analytics.duckdb")?;
    let rows: Vec<AnalyticsRow> = analytics.query("SELECT 1", QueryLimit::new(10)?).await?;
    match error {
        PolyglotError::Worker { .. } => {}
        _ => {}
    }
    assert_eq!(MongoDbStore::<Event>::capabilities().backend(), Backend::MongoDb);
    let _ = (Capability::Analytics, AnalyticsValue::Null, AnalyticsTimeUnit::Seconds);
    Ok(())
}
"#;
    assert_eq!(
        lines_of(source),
        vec![2, 3, 4, 6, 7, 8, 10, 13, 14],
        "{source}"
    );
    let rule = CATALOG.iter().find(|rule| rule.code == CODE).unwrap();
    assert_eq!(rule.kind, FindingKind::MustChange);
    assert_eq!(rule.introduced, "13.0.0-alpha.5");
    assert!(rule.guidance.contains("book/recipes/mongodb.html"));
    assert!(rule.guidance.contains("book/recipes/duckdb.html"));
}

#[test]
fn removed_module_paths_must_change_in_use_trees_and_expressions() {
    let source = r#"
use rullst::orm::{polyglot::{duckdb, CollectionName}, Orm};
fn f() {
    let _ = rullst_orm::polyglot::mongodb::MongoDbStore::<Event>::is_mock;
    let _ = rullst_orm::Backend::DuckDb;
}
"#;
    assert_eq!(lines_of(source), vec![2, 4, 5], "{source}");
}

#[test]
fn names_outside_the_polyglot_module_and_kept_items_are_not_reported() {
    // An application's own types with these names, in files that do not
    // name the ORM's `polyglot` module, and enums of its own.
    assert_eq!(
        lines_of(
            r#"
struct MongoDbStore { client: mongodb::Client }
struct QueryLimit(u32);
enum Backend { MongoDb, DuckDb }
fn f() -> (Backend, QueryLimit, AnalyticsRow) {
    let _ = Capability::Analytics;
    (Backend::MongoDb, QueryLimit(5), AnalyticsRow::default())
}
"#
        ),
        Vec::<usize>::new()
    );
    // What the removal keeps compiles unchanged.
    assert_eq!(
        lines_of(
            r#"
use rullst::orm::polyglot::{
    CollectionName, DocumentId, DocumentRepository, MockDocumentStore, SurrealDbStore,
    export_document_snapshot,
};
use rullst_orm::{Backend, Capability, PolyglotError};
fn f(error: PolyglotError) {
    let _ = (Backend::SurrealDb, Capability::Documents);
    let _ = matches!(error, PolyglotError::Driver { .. });
    let _ = rullst_orm::polyglot::SurrealDbStore::capabilities;
}
"#
        ),
        Vec::<usize>::new()
    );
    // A comment or a string that names them is not code.
    assert_eq!(
        lines_of(
            "use rullst::orm::polyglot::CollectionName;\n// MongoDbStore and Backend::MongoDb\nconst NOTE: &str = \"DuckDbStore\";\n"
        ),
        Vec::<usize>::new()
    );
}

#[test]
fn manifests_report_the_removed_adapter_features() {
    let manifest = r#"[package]
name = "app"

[dependencies]
rullst = { version = "13.0.0-alpha.4", default-features = false, features = ["strict-postgres", "orm-mongodb"] }
analytics = { package = "rullst-orm", version = "13.0.0-alpha.4", default-features = false, features = ["strict-sqlite", "duckdb"] }
rullst-orm = { version = "13.0.0-alpha.4", default-features = false, features = ["mongodb", "duckdb"] }
mongodb = "3"
duckdb = { version = "1", features = ["bundled"] }

[features]
reports = ["rullst/orm-duckdb"]
"#;
    let (findings, _) = manifest_findings(&[plan(PathBuf::from("Cargo.toml"), manifest)]);
    let mut found: Vec<_> = findings
        .iter()
        .map(|finding| (finding.rule.code, finding.line))
        .collect();
    found.sort_by_key(|(_, line)| *line);
    // The application's own `mongodb` and `duckdb` dependencies are fine.
    assert_eq!(found, vec![(CODE, 5), (CODE, 6), (CODE, 7), (CODE, 12)]);
    // The kept adapters are not reported.
    let kept = r#"[dependencies]
rullst = { version = "13.0.0-alpha.4", default-features = false, features = ["orm-surrealdb", "orm-turso"] }
rullst-orm = { version = "13.0.0-alpha.4", default-features = false, features = ["surrealdb", "turso", "redis"] }
"#;
    let (findings, _) = manifest_findings(&[plan(PathBuf::from("Cargo.toml"), kept)]);
    assert!(findings.is_empty(), "{findings:?}");
}
