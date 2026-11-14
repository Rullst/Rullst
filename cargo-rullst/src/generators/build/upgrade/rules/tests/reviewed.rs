//! The `rullst-upgrade: reviewed` marker, `V13-CLI-GREEN-RENAMED` and the
//! rules whose context guards read code only (13.0.0-alpha.5).

use super::super::files::project_findings;
use super::super::reviewed::{covers, first_unreviewed, marked_codes};
use super::files::Project;
use super::*;

const GREEN: &str = "V13-CLI-GREEN-RENAMED";

/// A package plan for the manifest `text`, written to `root`.
fn package(root: &Path, text: &str) -> ManifestUpgradePlan {
    std::fs::write(root.join("Cargo.toml"), text).unwrap();
    ManifestUpgradePlan {
        path: root.join("Cargo.toml"),
        original: text.to_string(),
        updated: text.to_string(),
        is_package: true,
        matched: 1,
        source_majors: BTreeSet::from([12]),
        source_versions: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    }
}

/// Codes and lines of a workspace scan of `root`, and the reviewed codes.
fn scan_project(
    root: &Path,
    manifest: &str,
) -> (Vec<(String, &'static str, usize)>, Vec<&'static str>) {
    let plans = [package(root, manifest)];
    let scan = scan_workspace(root, &plans, TARGET_MAJOR).unwrap();
    let findings = scan
        .findings
        .iter()
        .map(|finding| {
            let relative = finding.path.strip_prefix(root).unwrap();
            (
                relative.to_string_lossy().replace('\\', "/"),
                finding.rule.code,
                finding.line,
            )
        })
        .collect();
    let mut reviewed: Vec<&str> = scan.reviewed.iter().map(|rule| rule.code).collect();
    reviewed.sort_unstable();
    (findings, reviewed)
}

const MANIFEST: &str = "[package]\nname = \"demo\"\n";

#[test]
fn a_marker_names_codes_until_the_first_other_word() {
    assert_eq!(
        marked_codes("    // rullst-upgrade: reviewed V13-PAGE-BOUNDS"),
        ["V13-PAGE-BOUNDS"]
    );
    assert_eq!(
        marked_codes("# rullst-upgrade: reviewed V13-A, V13-B: kept on purpose V13-C"),
        ["V13-A", "V13-B"]
    );
    assert_eq!(
        marked_codes("<!-- rullst-upgrade: reviewed V13-CLI-GREEN-RENAMED-->"),
        [GREEN]
    );
    for line in [
        "// rullst-upgrade: reviewed",
        "// rullst-upgrade: reviewed v13-page-bounds",
        "// rullst-upgrade reviewed V13-PAGE-BOUNDS",
        "// V13-PAGE-BOUNDS",
        "// rullst-upgrade: reviewed V13-",
        "// rullst-upgrade: reviewed X13-PAGE-BOUNDS",
        "// rullst-upgrade: reviewed V13-Page",
    ] {
        assert!(marked_codes(line).is_empty(), "{line}");
    }
}

#[test]
fn a_marker_covers_its_own_line_or_the_line_below_when_alone() {
    let lines = [
        "fn a() {",
        "    // rullst-upgrade: reviewed V13-PAGE-BOUNDS",
        "    x.paginate(p, 1);",
        "    y.paginate(p, 1); // rullst-upgrade: reviewed V13-PAGE-BOUNDS",
        "    z.paginate(p, 1);",
        "}",
    ];
    let rule = &catalog::PAGE_BOUNDS;
    let covered: Vec<usize> = (0..=7).filter(|line| covers(&lines, *line, rule)).collect();
    assert_eq!(covered, [2, 3, 4]);
    // A must-change rule ignores the marker.
    let must = [
        "// rullst-upgrade: reviewed V13-ORM-FROM-ROW",
        "#[derive(FromRow, Orm)]",
    ];
    assert!(!covers(&must, 2, &catalog::ORM_FROM_ROW));
    assert!(!covers(&must, 1, &catalog::ORM_FROM_ROW));
    // Another rule's marker does not cover this one.
    assert!(!covers(
        &["x.paginate(p, 1); // rullst-upgrade: reviewed V13-MODEL-ALL"],
        1,
        rule
    ));

    let mut reviewed = Vec::new();
    let paginates = |line: &str| line.contains("paginate");
    assert_eq!(
        first_unreviewed(&lines, rule, paginates, &mut reviewed),
        Some(5)
    );
    assert_eq!(reviewed, [rule, rule]);
    let mut reviewed = Vec::new();
    assert_eq!(
        first_unreviewed(&lines[..4], rule, paginates, &mut reviewed),
        None
    );
    assert_eq!(reviewed.len(), 2);
}

#[test]
fn reviewed_rust_locations_leave_the_plan_and_the_next_one_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/posts.rs"),
        "async fn a() { Doc::query().paginate(page, 20).await; } // rullst-upgrade: reviewed V13-PAGE-BOUNDS\n\
async fn b() {\n\
    // rullst-upgrade: reviewed V13-PAGE-BOUNDS: bounded by the extractor\n\
    Doc::query().paginate(page, 20).await;\n\
}\n\
async fn c() { Doc::query().paginate(page, 20).await; }\n\
fn v() -> Markup { html! { <a onclick={x}></a> } } // rullst-upgrade: reviewed V13-HTML-DYNAMIC-EVENT-HANDLER\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "// rullst-upgrade: reviewed V13-HEALTH-PROBES\nfn main() { let server = Server::new(); }\n",
    )
    .unwrap();
    let (findings, reviewed) = scan_project(root, MANIFEST);
    assert_eq!(
        findings,
        [
            ("src/posts.rs".to_string(), "V13-PAGE-BOUNDS", 6),
            (
                "src/posts.rs".to_string(),
                "V13-HTML-DYNAMIC-EVENT-HANDLER",
                7
            ),
        ]
    );
    assert_eq!(
        reviewed,
        ["V13-HEALTH-PROBES", "V13-PAGE-BOUNDS", "V13-PAGE-BOUNDS"]
    );

    // Without the marker the start-up is reported.
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() { let server = Server::new(); }\n",
    )
    .unwrap();
    let (findings, _) = scan_project(root, MANIFEST);
    assert!(findings.contains(&("src/main.rs".to_string(), "V13-HEALTH-PROBES", 1)));
}

