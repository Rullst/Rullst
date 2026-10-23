#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::path::PathBuf;

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(lock: Option<&str>) -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-current-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        if let Some(lock) = lock {
            std::fs::write(root.join("Cargo.lock"), lock).unwrap();
        }
        Self { root }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn lock(packages: &[(&str, &str)]) -> String {
    let mut text = String::from("version = 4\n");
    for (name, version) in packages {
        text.push_str(&format!(
            "\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"
        ));
    }
    text
}

fn plan(versions: &[&str]) -> ManifestUpgradePlan {
    ManifestUpgradePlan {
        path: PathBuf::from("Cargo.toml"),
        original: String::new(),
        updated: String::new(),
        is_package: true,
        matched: versions.len(),
        source_majors: Default::default(),
        source_versions: versions
            .iter()
            .map(|version| Version::parse(version).unwrap())
            .collect(),
        changes: Vec::new(),
        warnings: Vec::new(),
    }
}

fn current(project: &Project, plans: &[ManifestUpgradePlan]) -> Option<String> {
    application_version(&project.root, plans)
        .unwrap()
        .map(|version| version.to_string())
}

#[test]
fn the_oldest_required_or_locked_version_is_the_application_version() {
    let locked = Project::new(Some(&lock(&[
        ("serde", "1.0.0"),
        ("rullst", "13.0.0-alpha.2"),
        ("rullst-orm", "13.0.0-alpha.2"),
    ])));
    // The manifests ask for alpha.2 or later; the lockfile holds alpha.2.
    assert_eq!(
        current(
            &locked,
            &[plan(&["13.0.0-alpha.2"]), plan(&["13.0.0-alpha.2"])]
        ),
        Some("13.0.0-alpha.2".to_string())
    );
    // A caret requirement on alpha.1 admits the locked alpha.2: the older wins.
    assert_eq!(
        current(&locked, &[plan(&["13.0.0-alpha.2", "13.0.0-alpha.1"])]),
        Some("13.0.0-alpha.1".to_string())
    );
    assert_eq!(current(&locked, &[]), Some("13.0.0-alpha.2".to_string()));

    let older_lock = Project::new(Some(&lock(&[("rullst-core", "12.3.0")])));
    assert_eq!(
        current(&older_lock, &[plan(&["13.0.0-alpha.2"])]),
        Some("12.3.0".to_string())
    );
}

#[test]
fn without_a_lockfile_the_requirements_decide_and_nothing_is_unknown() {
    let project = Project::new(None);
    assert_eq!(
        current(&project, &[plan(&["13.0.0-alpha.3", "13.0.0-alpha.1"])]),
        Some("13.0.0-alpha.1".to_string())
    );
    assert_eq!(current(&project, &[plan(&[])]), None);
}

#[test]
fn the_lockfile_lists_rullst_packages_and_finds_newer_ones() {
    let project = Project::new(Some(&lock(&[
        ("rullst-orm", "13.0.0-alpha.3"),
        ("rullst-unrelated", "99.0.0"),
        ("rullst", "13.0.0-alpha.2"),
    ])));
    let locked = super::super::isolated::locked_versions(&project.root).unwrap();
    assert_eq!(
        locked
            .iter()
            .map(|(name, version)| format!("{name} {version}"))
            .collect::<Vec<_>>(),
        ["rullst-orm 13.0.0-alpha.3", "rullst 13.0.0-alpha.2"]
    );
    let above = |target: &str| {
        super::super::isolated::locked_above(&project.root, &Version::parse(target).unwrap())
            .unwrap()
            .map(|(name, version)| format!("{name} {version}"))
    };
    assert_eq!(
        above("13.0.0-alpha.2"),
        Some("rullst-orm 13.0.0-alpha.3".to_string())
    );
    assert_eq!(above("13.0.0-alpha.3"), None);
    assert!(
        super::super::isolated::locked_versions(&Project::new(None).root)
            .unwrap()
            .is_empty()
    );
    let directory = Project::new(None);
    std::fs::create_dir(directory.root.join("Cargo.lock")).unwrap();
    assert!(application_version(&directory.root, &[]).is_err());
}

/// Project preparations gate execution on must-change findings, so they keep
/// every rule even for an application already on a 13.0 prerelease.
#[test]
fn project_preparations_apply_every_rule() {
    let project = Project::new(None);
    std::fs::write(
        project.root.join("Cargo.toml"),
        "[package]\nname = \"prepared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"=13.0.0-alpha.2\", default-features = false }\n",
    )
    .unwrap();
    std::fs::write(
        project.root.join("src/main.rs"),
        "fn main() {}\nfn page(htmx: &Htmx) -> String { render_page(htmx, \"Home\", String::new()) }\n",
    )
    .unwrap();
    let report = super::super::prepare_manifests(
        &project.root,
        vec![project.root.join("Cargo.toml")],
        "13.0.0-alpha.2",
    )
    .unwrap();
    assert_eq!(
        report["rule_selection"],
        serde_json::json!({
            "current_version": null,
            "all_rules": true,
            "skipped_rules": 0,
            "hidden_findings": 0,
        })
    );
    let page = report["source_findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["code"] == "V13-RENDER-PAGE-LANGUAGE")
        .unwrap_or_else(|| panic!("{report}"));
    assert_eq!(page["introduced_in"], "13.0.0-alpha.1");
}
