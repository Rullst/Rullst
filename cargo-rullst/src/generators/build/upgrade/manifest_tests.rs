#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// `[dependencies.<name>]` tables are upgraded like inline tables: renamed
/// packages included, inherited ones left alone, and versionless or
/// non-string versions reported.
#[test]
fn dependency_tables_are_upgraded_like_inline_tables() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Cargo.toml");
    std::fs::write(
        &path,
        r#"[dependencies.rullst]
version = "12.3"
default-features = false

[dependencies.web]
package = "rullst-core"
version = "12.3"

[dependencies.rullst-auth]
workspace = true

[dependencies.rullst-mail]
path = "../mail"

[dependencies.rullst-ai]
version = 12
"#,
    )
    .unwrap();
    let plan = plan_manifest(path, "13.0.0").unwrap();
    assert_eq!(plan.matched, 5);
    let changes: Vec<_> = plan
        .changes
        .iter()
        .map(|change| {
            (
                change.key.as_str(),
                change.package.as_str(),
                change.from.as_str(),
                change.to.as_str(),
            )
        })
        .collect();
    assert_eq!(
        changes,
        [
            ("rullst", "rullst", "12.3", "=13.0.0"),
            ("web", "rullst-core", "12.3", "=13.0.0"),
        ]
    );
    assert_eq!(
        plan.warnings,
        [
            "dependency `rullst-mail` resolves package `rullst-mail` without a version; path/git dependencies require manual review",
            "dependency `rullst-ai` has a non-string version",
        ]
    );
    assert!(
        plan.updated.starts_with(
            "[dependencies.rullst]\nversion = \"=13.0.0\"\ndefault-features = false\n"
        ),
        "{}",
        plan.updated
    );
}