#[test]
fn reviewed_script_manifest_and_project_file_lines_leave_the_plan() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir_all(root.join(".github/workflows")).unwrap();
    std::fs::write(
        root.join(".github/workflows/ci.yml"),
        "steps:\n  # rullst-upgrade: reviewed V13-CLI-MIGRATION-AUTO-STATUS\n  - run: cargo rullst make:migration:auto\n  - run: cargo rullst make:migration:auto --name x\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Dockerfile"),
        "FROM rust\nRUN cargo build --release # rullst-upgrade: reviewed V13-DOCKER-UNLOCKED-BUILD\n",
    )
    .unwrap();
    let manifest = "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = \"12.1\" # rullst-upgrade: reviewed V13-RULLST-DEFAULT-FEATURES\n";
    let (findings, reviewed) = scan_project(root, manifest);
    assert_eq!(
        findings,
        [(
            ".github/workflows/ci.yml".to_string(),
            "V13-CLI-MIGRATION-AUTO-STATUS",
            4
        )]
    );
    assert_eq!(
        reviewed,
        [
            "V13-CLI-MIGRATION-AUTO-STATUS",
            "V13-DOCKER-UNLOCKED-BUILD",
            "V13-RULLST-DEFAULT-FEATURES"
        ]
    );
    // Unmarked, the same files are reported.
    std::fs::write(
        root.join("Dockerfile"),
        "FROM rust\nRUN cargo build --release\n",
    )
    .unwrap();
    let (findings, _) = scan_project(
        root,
        "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = \"12.1\"\n",
    );
    for expected in [
        ("Dockerfile".to_string(), "V13-DOCKER-UNLOCKED-BUILD", 2),
        ("Cargo.toml".to_string(), "V13-RULLST-DEFAULT-FEATURES", 5),
    ] {
        assert!(findings.contains(&expected), "{findings:?}");
    }
}

