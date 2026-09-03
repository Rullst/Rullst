use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{collections::BTreeSet, fs};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str, dependency_requirement: &str, fake_version: &str, source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rullst-upgrade-cli-{name}-{}",
            rand::random::<u64>()
        ));
        let app = root.join("app");
        let framework = root.join("framework");
        std::fs::create_dir_all(app.join("src")).expect("app directory");
        std::fs::create_dir_all(framework.join("src")).expect("framework directory");
        std::fs::write(
            framework.join("Cargo.toml"),
            format!(
                "[package]\nname = \"rullst\"\nversion = \"{fake_version}\"\nedition = \"2024\"\n"
            ),
        )
        .expect("framework manifest");
        std::fs::write(framework.join("src/lib.rs"), "pub fn marker() {}\n")
            .expect("framework source");
        std::fs::write(
            app.join("Cargo.toml"),
            format!(
                "[package]\nname = \"upgrade-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = {{ version = \"{dependency_requirement}\", path = \"../framework\" }}\n"
            ),
        )
        .expect("app manifest");
        std::fs::write(app.join("src/main.rs"), source).expect("app source");
        Self { root }
    }

    fn app(&self) -> PathBuf {
        self.root.join("app")
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_at(&self.app(), arguments)
    }

    fn run_at(&self, directory: &Path, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
            .args(arguments)
            .current_dir(directory)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .expect("upgrade command")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn contains_report(backup_root: &Path) -> bool {
    std::fs::read_dir(backup_root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            entry.path().join("report.md").is_file() && entry.path().join("report.json").is_file()
        })
}

fn first_backup(backup_root: &Path) -> PathBuf {
    std::fs::read_dir(backup_root)
        .expect("backup root")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join("report.md").is_file())
        .expect("upgrade backup")
}

#[test]
fn dry_run_reports_v5_markers_without_writing() {
    let fixture = Fixture::new(
        "dry-run",
        "5",
        "5.0.0",
        "#[routes]\nfn home() {}\nfn main() { Server::new(); }\n",
    );
    let manifest = std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest");

    let output = fixture.run(&["upgrade", "--dry-run"]);

    assert!(output.status.success(), "{}", output_text(&output));
    let text = output_text(&output);
    assert!(text.contains("DRY RUN"));
    assert!(text.contains("V5-ROUTES-ATTRIBUTE"));
    assert!(text.contains("V5-SERVER-CONSTRUCTOR"));
    assert_eq!(
        std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest after"),
        manifest
    );
    assert!(!fixture.app().join("Cargo.lock").exists());
}

#[test]
fn json_dry_run_is_versioned_and_machine_readable() {
    let fixture = Fixture::new(
        "json-dry-run",
        "5",
        "5.0.0",
        "#[routes]\nfn home() {}\nfn main() {}\n",
    );

    let output = fixture.run(&["upgrade", "--dry-run", "--json"]);

    assert!(output.status.success(), "{}", output_text(&output));
    let report = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .expect("stdout must contain only JSON");
    assert_eq!(report["schema_version"], "rullst.upgrade-plan.v1");
    assert_eq!(report["rule_catalog"], "rullst-upgrade-rules-v1");
    assert_eq!(report["production_ready"], false);
    assert_eq!(report["source_findings"][0]["code"], "V5-ROUTES-ATTRIBUTE");
}

