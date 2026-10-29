//! Configuration sources, their labels and the dotenv size bound.
use super::*;

#[test]
fn sources_have_exact_labels() {
    assert_eq!(Source::Environment.label(), "environment");
    assert_eq!(Source::DotEnv.label(), ".env");
    assert_eq!(Source::RullstToml.label(), "Rullst.toml");
}

#[test]
fn process_variables_come_from_the_environment() {
    // Cargo sets the package name for the test process.
    assert_eq!(
        ProcessVars.var("CARGO_PKG_NAME").as_deref(),
        Some("cargo-rullst")
    );
    assert_eq!(ProcessVars.var("RULLST_SURELY_UNSET_VARIABLE"), None);
}

#[test]
fn dotenv_files_larger_than_a_few_kib_are_parsed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(".env");
    let value = "v".repeat(4000);
    std::fs::write(&path, format!("APP_KEY={value}\n")).unwrap();
    assert_eq!(DotEnv::load(&path).get("APP_KEY"), Some(value.as_str()));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(DotEnv::load(&path), DotEnv::Missing);
}
