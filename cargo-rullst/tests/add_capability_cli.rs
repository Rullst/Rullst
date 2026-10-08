//! `cargo rullst add <capability>` through the real binary: plain output
//! outside a terminal, idempotence, `--dry-run`, scope and usage errors.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const MANIFEST: &str = r#"[package]
name = "shop"
version = "0.1.0"
edition = "2024"

[dependencies]
# Framework facade.
rullst = { version = "13.0.0", default-features = false, features = ["orm"] }
"#;

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(args)
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn snapshot(root: &Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn add_enables_documents_and_stays_idempotent() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), MANIFEST).unwrap();
    fs::write(root.path().join(".env"), "APP_KEY=local\n").unwrap();

    let preview = cli(root.path(), &["add", "mail", "--dry-run"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    let text = stdout(&preview);
    assert!(text.contains("+ rullst = { version = \"13.0.0\", default-features = false, features = [\"orm\", \"mail\"] }"), "{text}");
    assert!(text.contains("Dry run: nothing was written."), "{text}");
    assert!(!text.contains("Next steps"), "{text}");
    assert!(!text.contains('\u{1b}'), "NO_COLOR output is plain: {text}");
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.toml")).unwrap(),
        MANIFEST
    );
    assert!(!root.path().join(".env.example").exists());

    let added = cli(root.path(), &["add", "mail"]);
    assert!(added.status.success(), "{}", stderr(&added));
    let text = stdout(&added);
    assert!(text.contains("Wire it in"), "{text}");
    assert!(text.contains("Next steps"), "{text}");
    assert!(text.contains("cargo rullst make:mail Welcome"), "{text}");
    let manifest = fs::read_to_string(root.path().join("Cargo.toml")).unwrap();
    assert!(manifest.contains("# Framework facade."));
    assert!(
        manifest.contains(r#"features = ["orm", "mail"]"#),
        "{manifest}"
    );
    assert!(
        fs::read_to_string(root.path().join(".env.example"))
            .unwrap()
            .contains("MAIL_FROM=")
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".env")).unwrap(),
        "APP_KEY=local\n"
    );

    let before = snapshot(root.path());
    let again = cli(root.path(), &["add", "mail"]);
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(
        stdout(&again).contains("already enabled"),
        "{}",
        stdout(&again)
    );
    assert_eq!(
        snapshot(root.path()),
        before,
        "a second run changes nothing"
    );
}

#[test]
fn add_refuses_outside_a_project_and_unknown_capabilities() {
    let empty = tempfile::tempdir().unwrap();
    let outside = cli(empty.path(), &["add", "auth"]);
    assert_eq!(outside.status.code(), Some(1));
    let message = stderr(&outside);
    assert!(message.contains("Not inside a Rullst project"), "{message}");
    assert!(
        fs::read_dir(empty.path()).unwrap().next().is_none(),
        "nothing written"
    );

    fs::write(empty.path().join("Cargo.toml"), MANIFEST).unwrap();
    let unknown = cli(empty.path(), &["add", "blockchain"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(
        stderr(&unknown).contains("possible values"),
        "{}",
        stderr(&unknown)
    );

    fs::write(
        empty.path().join("Cargo.toml"),
        "[dependencies]\nrullst = 13\n",
    )
    .unwrap();
    let unsupported = cli(empty.path(), &["add", "ai"]);
    assert_eq!(unsupported.status.code(), Some(1));
    let message = stderr(&unsupported);
    assert!(
        message.contains("The rullst dependency cannot take features"),
        "{message}"
    );
    assert!(message.contains("cargo-rullst-add"), "{message}");
}
