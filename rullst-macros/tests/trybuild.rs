use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[cfg_attr(miri, ignore)]
fn ui() {
    copy_i18n_fixtures();
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

/// `i18n_catalog!` reads `dir` relative to `CARGO_MANIFEST_DIR`, which for
/// these tests is trybuild's generated project in
/// `<target>/tests/trybuild/rullst-macros`, so the fixture catalogs are
/// copied there first.
fn copy_i18n_fixtures() {
    let destination = trybuild_target_dir().join("tests/trybuild/rullst-macros/tests/i18n");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/i18n");
    for case in fs::read_dir(&source).expect("fixture directory") {
        let case = case.expect("fixture entry").path();
        let Some(name) = case.file_name() else {
            continue;
        };
        let copy = destination.join(name);
        fs::create_dir_all(&copy).expect("create fixture copy");
        for file in fs::read_dir(&case).expect("fixture case") {
            let file = file.expect("fixture file").path();
            if let Some(file_name) = file.file_name() {
                fs::copy(&file, copy.join(file_name)).expect("copy fixture");
            }
        }
    }
}

/// The target directory trybuild builds its project in. Like trybuild, ask
/// `cargo metadata`, which follows `CARGO_TARGET_DIR` and Cargo config, instead
/// of assuming the directory this test was built in: `cargo llvm-cov` builds
/// the test in its own target directory while trybuild keeps the workspace's.
fn trybuild_target_dir() -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version=1"])
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata JSON");
    let target = metadata["target_directory"]
        .as_str()
        .expect("cargo metadata names the target directory");
    PathBuf::from(target)
}
