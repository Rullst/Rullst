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
        &root,
        &overlay,
    )
    .unwrap();
    let Prepared::File { new, old, .. } = &prepared else {
        panic!("expected a file change");
    };
    assert_eq!(new, "fn main() {\n    new();\n}\n");
    assert!(old.is_some());
    assert!(preview(&prepared, 1, 1, PLAIN).contains("- "));

    let missing = prepare(
        &edit("src/main.rs", "absent", "x"),
        Some(&root),
        &root,
        &overlay,
    );
    assert!(missing.err().unwrap().contains("does not occur"));
    fs::write(root.join("src/twice.rs"), "a a").unwrap();
    let twice = prepare(
        &edit("src/twice.rs", "a", "b"),
        Some(&root),
        &root,
        &overlay,
    );
    assert!(twice.err().unwrap().contains("2 times"));
    let absent = prepare(&edit("src/none.rs", "a", "b"), Some(&root), &root, &overlay);
    assert!(absent.err().unwrap().contains("does not exist"));
}

#[test]
fn plans_build_on_earlier_planned_changes() {
    let (_guard, root) = project();
    let mut overlay = Overlay::new();
    let created = prepare(
        &write("notes.md", "Status: planned\n"),
        Some(&root),
        &root,
        &overlay,
    )
    .unwrap();
    plan(&created, &mut overlay);
    let edited = prepare(
        &edit("notes.md", "planned", "reviewed"),
        Some(&root),
        &root,
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
            prepare(&action, Some(&root), &root, &overlay).is_err(),
            "{action:?}"
        );
    }
    let outside = prepare(&write("a.rs", "x"), None, &root, &overlay);
    assert!(outside.err().unwrap().contains("outside a Rust project"));
}

#[test]
fn applying_writes_atomically_and_creates_parents() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let prepared = prepare(
        &write("src/controllers/posts.rs", "pub fn index() {}\n"),
        Some(&root),
        &root,
        &overlay,
    )
    .unwrap();
    assert!(prepared.mutates());
    let mut out = Vec::new();
    let applied = apply(&prepared, Some(&root), &mut out, PLAIN);
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
    let prepared = prepare(&edit("run.sh", "old", "new"), Some(&root), &root, &overlay).unwrap();
    assert!(apply(&prepared, Some(&root), &mut Vec::new(), PLAIN).success);
    let mode = fs::metadata(&script).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);

    let outside = tempfile::tempdir().unwrap();
    let prepared = prepare(&write("src/late.rs", "x"), Some(&root), &root, &overlay).unwrap();
    std::os::unix::fs::symlink(outside.path().join("target.rs"), root.join("src/late.rs")).unwrap();
    let applied = apply(&prepared, Some(&root), &mut Vec::new(), PLAIN);
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
    let prepared = prepare(
        &write("linked.txt", "inside\n"),
        Some(&root),
        &root,
        &overlay,
    )
    .unwrap();
    assert!(apply(&prepared, Some(&root), &mut Vec::new(), PLAIN).success);
    assert_eq!(fs::read_to_string(&original).unwrap(), "outside\n");
    assert_eq!(
        fs::read_to_string(root.join("linked.txt")).unwrap(),
        "inside\n"
    );
}

fn run(args: &[&str]) -> Action {
    Action::RunRullst {
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
    }
}

#[test]
fn new_projects_are_only_proposed_outside_a_project_with_valid_flags() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let inside = prepare(&run(&["new", "shop"]), Some(&root), &root, &overlay);
    assert!(
        inside
            .err()
            .unwrap()
            .contains("only available outside a project")
    );

    let prepared = prepare(
        &run(&["new", "shop", "--blueprint", "saas", "--database=postgres"]),
        None,
        &root,
        &overlay,
    )
    .unwrap();
    assert!(prepared.always_confirm() && !prepared.mutates());
    assert_eq!(
        prepared.summary(),
        "cargo rullst new shop --default --blueprint saas --database postgres"
    );
    let Prepared::NewProject { directory, .. } = &prepared else {
        panic!("expected a new project");
    };
    assert_eq!(directory, &root.join("shop"));

    for args in [
        &["new", "Shop"][..],
        &["new", "../x"],
        &["new", "shop", "--blueprint", "wordpress"],
        &["new", "shop", "--database", "oracle"],
        &["new", "shop", "--nix"],
        &["new", "shop", "--blueprint"],
        &["new"],
    ] {
        assert!(
            prepare(&run(args), None, &root, &overlay).is_err(),
            "{args:?}"
        );
    }
    // An existing directory is never reused.
    assert!(prepare(&run(&["new", "src"]), None, &root, &overlay).is_err());
}

#[test]
fn migrations_are_confirmed_and_refused_outside_development() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let prepared = prepare(&run(&["db:migrate"]), Some(&root), &root, &overlay).unwrap();
    assert!(prepared.always_confirm() && !prepared.mutates());
    assert!(preview(&prepared, 1, 1, PLAIN).contains("cannot undo"));
    fs::write(root.join(".env"), "APP_ENV=production\n").unwrap();
    let refused = prepare(&run(&["db:migrate"]), Some(&root), &root, &overlay);
    assert!(refused.err().unwrap().contains("production"));
    assert!(prepare(&run(&["db:migrate"]), None, &root, &overlay).is_err());
}

#[cfg(unix)]
#[test]
fn a_created_project_becomes_the_new_root() {
    use std::os::unix::fs::PermissionsExt;
    let (_guard, root) = project();
    let fake = root.join("fake-rullst");
    fs::write(
        &fake,
        "#!/bin/sh\nmkdir -p \"$2/src\" && printf '[package]\\nname = \"%s\"\\n' \"$2\" > \"$2/Cargo.toml\"\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    crate::ai::commands::TEST_RULLST_PROGRAM.with(|program| *program.borrow_mut() = Some(fake));
    let overlay = Overlay::new();
    let prepared = prepare(&run(&["new", "shop"]), None, &root, &overlay).unwrap();
    let applied = apply(&prepared, None, &mut Vec::new(), PLAIN);
    crate::ai::commands::TEST_RULLST_PROGRAM.with(|program| *program.borrow_mut() = None);
    assert!(applied.success, "{}", applied.result);
    assert_eq!(
        applied.new_root,
        Some(fs::canonicalize(root.join("shop")).unwrap())
    );
}
