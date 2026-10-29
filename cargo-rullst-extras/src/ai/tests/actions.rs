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

#[test]
fn inspect_never_reads_a_file_the_path_policy_refuses() {
    let (_guard, root) = project();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "TOKEN=1\n").unwrap();
    fs::write(
        root.join(".env"),
        "DATABASE_URL=postgres://user:pass@db/app\n",
    )
    .unwrap();
    fs::write(
        root.join("id_ed25519"),
        "-----BEGIN OPENSSH PRIVATE KEY-----\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path().join("secret.txt"), root.join("notes.txt")).unwrap();
    let overlay = Overlay::new();
    for target in [
        ".env",
        "id_ed25519",
        "notes.txt",
        "src/main.rs",
        "C:/Users/me/.ssh/id_rsa",
    ] {
        let refused = prepare(&run(&["inspect", target]), Some(&root), &root, &overlay);
        assert!(refused.is_err(), "inspect {target} was allowed");
    }
    assert!(
        prepare(
            &run(&["inspect", "routes", "--x"]),
            Some(&root),
            &root,
            &overlay
        )
        .is_err()
    );
    for args in [
        &["inspect"][..],
        &["inspect", "routes"],
        &["inspect", "models"],
    ] {
        let prepared = prepare(&run(args), Some(&root), &root, &overlay).unwrap();
        assert!(!prepared.mutates(), "{args:?}");
    }
    assert!(prepare(&run(&["inspect", "schema"]), Some(&root), &root, &overlay).is_ok());
    // A schema snapshot that is a link is printed in full by `inspect schema`.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            root.join("rullst-schema.json"),
        )
        .unwrap();
        let refused = prepare(&run(&["inspect", "schema"]), Some(&root), &root, &overlay);
        assert!(refused.err().unwrap().contains("symbolic link"));
    }
}

#[test]
fn path_valued_flags_follow_the_path_policy() {
    let (_guard, root) = project();
    fs::create_dir_all(root.join("api")).unwrap();
    fs::write(root.join("api/openapi.json"), "{}").unwrap();
    fs::write(root.join(".env"), "SECRET=1\n").unwrap();
    let overlay = Overlay::new();
    let generate = |schema: &str, output: &str| {
        run(&[
            "generate:api",
            "--schema",
            schema,
            &format!("--output={output}"),
        ])
    };
    assert!(
        prepare(
            &generate("api/openapi.json", "src/api"),
            Some(&root),
            &root,
            &overlay
        )
        .is_ok()
    );
    for (schema, output) in [
        (".env", "src/api"),
        ("api/openapi.json", ".git/hooks"),
        ("api/openapi.json", "target/api"),
        ("api/openapi.json", "src/main.rs"),
        ("api", "src/api"),
    ] {
        assert!(
            prepare(&generate(schema, output), Some(&root), &root, &overlay).is_err(),
            "{schema} {output}"
        );
    }
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("linked")).unwrap();
        let refused = prepare(
            &generate("api/openapi.json", "linked/api"),
            Some(&root),
            &root,
            &overlay,
        );
        assert!(refused.err().unwrap().contains("symbolic link"));
        let privacy = run(&["make:privacy", "--privacy-source", "linked"]);
        assert!(prepare(&privacy, Some(&root), &root, &overlay).is_err());
    }
}

#[test]
fn tests_and_proc_macros_are_flagged_in_the_review() {
    let (_guard, root) = project();
    let overlay = Overlay::new();
    let flagged = |action: Action| {
        let prepared = prepare(&action, Some(&root), &root, &overlay).unwrap();
        preview(&prepared, 1, 1, PLAIN).contains("review carefully")
    };
    assert!(flagged(write("tests/smoke.rs", "fn main() {}\n")));
    assert!(flagged(write(
        "src/models/post.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n"
    )));
    assert!(flagged(write(
        "macros/src/lib.rs",
        "use proc_macro::TokenStream;\n"
    )));
    assert!(!flagged(write("src/models/post.rs", "pub struct Post;\n")));
    assert!(!flagged(write(
        "notes.md",
        "Run #[test] functions with cargo test.\n"
    )));
}

/// Creates a file through the assistant and returns its mode.
#[cfg(unix)]
fn created_mode(root: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    let overlay = Overlay::new();
    let prepared = prepare(
        &write("config/seed.toml", "key = 1\n"),
        Some(root),
        root,
        &overlay,
    )
    .unwrap();
    assert!(apply(&prepared, Some(root), &mut Vec::new(), PLAIN).success);
    fs::metadata(root.join("config/seed.toml"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

/// Run by [`new_files_follow_the_umask`] in a child process with umask 077.
#[cfg(unix)]
#[test]
#[ignore = "run in a child process by new_files_follow_the_umask"]
fn new_file_mode_under_umask_077() {
    if std::env::var_os("RULLST_TEST_UMASK_CHILD").is_none() {
        return;
    }
    let (_guard, root) = project();
    assert_eq!(created_mode(&root), 0o600);
}

#[cfg(unix)]
#[test]
fn new_files_follow_the_umask() {
    use std::os::unix::fs::PermissionsExt;
    let (_guard, root) = project();
    // A file created normally in this process gets the same mode.
    fs::write(root.join("plain.toml"), "").unwrap();
    let expected = fs::metadata(root.join("plain.toml"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(created_mode(&root), expected);

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("umask 077 && exec \"$0\" \"$@\"")
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ai::actions::tests::new_file_mode_under_umask_077",
            "--ignored",
            "--test-threads=1",
        ])
        .env("RULLST_TEST_UMASK_CHILD", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("1 passed"), "{stdout}");
}
