use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