#[test]
fn virtual_workspace_updates_only_cargo_metadata_members() {
    let root = std::env::temp_dir().join(format!(
        "rullst-upgrade-cli-workspace-{}",
        rand::random::<u64>()
    ));
    for directory in ["app/src", "framework/src", "excluded/src"] {
        std::fs::create_dir_all(root.join(directory)).expect("workspace directory");
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\nexclude = [\"framework\", \"excluded\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        root.join("framework/Cargo.toml"),
        "[package]\nname = \"rullst\"\nversion = \"5.0.0\"\nedition = \"2024\"\n",
    )
    .expect("framework manifest");
    std::fs::write(root.join("framework/src/lib.rs"), "pub fn marker() {}\n")
        .expect("framework source");
    std::fs::write(
        root.join("app/Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"5\", path = \"../framework\" }\n",
    )
    .expect("app manifest");
    std::fs::write(root.join("app/src/main.rs"), "fn main() {}\n").expect("app source");
    std::fs::write(
        root.join("excluded/Cargo.toml"),
        "[package]\nname = \"excluded\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"4\"\n",
    )
    .expect("excluded manifest");
    std::fs::write(root.join("excluded/src/main.rs"), "fn main() {}\n").expect("excluded source");
    let fixture = Fixture { root };

    let output = fixture.run_at(&fixture.root, &["upgrade", "--dry-run"]);

    assert!(output.status.success(), "{}", output_text(&output));
    let text = output_text(&output);
    assert!(text.contains("app/Cargo.toml"));
    assert!(!text.contains("excluded/Cargo.toml"));
    assert!(
        std::fs::read_to_string(fixture.root.join("excluded/Cargo.toml"))
            .expect("excluded manifest after")
            .contains("rullst = \"4\"")
    );
}

#[test]
fn successful_upgrade_changes_the_manifest_and_keeps_a_review_report() {
    let fixture = Fixture::new("success", ">=5, <13", "12.0.0", "fn main() {}\n");
    let original_manifest =
        std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest before");

    let output = fixture.run(&["upgrade"]);

    assert!(output.status.success(), "{}", output_text(&output));
    let manifest = std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest");
    assert!(manifest.contains("version = \"12.0.0\""));
    assert!(fixture.app().join("Cargo.lock").is_file());
    assert!(contains_report(
        &fixture.app().join("target/rullst-upgrades")
    ));

    let backup = first_backup(&fixture.app().join("target/rullst-upgrades"));
    std::fs::write(fixture.app().join("Cargo.toml"), "corrupted\n").expect("corrupt manifest");
    let restore = fixture.run(&[
        "upgrade",
        "--restore",
        backup.to_str().expect("UTF-8 backup path"),
    ]);
    assert!(restore.status.success(), "{}", output_text(&restore));
    assert_eq!(
        std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("restored manifest"),
        original_manifest
    );
    assert!(!fixture.app().join("Cargo.lock").exists());
}

#[test]
fn failed_upgrade_restores_manifest_source_and_absent_lockfile() {
    let fixture = Fixture::new("rollback", "5", "5.0.0", "fn main() {}\n");
    let original_manifest =
        std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest");
    let original_source =
        std::fs::read_to_string(fixture.app().join("src/main.rs")).expect("source");

    let output = fixture.run(&["upgrade"]);

    assert!(!output.status.success(), "upgrade unexpectedly succeeded");
    assert!(output_text(&output).contains("were restored"));
    assert_eq!(
        std::fs::read_to_string(fixture.app().join("Cargo.toml")).expect("manifest after"),
        original_manifest
    );
    assert_eq!(
        std::fs::read_to_string(fixture.app().join("src/main.rs")).expect("source after"),
        original_source
    );
    assert!(!fixture.app().join("Cargo.lock").exists());
    assert!(contains_report(
        &fixture.app().join("target/rullst-upgrades")
    ));
}

#[test]
fn source_rule_catalog_covers_every_documented_upgrade_origin() {
    let source = "#[routes]\nfn old() { Server::new(); app.run(); Nexus::build(); }\n// STUDIO_PASSWORD APP_ENV\n";
    let cases = [
        (
            "v5",
            "5",
            BTreeSet::from([
                "V5-ENV-ALIAS",
                "V5-NEXUS-POLICY",
                "V5-ROUTES-ATTRIBUTE",
                "V5-SERVER-CONSTRUCTOR",
                "V5-SERVER-PORT",
                "V5-STUDIO-BOUNDARY",
            ]),
        ),
        (
            "v6",
            "6",
            BTreeSet::from(["V5-ENV-ALIAS", "V5-NEXUS-POLICY", "V5-STUDIO-BOUNDARY"]),
        ),
        (
            "v11",
            "11",
            BTreeSet::from(["V5-ENV-ALIAS", "V5-NEXUS-POLICY", "V5-STUDIO-BOUNDARY"]),
        ),
        ("v12", "12", BTreeSet::new()),
    ];

    for (name, requirement, expected) in cases {
        let fixture = Fixture::new(name, requirement, "12.0.0", source);
        let output = fixture.run(&["upgrade", "--dry-run", "--json"]);
        assert!(output.status.success(), "{name}: {}", output_text(&output));
        let report = serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .unwrap_or_else(|error| panic!("{name}: invalid JSON report: {error}"));
        let actual = report["source_findings"]
            .as_array()
            .expect("source findings array")
            .iter()
            .filter_map(|finding| finding["code"].as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{name}: wrong source rule selection");
    }
}

#[test]
fn failed_workspace_upgrade_restores_every_member_atomically() {
    let root = std::env::temp_dir().join(format!(
        "rullst-upgrade-cli-multi-{}",
        rand::random::<u64>()
    ));
    for directory in ["app-a/src", "app-b/src", "framework/src"] {
        fs::create_dir_all(root.join(directory)).expect("workspace directory");
    }
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app-a\", \"app-b\"]\nexclude = [\"framework\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest");
    fs::write(
        root.join("framework/Cargo.toml"),
        "[package]\nname = \"rullst\"\nversion = \"5.0.0\"\nedition = \"2024\"\n",
    )
    .expect("framework manifest");
    fs::write(root.join("framework/src/lib.rs"), "pub fn marker() {}\n").expect("framework source");

    let mut originals = Vec::new();
    for member in ["app-a", "app-b"] {
        let manifest = format!(
            "[package]\nname = \"{member}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = {{ version = \"5\", path = \"../framework\" }}\n"
        );
        fs::write(root.join(member).join("Cargo.toml"), &manifest).expect("member manifest");
        fs::write(root.join(member).join("src/main.rs"), "fn main() {}\n").expect("member source");
        originals.push((member, manifest));
    }
    let fixture = Fixture { root };

    let output = fixture.run_at(&fixture.root, &["upgrade"]);

    assert!(!output.status.success(), "upgrade unexpectedly succeeded");
    assert!(output_text(&output).contains("were restored"));
    for (member, original) in originals {
        assert_eq!(
            fs::read_to_string(fixture.root.join(member).join("Cargo.toml"))
                .expect("restored member manifest"),
            original,
            "{member} was not restored atomically"
        );
    }
    assert!(contains_report(
        &fixture.root.join("target/rullst-upgrades")
    ));
}

#[test]
fn keep_on_failure_preserves_review_state_until_explicit_restore() {
    let fixture = Fixture::new("keep", "5", "5.0.0", "fn main() {}\n");
    let original_manifest =
        fs::read_to_string(fixture.app().join("Cargo.toml")).expect("original manifest");

    let output = fixture.run(&["upgrade", "--keep-on-failure"]);

    assert!(!output.status.success(), "upgrade unexpectedly succeeded");
    assert!(output_text(&output).contains("edited files were kept by request"));
    let edited = fs::read_to_string(fixture.app().join("Cargo.toml")).expect("edited manifest");
    assert!(edited.contains("version = \"12.0.0\""));

    let backup = first_backup(&fixture.app().join("target/rullst-upgrades"));
    let restore = fixture.run(&[
        "upgrade",
        "--restore",
        backup.to_str().expect("UTF-8 backup path"),
    ]);
    assert!(restore.status.success(), "{}", output_text(&restore));
    assert_eq!(
        fs::read_to_string(fixture.app().join("Cargo.toml")).expect("restored manifest"),
        original_manifest
    );
}

#[cfg(unix)]
#[test]
fn symlinked_rust_sources_fail_before_the_upgrade_transaction() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("source-symlink", "5", "12.0.0", "fn main() {}\n");
    let manifest_path = fixture.app().join("Cargo.toml");
    let original_manifest = fs::read_to_string(&manifest_path).expect("original manifest");
    let outside_source = fixture.root.join("outside.rs");
    fs::write(&outside_source, "#[routes]\nfn outside() {}\n").expect("outside source");
    fs::remove_file(fixture.app().join("src/main.rs")).expect("remove regular source");
    symlink(&outside_source, fixture.app().join("src/main.rs")).expect("source symlink");

    let output = fixture.run(&["upgrade", "--dry-run"]);

    assert!(!output.status.success(), "symlinked source was accepted");
    assert!(output_text(&output).contains("refusing to scan symlinked Rust source"));
    assert_eq!(
        fs::read_to_string(manifest_path).expect("manifest after rejection"),
        original_manifest
    );
    assert!(!fixture.app().join("target/rullst-upgrades").exists());
}
