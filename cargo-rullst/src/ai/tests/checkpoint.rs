use super::*;
use std::fs;

fn run_git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn repository() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    run_git(root, &["init", "-q"]);
    fs::write(root.join("tracked.rs"), "fn original() {}\n").unwrap();
    fs::write(root.join(".gitignore"), "ignored.log\n").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-q", "-m", "initial"]);
    directory
}

#[test]
fn checkpoint_captures_the_work_tree_without_touching_index_or_files() {
    let directory = repository();
    let root = directory.path();
    fs::write(root.join("tracked.rs"), "fn edited() {}\n").unwrap();
    fs::write(root.join("staged.rs"), "fn staged() {}\n").unwrap();
    run_git(root, &["add", "staged.rs"]);
    fs::write(root.join("untracked.rs"), "fn new() {}\n").unwrap();
    fs::write(root.join("ignored.log"), "noise\n").unwrap();
    fs::write(root.join(".env"), "SECRET=value\n").unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::write(root.join("target/out.bin"), "build").unwrap();

    let status_before = run_git(root, &["status", "--porcelain"]);
    let staged_before = run_git(root, &["diff", "--cached", "--name-only"]);
    let stash_before = run_git(root, &["stash", "list"]);

    let checkpoint = create(root, "20261001T120000Z").unwrap();
    assert_eq!(
        checkpoint.reference,
        format!("{REF_PREFIX}20261001T120000Z")
    );

    assert_eq!(run_git(root, &["status", "--porcelain"]), status_before);
    assert_eq!(
        run_git(root, &["diff", "--cached", "--name-only"]),
        staged_before
    );
    assert_eq!(run_git(root, &["stash", "list"]), stash_before);
    assert_eq!(
        fs::read_to_string(root.join("tracked.rs")).unwrap(),
        "fn edited() {}\n"
    );

    let files = run_git(
        root,
        &["ls-tree", "-r", "--name-only", &checkpoint.reference],
    );
    for expected in ["tracked.rs", "staged.rs", "untracked.rs", ".gitignore"] {
        assert!(
            files.lines().any(|line| line == expected),
            "{expected}: {files}"
        );
    }
    for excluded in ["ignored.log", ".env", "target/out.bin"] {
        assert!(
            !files.lines().any(|line| line == excluded),
            "{excluded}: {files}"
        );
    }
    let content = run_git(
        root,
        &["show", &format!("{}:tracked.rs", checkpoint.reference)],
    );
    assert_eq!(content, "fn edited() {}\n");
    let parent = run_git(root, &["rev-parse", &format!("{}^", checkpoint.reference)]);
    assert_eq!(parent, run_git(root, &["rev-parse", "HEAD"]));

    // A second checkpoint in the same second gets a distinct ref.
    let again = create(root, "20261001T120000Z").unwrap();
    assert_eq!(again.reference, format!("{REF_PREFIX}20261001T120000Z-1"));

    // The documented restore command brings the checkpointed content back.
    fs::write(root.join("tracked.rs"), "fn broken() {}\n").unwrap();
    run_git(
        root,
        &[
            "restore",
            &format!("--source={}", checkpoint.reference),
            "--worktree",
            "--",
            ".",
        ],
    );
    assert_eq!(
        fs::read_to_string(root.join("tracked.rs")).unwrap(),
        "fn edited() {}\n"
    );
}

#[test]
fn repositories_without_commits_and_project_subdirectories_work() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    run_git(root, &["init", "-q"]);
    fs::create_dir_all(root.join("apps/web/src")).unwrap();
    fs::write(root.join("apps/web/src/main.rs"), "fn main() {}\n").unwrap();
    let checkpoint = create(&root.join("apps/web"), "20261001T120001Z").unwrap();
    let files = run_git(
        root,
        &["ls-tree", "-r", "--name-only", &checkpoint.reference],
    );
    assert!(files.contains("apps/web/src/main.rs"));
}

#[test]
fn directories_outside_git_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let result = create(directory.path(), "20261001T120002Z");
    assert!(matches!(
        result,
        Err(CheckpointError::NotARepository | CheckpointError::GitMissing)
    ));
}
