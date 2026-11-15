use super::*;

const LOCK: &str = r#"
version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = ["helper", "serde"]

[[package]]
name = "helper"
version = "0.1.0"

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "serde_derive"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "windows-sys"
version = "0.61.2"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "forked"
version = "0.2.0"
source = "git+https://example.com/forked#0123abcd"

[[package]]
name = "rand"
version = "0.8.5"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "rand"
version = "0.9.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;

/// What `cargo tree --prefix none --format {p}` prints for that lock on a
/// platform without `windows-sys`: duplicates marked `(*)`, a proc macro, a
/// git source, path packages and two versions of one crate.
const TREE: &str = "app v0.1.0 (/work/app)
helper v0.1.0 (/work/app/helper)
serde v1.0.0
serde_derive v1.0.0 (proc-macro)
forked v0.2.0 (https://example.com/forked#0123abcd)
rand v0.8.5
rand v0.9.2
serde v1.0.0 (*)
helper v0.1.0 (/work/app/helper) (*)

";

#[test]
fn compiled_third_party_packages_are_counted_once() {
    let sourced = third_party(LOCK).unwrap();
    assert_eq!(sourced.len(), 6);
    assert!(sourced.contains(&("rand".to_string(), "0.9.2".to_string())));
    assert!(!sourced.contains(&("helper".to_string(), "0.1.0".to_string())));
    // serde, serde_derive, forked and both rand; not app, helper or the
    // windows-sys this platform never compiles.
    assert_eq!(count_compiled(TREE, &sourced), 5);
    assert_eq!(count_compiled("", &sourced), 0);
    // A version the lock does not record, or a line without one, is not a
    // compiled third-party package.
    assert_eq!(
        count_compiled("serde v9.9.9\nserde\nserde 1.0.0\n", &sourced),
        0
    );
    assert_eq!(third_party("not toml ["), None);
    assert_eq!(third_party("version = 4\n"), None);
    assert_eq!(third_party("package = []\n").map(|set| set.len()), Some(0));
}

#[test]
fn the_tree_arguments_select_compiled_units_without_network() {
    assert_eq!(
        TREE_ARGUMENTS.join(" "),
        "tree --locked --offline --edges normal,build --prefix none --format {p} --color never"
    );
    assert!(METHOD.contains("dev-dependencies excluded"));
}

/// A project directory with `files`.
fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    for (name, text) in files {
        let path = directory.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    directory
}

const MANIFEST: &str =
    "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n";

#[test]
fn a_directory_without_a_manifest_or_lock_is_not_measured() {
    let cargo = Path::new("cargo-that-must-not-run");
    let empty = project(&[("Cargo.lock", LOCK)]);
    let missing = count_with(cargo, empty.path(), Duration::from_secs(5));
    assert_eq!(missing.status, "not_measured");
    assert_eq!(missing.unit, "packages");
    assert_eq!(
        missing.reason.as_deref(),
        Some("no Cargo.toml in this directory; run footprint from the project directory")
    );
    let unparsable = project(&[("Cargo.toml", MANIFEST), ("Cargo.lock", "[[package]\n")]);
    assert_eq!(
        count_with(cargo, unparsable.path(), Duration::from_secs(5))
            .reason
            .as_deref(),
        Some("Cargo.lock is missing or could not be parsed")
    );
    let not_found = project(&[("Cargo.toml", MANIFEST), ("Cargo.lock", LOCK)]);
    assert_eq!(
        count_with(cargo, not_found.path(), Duration::from_secs(5))
            .reason
            .as_deref(),
        Some("the cargo command is not available")
    );
}

/// The real `cargo tree` of a project whose only dependency is a path
/// package: it runs offline and counts no third-party package.
#[test]
fn the_real_cargo_tree_counts_no_path_package() {
    let directory = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhelper = { path = \"helper\" }\n\n[workspace]\n",
        ),
        ("src/lib.rs", ""),
        (
            "helper/Cargo.toml",
            "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        ("helper/src/lib.rs", ""),
    ]);
    let generated = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(generated.status.success(), "{generated:?}");
    let counted = count_with(Path::new("cargo"), directory.path(), TREE_TIMEOUT);
    assert_eq!(counted.status, "measured", "{counted:?}");
    assert_eq!(counted.value, Some(0));
    assert_eq!(counted.method.as_deref(), Some(METHOD));
}

