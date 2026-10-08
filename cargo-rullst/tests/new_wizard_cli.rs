//! `cargo rullst new` outside a terminal: dry runs, the never-prompt rule and
//! the next steps; `cargo rullst tour` as a plain list. No project is built.
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(args)
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .env_remove("PORT")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn count_files(root: &Path) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if entry.file_name() == ".git" {
            // `--vcs git` (the default) metadata, not generated project files.
            continue;
        }
        if kind.is_dir() {
            count += count_files(&entry.path());
        } else if kind.is_file() {
            count += 1;
        }
    }
    count
}

fn previewed_count(output: &str) -> usize {
    let line = output
        .lines()
        .find(|line| line.contains("files will be written"))
        .unwrap_or_else(|| panic!("no file count in:\n{output}"));
    line.split_whitespace()
        .find_map(|word| word.parse().ok())
        .unwrap()
}

#[test]
fn dry_runs_preview_exactly_the_files_a_real_run_writes_and_write_nothing() {
    let temporary = tempfile::tempdir().unwrap();
    let profiles: [&[&str]; 8] = [
        &["--blueprint", "blank"],
        &["--blueprint", "blank", "--api", "--no-database"],
        &[
            "--blueprint",
            "saas",
            "--database",
            "postgres",
            "--ai",
            "--redis",
        ],
        &["--blueprint", "lms"],
        &["--blueprint", "blog"],
        &["--blueprint", "portfolio", "--database", "mysql"],
        &["--blueprint", "erp", "--mongodb", "--qdrant"],
        &["--docker", "--nix", "--buildah", "--database", "turso"],
    ];
    for (index, profile) in profiles.iter().enumerate() {
        let name = format!("profile-{index}");
        let mut arguments = vec![
            "new",
            name.as_str(),
            "--default",
            "--skip-initial-migration",
        ];
        arguments.extend_from_slice(profile);

        let mut dry = arguments.clone();
        dry.push("--dry-run");
        let preview = cli(temporary.path(), &dry);
        let preview_text = text(&preview);
        assert!(preview.status.success(), "{preview_text}");
        assert!(preview_text.contains("Dry run: nothing was created."));
        assert!(preview_text.contains("Runs         no commands; files only"));
        assert!(
            !preview_text.contains('\x1b'),
            "plain output outside a terminal"
        );
        assert!(!temporary.path().join(&name).exists(), "{profile:?}");

        let created = cli(temporary.path(), &arguments);
        assert!(created.status.success(), "{}", text(&created));
        assert_eq!(
            previewed_count(&preview_text),
            count_files(&temporary.path().join(&name)),
            "{profile:?}: the preview must list what generation writes"
        );

        let again = cli(temporary.path(), &dry);
        assert!(
            !again.status.success(),
            "an existing destination is reported"
        );
        assert!(text(&again).contains("already exists"));
    }
}

#[test]
fn without_a_terminal_new_never_prompts_and_creates_nothing() {
    let temporary = tempfile::tempdir().unwrap();
    for arguments in [
        &["new", "app"][..],
        &["new"],
        &["new", "app", "--blueprint", "blog", "--database", "sqlite"],
        &["new", "app", "--dry-run"],
    ] {
        let output = cli(temporary.path(), arguments);
        let message = text(&output);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(message.contains("interactive terminal"), "{message}");
        assert!(message.contains("--default"), "{message}");
        assert!(!temporary.path().join("app").exists());
    }
}

#[test]
fn creation_ends_with_numbered_steps_to_the_welcome_page() {
    let temporary = tempfile::tempdir().unwrap();
    let html = cli(
        temporary.path(),
        &["new", "hello", "--default", "--skip-initial-migration"],
    );
    let output = text(&html);
    assert!(html.status.success(), "{output}");
    for expected in [
        "✨ Project 'hello' created successfully!",
        "Next steps",
        "  1  cd hello",
        "  2  cargo rullst dev",
        "  3  open http://127.0.0.1:3000   your generated welcome page",
        "cargo rullst tour",
    ] {
        assert!(output.contains(expected), "{expected}:\n{output}");
    }
    assert!(!output.contains("How to run:"));

    let api = Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(temporary.path())
        .args([
            "new",
            "service",
            "--default",
            "--api",
            "--no-database",
            "--skip-initial-migration",
        ])
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .env("PORT", "4123")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let output = text(&api);
    assert!(api.status.success(), "{output}");
    assert!(output.contains("curl http://127.0.0.1:4123/"), "{output}");
    assert!(
        output.contains("builds and serves with live reload"),
        "{output}"
    );
}

#[test]
fn the_tour_prints_its_steps_without_a_terminal() {
    let temporary = tempfile::tempdir().unwrap();
    for arguments in [&["tour", "--list"][..], &["tour"]] {
        let output = cli(temporary.path(), arguments);
        let listing = text(&output);
        assert!(output.status.success(), "{listing}");
        assert!(listing.starts_with("Rullst tour · 7 steps\n"), "{listing}");
        for expected in [
            "1. Create a project  (new)",
            "   Example  cargo rullst new tour-demo --default --dry-run",
            "4. Generate code  (make:*)",
            "7. Work with AI  (ai)",
        ] {
            assert!(listing.contains(expected), "{expected}:\n{listing}");
        }
        assert!(!listing.contains('\x1b'));
    }
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);
}
