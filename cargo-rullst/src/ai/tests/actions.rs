use super::*;
use std::fs;

const PLAIN: Style = Style { color: false };

fn project() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {\n    old();\n}\n").unwrap();
    (directory, root)
}

fn write(path: &str, content: &str) -> Action {
    Action::WriteFile {
        path: path.into(),
        content: content.into(),
    }
}

fn edit(path: &str, find: &str, replace: &str) -> Action {
    Action::EditFile {
        path: path.into(),
        find: find.into(),
        replace: replace.into(),
    }
}

#[test]
fn edits_require_one_exact_match() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let prepared = prepare(
        &edit("src/main.rs", "old()", "new()"),
        Some(&root),
        &overlay,
    )
    .unwrap();
    let Prepared::File { new, old, .. } = &prepared else {
        panic!("expected a file change");
    };
    assert_eq!(new, "fn main() {\n    new();\n}\n");
    assert!(old.is_some());
    assert!(preview(&prepared, 1, 1, PLAIN).contains("- "));

    let missing = prepare(&edit("src/main.rs", "absent", "x"), Some(&root), &overlay);
    assert!(missing.err().unwrap().contains("does not occur"));
    fs::write(root.join("src/twice.rs"), "a a").unwrap();
    let twice = prepare(&edit("src/twice.rs", "a", "b"), Some(&root), &overlay);
    assert!(twice.err().unwrap().contains("2 times"));
    let absent = prepare(&edit("src/none.rs", "a", "b"), Some(&root), &overlay);
    assert!(absent.err().unwrap().contains("does not exist"));
}

#[test]
fn plans_build_on_earlier_planned_changes() {
    let (_guard, root) = project();
    let mut overlay = Overlay::new();
    let created = prepare(
        &write("notes.md", "Status: planned\n"),
        Some(&root),
        &overlay,
    )
    .unwrap();
    plan(&created, &mut overlay);
    let edited = prepare(
        &edit("notes.md", "planned", "reviewed"),
        Some(&root),
        &overlay,
    )
    .unwrap();
    let text = preview(&edited, 2, 2, PLAIN);
    assert!(
        text.contains("- Status: planned") && text.contains("+ Status: reviewed"),
        "{text}"
    );
    assert!(!root.join("notes.md").exists());
}

#[test]
fn unsafe_targets_and_commands_are_rejected_before_review() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    for action in [
        write("../escape.rs", "x"),
        write(".env", "SECRET=1"),
        write(".git/hooks/pre-commit", "#!/bin/sh"),
        write("target/x", "x"),
        Action::RunRullst {
            args: vec!["deploy".into()],
        },
        Action::Cargo {
            args: vec!["run".into()],
        },
    ] {
        assert!(
            prepare(&action, Some(&root), &overlay).is_err(),
            "{action:?}"
        );
    }
    let outside = prepare(&write("a.rs", "x"), None, &overlay);
    assert!(outside.err().unwrap().contains("outside a Rust project"));
}

#[test]
fn applying_writes_atomically_and_creates_parents() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let prepared = prepare(
        &write("src/controllers/posts.rs", "pub fn index() {}\n"),
        Some(&root),
        &overlay,
    )
    .unwrap();
    assert!(prepared.mutates());
    let mut out = Vec::new();
    let applied = apply(&prepared, &root, &mut out, PLAIN);
    assert!(applied.success);
    assert_eq!(applied.result, "created (1 lines)");
    assert_eq!(
        fs::read_to_string(root.join("src/controllers/posts.rs")).unwrap(),
        "pub fn index() {}\n"
    );
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("✓ created src/controllers/posts.rs")
    );
    let leftovers = fs::read_dir(root.join("src/controllers")).unwrap().count();
    assert_eq!(leftovers, 1, "no temporary files remain");
}

#[cfg(unix)]
#[test]
fn applying_keeps_permissions_and_refuses_links_created_after_review() {
    use std::os::unix::fs::PermissionsExt;
    let (_guard, root) = project();
    let script = root.join("run.sh");
    fs::write(&script, "echo old\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let overlay = Overlay::new();
    let prepared = prepare(&edit("run.sh", "old", "new"), Some(&root), &overlay).unwrap();
    assert!(apply(&prepared, &root, &mut Vec::new(), PLAIN).success);
    let mode = fs::metadata(&script).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);

    let outside = tempfile::tempdir().unwrap();
    let prepared = prepare(&write("src/late.rs", "x"), Some(&root), &overlay).unwrap();
    std::os::unix::fs::symlink(outside.path().join("target.rs"), root.join("src/late.rs")).unwrap();
    let applied = apply(&prepared, &root, &mut Vec::new(), PLAIN);
    assert!(!applied.success);
    assert!(!outside.path().join("target.rs").exists());
}

#[cfg(unix)]
#[test]
fn hard_links_are_detached_instead_of_written_through() {
    let (_guard, root) = project();
    let outside = tempfile::tempdir().unwrap();
    let original = outside.path().join("bashrc");
    fs::write(&original, "outside\n").unwrap();
    if fs::hard_link(&original, root.join("linked.txt")).is_err() {
        return; // temporary directories on different filesystems
    }
    let overlay = Overlay::new();
    let prepared = prepare(&write("linked.txt", "inside\n"), Some(&root), &overlay).unwrap();
    assert!(apply(&prepared, &root, &mut Vec::new(), PLAIN).success);
    assert_eq!(fs::read_to_string(&original).unwrap(), "outside\n");
    assert_eq!(
        fs::read_to_string(root.join("linked.txt")).unwrap(),
        "inside\n"
    );
}