#[cfg(unix)]
mod fake_cargo {
    use super::*;
    use crate::generators::footprint::tests::support::{script, sleep_program};

    /// A `cargo` that records its arguments in `arguments` and prints
    /// `TREE`, fails or hangs as `FAKE_CARGO` says.
    const CARGO: &str = r#"printf '%s\n' "$*" > "$FAKE_CARGO_ARGUMENTS"
case "$FAKE_CARGO" in
  tree) printf '%s' "$FAKE_CARGO_TREE" ;;
  fail) printf 'serde v1.0.0\n'; exit 101 ;;
  hang) exec "$FAKE_SLEEP" 30 ;;
esac"#;

    fn run(mode: &str, files: &[(&str, &str)], timeout: Duration) -> (Metric<u64>, String) {
        let directory = project(files);
        let bin = tempfile::tempdir().unwrap();
        let cargo = script(bin.path(), "cargo", CARGO);
        let arguments = bin.path().join("arguments");
        // The variables reach the script through a wrapper, so this test
        // never changes its own process environment.
        let wrapper = script(
            bin.path(),
            "wrapper",
            &format!(
                "FAKE_CARGO={mode} FAKE_CARGO_ARGUMENTS='{}' FAKE_SLEEP='{}' FAKE_CARGO_TREE='{}' exec '{}' \"$@\"",
                arguments.display(),
                sleep_program().display(),
                TREE,
                cargo.display()
            ),
        );
        let metric = count_with(&wrapper, directory.path(), timeout);
        let recorded = std::fs::read_to_string(arguments).unwrap_or_default();
        (metric, recorded)
    }

    #[test]
    fn the_package_of_the_manifest_is_counted_from_the_tree() {
        let (metric, arguments) = run(
            "tree",
            &[("Cargo.toml", MANIFEST), ("Cargo.lock", LOCK)],
            Duration::from_secs(10),
        );
        assert_eq!(metric.status, "measured", "{metric:?}");
        assert_eq!(metric.value, Some(5));
        assert_eq!(metric.unit, "packages");
        assert_eq!(
            arguments.trim_end(),
            "tree --locked --offline --edges normal,build --prefix none --format {p} --color never --package app"
        );
    }

    #[test]
    fn a_member_without_a_package_name_and_a_lock_above_it_is_counted() {
        let (metric, arguments) = run(
            "tree",
            &[("Cargo.toml", "[workspace]\n"), ("Cargo.lock", LOCK)],
            Duration::from_secs(10),
        );
        assert_eq!(metric.value, Some(5));
        assert!(!arguments.contains("--package"), "{arguments}");
    }

    #[test]
    fn a_failed_or_hanging_tree_is_not_measured() {
        let files = [("Cargo.toml", MANIFEST), ("Cargo.lock", LOCK)];
        let (failed, _) = run("fail", &files, Duration::from_secs(10));
        assert_eq!(failed.status, "not_measured");
        assert_eq!(failed.value, None);
        assert_eq!(
            failed.reason.as_deref(),
            Some(
                "`cargo tree --locked --offline` failed; build the project first (`cargo build --release`) so Cargo.lock is current and every dependency is downloaded"
            )
        );
        let started = Instant::now();
        let (hung, _) = run("hang", &files, Duration::from_millis(500));
        assert!(started.elapsed() < Duration::from_secs(20));
        assert_eq!(
            hung.reason.as_deref(),
            Some("`cargo tree` did not finish within 0.5 s")
        );
    }
}