#[test]
fn scripts_ci_container_files_and_docs_running_green_are_reviewed() {
    let project = Project::new();
    project
        .write(".github/workflows/footprint.yml", "steps:\n  - run: cargo rullst green --bench --json\n")
        .write("Dockerfile", "FROM app\nRUN cargo rullst green --path /health\n")
        .write("Makefile", "measure:\n\trullst green --json | jq -e '.schema == \"rullst.green.v1\"'\n")
        .write(
            "docs/measuring.md",
            "# Measuring\n<!-- rullst-upgrade: reviewed V13-CLI-GREEN-RENAMED -->\nWe ran `cargo rullst green` in 2026.\n\nRun `cargo rullst green --bench`.\n",
        )
        .write("CHANGELOG.md", "- measured with cargo rullst green <!-- rullst-upgrade: reviewed V13-CLI-GREEN-RENAMED -->\n")
        .write("scripts/measure.py", "os.system('cargo rullst green --json')\n")
        .write("a/b/c/deep.md", "cargo rullst green\n")
        .write("a/b/c/d/deeper.md", "cargo rullst green\n")
        .write("src/main.rs", "// cargo rullst green\n")
        .write("target/ci.yml", "cargo rullst green\n")
        .write("node_modules/x/README.md", "cargo rullst green\n")
        .write("README.md", "Run `cargo rullst footprint` (green software practices).\n");
    let mut found = project.findings(MANIFEST);
    found.retain(|(_, code, _)| *code == GREEN);
    found.sort();
    let expected: Vec<(String, &str, usize)> = [
        (".github/workflows/footprint.yml", 2),
        ("Dockerfile", 2),
        ("Makefile", 2),
        ("a/b/c/deep.md", 1),
        ("docs/measuring.md", 5),
        ("scripts/measure.py", 1),
    ]
    .into_iter()
    .map(|(file, line)| (file.to_string(), GREEN, line))
    .collect();
    assert_eq!(found, expected);
    let plans = vec![package(project.root(), MANIFEST)];
    let (_, reviewed) = project_findings(project.root(), &plans);
    let reviewed: Vec<&str> = reviewed.iter().map(|rule| rule.code).collect();
    assert_eq!(reviewed, [GREEN, GREEN]);
    let rule = CATALOG.iter().find(|rule| rule.code == GREEN).unwrap();
    assert_eq!(rule.kind, FindingKind::Review);
    assert_eq!(rule.introduced, "13.0.0-alpha.5");
    assert!(rule.guidance.contains("cargo rullst footprint"));
    assert!(rule.guidance.contains("rullst.footprint.v1"));
    assert!(
        rule.guidance
            .contains("cargo install cargo-rullst --version")
    );
}

#[test]
fn comments_and_strings_never_decide_the_context_guards() {
    let has = |source: &str, code: &str| codes(source).contains(&code);
    let media = "fn k() -> FieldKind { FieldKind::Url }";
    for unrelated in [
        "// captions_url",
        "/// captions_url\nfn d() {}",
        "const Q: &str = \"SELECT captions_url FROM lessons\";",
        "#[serde(rename = \"captions_url\")] struct S;",
        "fn f() { let captions_url_x = 1; }",
    ] {
        assert!(
            !has(&format!("{media}\n{unrelated}"), "V13-LMS-MEDIA-KINDS"),
            "{unrelated}"
        );
    }
    for related in [
        "struct Lesson { captions_url: String }",
        "const NAME: &str = \"captions_url\";",
    ] {
        assert!(
            has(&format!("{media}\n{related}"), "V13-LMS-MEDIA-KINDS"),
            "{related}"
        );
    }

    let paging = "async fn p() { Doc::query().paginate(page, 20).await; }";
    for unrelated in [
        "// MAX_PAGE is checked by the caller",
        "/// MAX_PAGE\nfn d() {}",
        "const NOTE: &str = \"MAX_PAGE\";",
    ] {
        assert!(
            has(&format!("{paging}\n{unrelated}"), "V13-PAGE-BOUNDS"),
            "{unrelated}"
        );
    }
    assert!(!has(
        &format!("const MAX_PAGE_SIZE: u64 = 9; {paging}"),
        "V13-PAGE-BOUNDS"
    ));

    let transaction = "async fn t() { Orm::transaction(|_| Box::pin(async { Ok(()) })).await; }";
    for unrelated in [
        "// DatabaseError is retried",
        "const E: &str = \"DatabaseError\";",
        "#[doc = \"DatabaseError\"] fn d() {}",
    ] {
        assert!(
            !has(
                &format!("{transaction}\n{unrelated}"),
                "V13-ORM-TRANSACTION-ERRORS"
            ),
            "{unrelated}"
        );
    }
    assert!(has(
        &format!(
            "{transaction}\nfn e(x: Error) -> bool {{ matches!(x, Error::DatabaseError(_)) }}"
        ),
        "V13-ORM-TRANSACTION-ERRORS"
    ));
}
