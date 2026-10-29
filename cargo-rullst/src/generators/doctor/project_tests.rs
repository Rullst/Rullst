//! Version comparison, missing keys, APP_KEY quality and `.env` in Git.
use super::super::report::Status;
use super::*;
use std::collections::HashMap;

fn context(root: &Path, dotenv: &[(&str, &str)], example: &[(&str, &str)]) -> ProjectContext {
    let parsed = |entries: &[(&str, &str)]| {
        DotEnv::Parsed(
            entries
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
    };
    ProjectContext {
        root: root.to_path_buf(),
        relative_root: None,
        name: "fixture".into(),
        manifest: String::new(),
        dotenv: parsed(dotenv),
        example: parsed(example),
        rullst_toml: None,
    }
}

#[test]
fn a_newer_project_asks_for_the_matching_cli() {
    let check = version_check(Some("14.0.0"), "13.0.0");
    assert_eq!(check.status, Status::Warn);
    assert_eq!(
        check.detail,
        "project uses rullst 14.0.0, newer than this CLI (13.0.0)"
    );
    assert_eq!(
        version_check(Some("12.3.0"), "13.0.0").detail,
        "project uses rullst 12.3.0; this CLI is 13.0.0"
    );
}

#[test]
fn more_than_eight_missing_keys_are_summarized() {
    let keys: Vec<String> = (0..10).map(|index| format!("KEY_{index}")).collect();
    let example: Vec<(&str, &str)> = keys.iter().map(|key| (key.as_str(), "x")).collect();
    let project = context(Path::new("."), &[], &example);
    let check = env_keys_check(&project, &HashMap::<String, String>::new()).unwrap();
    assert!(check.detail.ends_with(" (+2 more)"), "{}", check.detail);
    let project = context(Path::new("."), &[], &example[..8]);
    let check = env_keys_check(&project, &HashMap::<String, String>::new()).unwrap();
    assert!(!check.detail.contains("more"), "{}", check.detail);
}

#[test]
fn app_keys_must_be_real_and_at_least_32_characters() {
    let check = |key: &str| {
        let project = context(Path::new("."), &[("APP_KEY", key)], &[]);
        app_key_check(&project, &HashMap::<String, String>::new()).unwrap()
    };
    let exact = "k".repeat(32);
    assert_eq!(check(&exact).status, Status::Pass);
    assert_eq!(
        check(&"k".repeat(31)).detail,
        "shorter than 32 characters in .env"
    );
    assert_eq!(
        check("replace_with_a_long_random_application_key").detail,
        "still a placeholder in .env"
    );
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn env_files_must_be_ignored_and_untracked() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    assert!(env_git_check(root, true).is_none());
    std::fs::write(root.join(".env"), "APP_KEY=x\n").unwrap();
    assert!(env_git_check(root, false).is_none());
    assert_eq!(env_git_check(root, true).unwrap().status, Status::Info);
    git(root, &["init", "--quiet"]);
    let check = env_git_check(root, true).unwrap();
    assert_eq!(
        (check.status, check.detail.as_str()),
        (Status::Warn, ".env is not ignored by Git")
    );
    std::fs::write(root.join(".gitignore"), ".env\n").unwrap();
    let check = env_git_check(root, true).unwrap();
    assert_eq!(
        (check.status, check.detail.as_str()),
        (Status::Pass, "ignored and untracked")
    );
    git(root, &["add", "--force", ".env"]);
    assert_eq!(env_git_check(root, true).unwrap().status, Status::Fail);
}
