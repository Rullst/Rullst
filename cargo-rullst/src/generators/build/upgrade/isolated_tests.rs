#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// A package depending on `rullst = "<requirement>"`, with an optional
/// lockfile that pins `rullst` at `locked`.
fn project(requirement: &str, locked: Option<&str>) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"prepared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"{requirement}\"\n"
        ),
    )
    .unwrap();
    if let Some(locked) = locked {
        std::fs::write(
            root.join("Cargo.lock"),
            format!("version = 4\n\n[[package]]\nname = \"rullst\"\nversion = \"{locked}\"\n"),
        )
        .unwrap();
    }
    directory
}

fn plan(root: &Path, paths: Vec<PathBuf>) -> Result<serde_json::Value, String> {
    plan_manifests(root, paths, "13.0.0", false).map_err(|error| error.to_string())
}

fn plan_root(directory: &tempfile::TempDir) -> Result<serde_json::Value, String> {
    plan(directory.path(), vec![directory.path().join("Cargo.toml")])
}

#[test]
fn members_are_manifests_inside_the_prepared_project() {
    let directory = project("13.0", None);
    let outside = project("13.0", None);
    for path in [
        directory.path().join("src/main.rs"),
        outside.path().join("Cargo.toml"),
    ] {
        assert_eq!(
            plan(directory.path(), vec![path.clone()]).unwrap_err(),
            "workspace members must stay inside the prepared project",
            "{}",
            path.display()
        );
    }
    plan_root(&directory).unwrap();
}

/// A requirement whose floor equals the target is upgraded in place; one
/// above it would be a downgrade.
#[test]
fn requirements_may_reach_but_not_pass_the_target() {
    plan_root(&project("13.0", None)).unwrap();
    plan_root(&project("13.0.0", None)).unwrap();
    assert_eq!(
        plan_root(&project("13.0.1", None)).unwrap_err(),
        "project preparation cannot downgrade a dependency requirement"
    );
}

#[test]
fn a_lockfile_above_the_target_is_a_downgrade() {
    plan_root(&project("13.0", Some("13.0.0"))).unwrap();
    assert_eq!(
        plan_root(&project("13.0", Some("13.0.1"))).unwrap_err(),
        "project preparation cannot downgrade a locked Rullst package"
    );
}

#[test]
fn only_v12_and_v13_sources_are_prepared() {
    plan_root(&project("12.3", None)).unwrap();
    assert_eq!(
        plan_root(&project("11", None)).unwrap_err(),
        "the migration catalog covers source majors 12 and 13 only; upgrade older applications to v12 with the v12 CLI first"
    );
}
