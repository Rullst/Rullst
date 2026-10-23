//! Application scope and the final phase of an application or recovery.
use super::*;
use std::{fs, path::Path};

fn change(before: &str, after: &str) -> super::super::review::Change {
    super::super::review::Change {
        before: snapshot::record(before, Some(b"old")),
        after: snapshot::record(after, Some(b"new")),
    }
}

#[test]
fn only_manifests_and_the_root_lockfile_are_applied() {
    for name in ["Cargo.toml", "crates/web/Cargo.toml", "Cargo.lock"] {
        validate_scope(&change(name, name)).unwrap();
    }
    for (before, after) in [
        ("src/main.rs", "src/main.rs"),
        ("crates/web/Cargo.lock", "crates/web/Cargo.lock"),
        ("Cargo.toml", "crates/web/Cargo.toml"),
    ] {
        assert!(validate_scope(&change(before, after)).is_err(), "{before}");
    }
}

fn staged(root: &Path, content: &Path) -> Staged {
    let mut replacements = Vec::new();
    for name in ["Cargo.lock", "Cargo.toml"] {
        fs::write(root.join(name), format!("before {name}")).unwrap();
        fs::write(content.join(name), format!("after {name}")).unwrap();
        let current = snapshot::record(name, Some(format!("before {name}").as_bytes()));
        let permissions = super::super::transaction::Permissions::capture(root, &current).unwrap();
        replacements.push(Replacement {
            current,
            desired: snapshot::record(name, Some(format!("after {name}").as_bytes())),
            permissions,
        });
    }
    Staged::create(root, content, replacements).unwrap()
}

#[test]
fn finishing_commits_every_replacement_and_records_the_final_phase() {
    for (phase, finished) in [("applying", "applied"), ("restoring", "restored")] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let (root, content, verified) = (
            base.join("root"),
            base.join("content"),
            base.join("verified"),
        );
        for directory in [&root, &content, &verified] {
            fs::create_dir(directory).unwrap();
        }
        let mut intent = Intent {
            schema_version: "rullst.project-application-intent.v1".into(),
            phase: phase.into(),
            source: root.clone(),
            prepared_directory: base.join("prepared"),
            verified_directory: verified.clone(),
            receipt_sha256: "a".repeat(64),
            review_sha256: "b".repeat(64),
            changes: Vec::new(),
            permissions: Vec::new(),
        };
        assert_eq!(finish(staged(&root, &content), &mut intent).unwrap(), 2);
        assert_eq!(intent.phase, finished);
        assert_eq!(
            fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            "after Cargo.toml"
        );
        let stored: serde_json::Value =
            serde_json::from_slice(&fs::read(verified.join("application.json")).unwrap()).unwrap();
        assert_eq!(stored["phase"], finished);
    }
}

#[test]
fn approvals_are_complete_lowercase_digests() {
    for (name, about) in [
        ("apply", "Apply the exact approved"),
        ("recover", "Restore only this approved"),
    ] {
        let text = command(name).get_about().unwrap().to_string();
        assert!(text.starts_with(about), "{text}");
    }
    let invalid =
        "project preparation rejected: approval must be the complete lowercase SHA-256 from review";
    for approval in ["A".repeat(64), "a".repeat(63), "g".repeat(64)] {
        let matches = command("apply")
            .try_get_matches_from([
                "apply",
                "--verified",
                "/rullst-missing-verification",
                "--approved-review",
                &approval,
            ])
            .unwrap();
        assert_eq!(
            execute(&matches, false).unwrap_err().to_string(),
            invalid,
            "{approval}"
        );
    }
}
