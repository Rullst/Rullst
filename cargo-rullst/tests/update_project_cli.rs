//! Preparation uses real Git/Cargo metadata, but must never compile the project.
#[path = "update_project_cli/application.rs"]
mod application;
#[path = "update_project_cli/migration.rs"]
mod migration;
#[path = "update_project_cli/review.rs"]
mod review;
#[path = "update_project_cli/verification.rs"]
mod verification;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    app: PathBuf,
}

impl Fixture {
    fn current() -> Self {
        Self::new(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_VERSION"))
    }

    fn new(requirement: &str, version: &str) -> Self {
        #[cfg(windows)]
        let temp = tempfile::tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
        #[cfg(not(windows))]
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let app = base.join("app");
        fs::create_dir_all(app.join("src")).unwrap();
        fs::create_dir_all(app.join("vendor/framework/src")).unwrap();
        fs::write(app.join("Cargo.toml"), format!(
            "[package]\nname = \"prepared-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst-core = {{ version = \"{requirement}\", path = \"vendor/framework\" }} # retain this comment\n"
        )).unwrap();
        fs::write(
            app.join("vendor/framework/Cargo.toml"),
            format!(
                "[package]\nname = \"rullst-core\"\nversion = \"{version}\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        fs::write(
            app.join("vendor/framework/src/lib.rs"),
            "pub fn marker() {}\n",
        )
        .unwrap();
        fs::write(app.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            app.join("build.rs"),
            "compile_error!(\"preparation must not execute build scripts\");\n",
        )
        .unwrap();
        fs::write(app.join(".gitignore"), ".env\nCargo.lock\ntarget/\n").unwrap();
        fs::write(app.join("deleted.txt"), "original\n").unwrap();
        let fixture = Self {
            _temp: temp,
            base,
            app,
        };
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["-c", "core.autocrlf=false", "add", "."]);
        let output = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(&fixture.app)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", text(&output));
        fixture
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.app)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", text(&output));
    }

    fn prepare(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
            .args(["update", "project", "prepare", "--project"])
            .arg(&self.app)
            .arg("--json")
            .env("XDG_CACHE_HOME", &self.base)
            .env("LOCALAPPDATA", &self.base)
            .env("CARGO_NET_OFFLINE", "true")
            .env("RULLST_DISABLE_UPDATE_CHECK", "true")
            .output()
            .unwrap()
    }

