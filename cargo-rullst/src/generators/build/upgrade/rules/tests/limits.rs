//! The scan's names, counts, file selection and bounds.

use super::*;

fn package_plan(root: &Path) -> ManifestUpgradePlan {
    ManifestUpgradePlan {
        path: root.join("Cargo.toml"),
        original: "[package]\nname = \"demo\"\n".to_string(),
        updated: "[package]\nname = \"demo\"\n".to_string(),
        is_package: true,
        matched: 1,
        source_majors: BTreeSet::from([12]),
        source_versions: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    }
}

#[test]
fn finding_kinds_have_fixed_names_and_are_counted_apart() {
    assert_eq!(FindingKind::MustChange.as_str(), "must-change");
    assert_eq!(FindingKind::Review.as_str(), "review");
    assert_eq!(FindingKind::MustChange.label(), "MUST-CHANGE");
    assert_eq!(FindingKind::Review.label(), "REVIEW");
    let rule = |kind| {
        CATALOG
            .iter()
            .copied()
            .find(|rule| rule.kind == kind)
            .unwrap()
    };
    let finding = |kind, line| SourceFinding {
        rule: rule(kind),
        path: PathBuf::from("src/main.rs"),
        line,
    };
    let scan = SourceScan {
        findings: vec![
            finding(FindingKind::MustChange, 1),
            finding(FindingKind::Review, 2),
            finding(FindingKind::MustChange, 3),
        ],
        ..SourceScan::default()
    };
    assert_eq!(scan.count(FindingKind::MustChange), 2);
    assert_eq!(scan.count(FindingKind::Review), 1);
}

/// Only regular `.rs` files are read, and those of at most 2 MiB; a larger
/// one is listed as unscanned.
#[test]
fn rust_files_up_to_2_mib_are_scanned() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir_all(root.join("src/folder.rs")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("src/notes.txt"), "not Rust {\n").unwrap();
    let comment = |bytes: u64| format!("//{}\n", "x".repeat(bytes as usize - 3));
    // Literal sizes, so a changed bound cannot move the fixture with it.
    std::fs::write(root.join("src/large.rs"), comment(2 * 1024 * 1024)).unwrap();
    std::fs::write(root.join("src/larger.rs"), comment(2 * 1024 * 1024 + 1)).unwrap();
    let mut scan = SourceScan::default();
    scan_package(root, &WorkspaceFacts::default(), &mut scan).unwrap();
    assert_eq!(scan.unscanned, vec![root.join("src/larger.rs")]);
    assert!(scan.findings.is_empty(), "{:?}", scan.findings);
}

/// A workspace may hold 10,000 findings; one more stops the scan.
#[test]
fn a_workspace_may_hold_at_most_10_000_findings() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    let plans = [package_plan(root)];
    let handlers = |count: usize| {
        (0..count)
            .map(|index| {
                format!("fn view{index}() -> Markup {{ html! {{ <a onclick={{x}}></a> }} }}\n")
            })
            .collect::<String>()
    };
    std::fs::write(root.join("src/views.rs"), handlers(MAX_FINDINGS)).unwrap();
    let scan = scan_workspace(root, &plans, TARGET_MAJOR).unwrap();
    assert_eq!(scan.findings.len(), MAX_FINDINGS);
    std::fs::write(root.join("src/views.rs"), handlers(MAX_FINDINGS + 1)).unwrap();
    assert_eq!(
        scan_workspace(root, &plans, TARGET_MAJOR)
            .unwrap_err()
            .to_string(),
        "migration findings exceed 10,000 entries; review a smaller project first"
    );
}