    fn assert_clean_failure(&self, output: &Output, reason: &str) {
        assert!(
            !output.status.success(),
            "unexpected success: {}",
            text(output)
        );
        assert!(output.stdout.is_empty());
        assert!(text(output).contains(reason), "{}", text(output));
        let cache = self.base.join("rullst-update-v1");
        if cache.exists() {
            assert_eq!(
                fs::read_dir(cache).unwrap().count(),
                0,
                "failed staging must be removed"
            );
        }
        assert!(!self.app.join("target").exists());
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn preserves_dirty_untracked_deleted_and_ignored_lock_inputs_without_running_builds() {
    let fixture = Fixture::current();
    let dirty = b"// uncommitted user work\nfn main() {}\n";
    fs::write(fixture.app.join("src/main.rs"), dirty).unwrap();
    fs::write(fixture.app.join("src/untracked.rs"), "// not indexed\n").unwrap();
    fs::remove_file(fixture.app.join("deleted.txt")).unwrap();
    fs::write(fixture.app.join(".env"), "DO_NOT_COPY=secret\n").unwrap();
    let manifest = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let index = fs::read(fixture.app.join(".git/index")).unwrap();
    let lock = fs::read(fixture.app.join("Cargo.lock")).unwrap();
    let output = fixture.prepare();
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["schema_version"],
        "rullst.project-preparation-result.v1"
    );
    let stage = Path::new(report["prepared_directory"].as_str().unwrap());
    let candidate = stage.join("candidate");
    assert!(!stage.starts_with(&fixture.app));
    for directory in [stage.join("before"), candidate.clone()] {
        assert_eq!(fs::read(directory.join("src/main.rs")).unwrap(), dirty);
        assert_eq!(fs::read(directory.join("Cargo.lock")).unwrap(), lock);
        assert!(directory.join("src/untracked.rs").is_file());
        for excluded in ["deleted.txt", ".env", ".git", "target"] {
            assert!(!directory.join(excluded).exists(), "{excluded}");
        }
    }
    assert_eq!(fs::read(stage.join("before/Cargo.toml")).unwrap(), manifest);
    let edited = fs::read_to_string(candidate.join("Cargo.toml")).unwrap();
    assert!(edited.contains(&format!("version = \"={}\"", env!("CARGO_PKG_VERSION"))));
    assert!(edited.contains("# retain this comment"));
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), manifest);
    assert_eq!(fs::read(fixture.app.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(fixture.app.join("Cargo.lock")).unwrap(), lock);
    assert_eq!(fs::read(fixture.app.join("src/main.rs")).unwrap(), dirty);
    assert!(!fixture.app.join("target").exists());
    let prepared = &report["preparation"];
    assert_eq!(prepared["execution_authorized"], false);
    assert_eq!(prepared["application_authorized"], false);
    assert_eq!(
        prepared["plan"]["manifests"].as_array().unwrap().len(),
        1,
        "Cargo and private storage path spellings must identify one manifest"
    );
    assert!(
        prepared["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "deleted.txt" && file["sha256"].is_null())
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(stage.join("preparation.json")).unwrap())
            .unwrap(),
        *prepared
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(stage).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}

#[test]
fn source_findings_are_review_data_and_never_execution_authority() {
    let fixture = Fixture::new("5", "5.0.0");
    fs::write(
        fixture.app.join("src/main.rs"),
        "#[routes]\nfn main() { Server::new(); }\n",
    )
    .unwrap();
    let output = fixture.prepare();
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let prepared = &report["preparation"];
    assert_eq!(prepared["execution_authorized"], false);
    assert_eq!(prepared["application_authorized"], false);
    assert_eq!(prepared["plan"]["production_ready"], false);
    assert!(
        prepared["plan"]["source_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["code"] == "V5-ROUTES-ATTRIBUTE")
    );
    assert!(
        fs::read_to_string(fixture.app.join("src/main.rs"))
            .unwrap()
            .contains("#[routes]")
    );
}

#[test]
fn virtual_workspace_prepares_members_and_records_an_absent_lockfile() {
    let fixture = Fixture::current();
    fs::create_dir_all(fixture.app.join("member/src")).unwrap();
    let manifest = fs::read_to_string(fixture.app.join("Cargo.toml"))
        .unwrap()
        .replace(
            "path = \"vendor/framework\"",
            "path = \"../vendor/framework\"",
        );
    fs::write(fixture.app.join("member/Cargo.toml"), &manifest).unwrap();
    fs::write(fixture.app.join("member/src/main.rs"), "fn main() {}\n").unwrap();
    let root_manifest =
        "[workspace]\nmembers = [\"member\"]\nexclude = [\"vendor/framework\"]\nresolver = \"3\"\n";
    fs::write(fixture.app.join("Cargo.toml"), root_manifest).unwrap();
    fs::remove_file(fixture.app.join("Cargo.lock")).unwrap();
    let output = fixture.prepare();
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let candidate = Path::new(report["candidate_directory"].as_str().unwrap());
    assert!(
        fs::read_to_string(candidate.join("member/Cargo.toml"))
            .unwrap()
            .contains(&format!("version = \"={}\"", env!("CARGO_PKG_VERSION")))
    );
    assert_eq!(
        fs::read_to_string(candidate.join("Cargo.toml")).unwrap(),
        root_manifest
    );
    assert_eq!(
        fs::read_to_string(fixture.app.join("member/Cargo.toml")).unwrap(),
        manifest
    );
    assert!(!fixture.app.join("Cargo.lock").exists());
    assert!(
        report["preparation"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "Cargo.lock" && file["sha256"].is_null())
    );
}

#[test]
fn rejects_unknown_catalog_downgrades_and_unversioned_dependencies_without_retaining_staging() {
    let unknown = Fixture::new("7", "7.0.0");
    unknown.assert_clean_failure(&unknown.prepare(), "source majors 5, 6, 11, 12 and 13");
    let mut newer = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    newer.patch += 1;
    newer.pre = semver::Prerelease::EMPTY;
    let newer = newer.to_string();
    let future = Fixture::new(&newer, &newer);
    future.assert_clean_failure(
        &future.prepare(),
        "cannot downgrade a dependency requirement",
    );
    let locked = Fixture::new(env!("CARGO_PKG_VERSION"), &newer);
    locked.assert_clean_failure(
        &locked.prepare(),
        "cannot downgrade a locked Rullst package",
    );
    let unversioned = Fixture::current();
    let manifest = fs::read_to_string(unversioned.app.join("Cargo.toml"))
        .unwrap()
        .replace(
            &format!("version = \"{}\", ", env!("CARGO_PKG_VERSION")),
            "",
        );
    fs::write(unversioned.app.join("Cargo.toml"), &manifest).unwrap();
    unversioned.assert_clean_failure(&unversioned.prepare(), "require manual review");
    assert_eq!(
        fs::read_to_string(unversioned.app.join("Cargo.toml")).unwrap(),
        manifest
    );
}

#[test]
fn rejects_oversized_inputs_and_reports_instead_of_filling_storage() {
    let fixture = Fixture::current();
    fs::File::create(fixture.app.join("large.bin"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    fixture.assert_clean_failure(&fixture.prepare(), "bounded to 64 MiB");
    fs::remove_file(fixture.app.join("large.bin")).unwrap();
    fs::write(
        fixture.app.join("Cargo.toml"),
        fs::read_to_string(fixture.app.join("Cargo.toml"))
            .unwrap()
            .replace(
                &format!("version = \"{}\"", env!("CARGO_PKG_VERSION")),
                "version = \"5\"",
            ),
    )
    .unwrap();
    fs::write(
        fixture.app.join("src/main.rs"),
        "#[routes]\n".repeat(10_001),
    )
    .unwrap();
    fixture.assert_clean_failure(&fixture.prepare(), "findings exceed 10,000");
}

#[cfg(unix)]
#[test]
fn rejects_linked_and_special_inputs_without_following_or_blocking() {
    let fixture = Fixture::current();
    std::os::unix::fs::symlink(fixture.base.join("outside"), fixture.app.join("link")).unwrap();
    fixture.assert_clean_failure(&fixture.prepare(), "regular files");
    fs::remove_file(fixture.app.join("link")).unwrap();
    // Git omits untracked FIFOs. A replaced tracked input must still fail.
    fs::write(fixture.app.join("pipe"), "tracked before replacement").unwrap();
    fixture.git(&["add", "pipe"]);
    fs::remove_file(fixture.app.join("pipe")).unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(fixture.app.join("pipe"))
            .status()
            .unwrap()
            .success()
    );
    fixture.assert_clean_failure(&fixture.prepare(), "regular files");
}

#[cfg(windows)]
#[test]
fn rejects_a_junction_replacing_a_tracked_source_directory() {
    let fixture = Fixture::current();
    let outside = fixture.base.join("outside");
    fs::rename(fixture.app.join("src"), &outside).unwrap();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(fixture.app.join("src"))
        .arg(&outside)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    fixture.assert_clean_failure(&fixture.prepare(), "linked or non-directory ancestor");
    assert_eq!(
        fs::read_to_string(outside.join("main.rs")).unwrap(),
        "fn main() {}\n"
    );
    fs::remove_dir(fixture.app.join("src")).unwrap();
}
