use super::*;

struct TempWorktree {
    path: PathBuf,
}

impl TempWorktree {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("rullst-hook-{label}-{}", rand::random::<u64>()));
        fs::create_dir_all(&path).expect("temporary hook worktree");
        Self { path }
    }

    fn init(&self) -> PathBuf {
        let hooks = self.path.join(".git/hooks");
        fs::create_dir_all(&hooks).expect("temporary Git hooks");
        hooks
    }
}

impl Drop for TempWorktree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn missing_worktree_fails_without_creating_git_metadata() {
    let worktree = TempWorktree::new("missing");
    assert!(matches!(
        install_git_hooks_with(&worktree.path, |_| None),
        Err(HookInstallError::NotGitWorktree(_))
    ));
    assert!(!worktree.path.join(".git").exists());
}

#[test]
fn installation_is_idempotent_and_executable() {
    let worktree = TempWorktree::new("idempotent");
    let hooks = worktree.init();
    install_git_hooks_with(&worktree.path, |_| None).expect("first hook installation");
    let pre_commit = hooks.join("pre-commit");
    let first = fs::read(&pre_commit).expect("first managed hook");
    install_git_hooks_with(&worktree.path, |_| None).expect("idempotent hook installation");
    assert_eq!(fs::read(&pre_commit).expect("second managed hook"), first);
    assert!(!backup_path(&pre_commit).exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&pre_commit)
            .expect("managed hook metadata")
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0);
    }
}

#[test]
fn existing_hooks_are_preserved_chained_and_not_overwritten_on_reinstall() {
    let worktree = TempWorktree::new("preserve");
    let hooks = worktree.init();
    let pre_commit = hooks.join("pre-commit");
    let commit_msg = hooks.join("commit-msg");
    fs::write(&pre_commit, "#!/bin/sh\necho existing-pre\n").expect("existing pre-commit");
    fs::write(&commit_msg, "#!/bin/sh\necho existing-message\n").expect("existing commit-msg");

    install_git_hooks_with(&worktree.path, |_| None).expect("preserving hook installation");
    assert_eq!(
        fs::read_to_string(backup_path(&pre_commit)).expect("preserved pre-commit"),
        "#!/bin/sh\necho existing-pre\n"
    );
    assert_eq!(
        fs::read_to_string(backup_path(&commit_msg)).expect("preserved commit-msg"),
        "#!/bin/sh\necho existing-message\n"
    );
    let wrapper = fs::read_to_string(&pre_commit).expect("managed pre-commit wrapper");
    assert!(wrapper.contains("${0}.rullst-original"));
    install_git_hooks_with(&worktree.path, |_| None)
        .expect("idempotent preserved hook installation");
    assert_eq!(
        fs::read_to_string(backup_path(&pre_commit)).expect("unchanged pre-commit backup"),
        "#!/bin/sh\necho existing-pre\n"
    );
}

#[test]
fn backup_collision_fails_before_mutating_any_hook() {
    let worktree = TempWorktree::new("collision");
    let hooks = worktree.init();
    let pre_commit = hooks.join("pre-commit");
    fs::write(&pre_commit, "custom").expect("custom pre-commit");
    fs::write(backup_path(&pre_commit), "older backup").expect("existing backup");

    assert!(matches!(
        install_git_hooks_with(&worktree.path, |_| None),
        Err(HookInstallError::BackupConflict { .. })
    ));
    assert_eq!(
        fs::read_to_string(&pre_commit).expect("untouched custom hook"),
        "custom"
    );
    assert!(!hooks.join("commit-msg").exists());
}

#[cfg(unix)]
#[test]
fn commit_msg_accepts_git_generated_subjects_and_rejects_free_text() {
    let worktree = TempWorktree::new("commit-msg");
    let script = worktree.path.join("commit-msg");
    fs::write(&script, COMMIT_MSG_SCRIPT).expect("commit-msg script");
    let accepts = |subject: &str| {
        let message = worktree.path.join("COMMIT_EDITMSG");
        fs::write(&message, format!("{subject}\n\nbody\n")).expect("commit message");
        std::process::Command::new("sh")
            .arg(&script)
            .arg(&message)
            .output()
            .expect("run commit-msg")
            .status
            .success()
    };
    for subject in [
        "feat(cli): add a generator",
        "fix!: drop a legacy flag",
        "Merge branch 'main' into feat/x",
        "Merge remote-tracking branch 'origin/main' into fix/v13-capital-low-fixes",
        "Merge tag 'v1.2.0'",
        "Merge branches 'a' and 'b'",
        "Merge pull request #367 from Rullst/chore/v13-remove-labs-runner",
        "Merge commit 'abc1234'",
        "Revert \"feat(x): y\"",
        "fixup! feat(cli): add a generator",
        "squash! fix(auth): hash passwords off the runtime",
        "amend! docs: update the guide",
    ] {
        assert!(accepts(subject), "rejected: {subject}");
    }
    for subject in ["update stuff", "Merged things", "Revert this", "fixup: x"] {
        assert!(!accepts(subject), "accepted: {subject}");
    }
}

#[test]
fn a_core_hooks_path_elsewhere_fails_before_writing_any_hook() {
    let worktree = TempWorktree::new("hooks-path");
    let hooks = worktree.init();
    let husky = worktree.path.join(".husky");
    let error = install_git_hooks_with(&worktree.path, |_| Some(husky.clone()))
        .expect_err("a foreign core.hooksPath must be refused");
    assert!(error.to_string().contains("core.hooksPath"));
    assert!(!hooks.join("pre-commit").exists());
    assert!(!hooks.join("commit-msg").exists());

    let default = worktree.path.join(".git/hooks");
    install_git_hooks_with(&worktree.path, |_| Some(default))
        .expect("core.hooksPath naming the default directory is accepted");
    assert!(hooks.join("pre-commit").exists());
}

#[test]
fn git_reports_a_relative_core_hooks_path_against_the_worktree() {
    let worktree = TempWorktree::new("git-config");
    let git = |arguments: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&worktree.path)
            .args(arguments)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    };
    // Git is optional for this test; the stubbed tests cover the policy.
    if !git(&["init", "--quiet"]).is_ok_and(|status| status.success()) {
        return;
    }
    assert!(
        git(&["config", "core.hooksPath", ".husky"])
            .expect("git config")
            .success()
    );
    let root = worktree.path.canonicalize().expect("canonical worktree");
    assert_eq!(configured_hooks_path(&root), Some(root.join(".husky")));
    assert!(matches!(
        install_git_hooks_at(&root),
        Err(HookInstallError::Io { .. })
    ));
    assert!(!root.join(".git/hooks/pre-commit").exists());
}

#[test]
fn linked_worktree_uses_the_common_git_hooks_directory() {
    let worktree = TempWorktree::new("linked");
    let common = worktree.path.join("common.git");
    let linked = common.join("worktrees/linked");
    fs::create_dir_all(&linked).expect("linked Git directory");
    fs::write(
        worktree.path.join(".git"),
        "gitdir: common.git/worktrees/linked\n",
    )
    .expect("linked worktree metadata");
    fs::write(linked.join("commondir"), "../..\n").expect("common directory metadata");

    let hooks = install_git_hooks_with(&worktree.path, |_| None).expect("linked hook installation");
    assert_eq!(hooks, common.canonicalize().unwrap().join("hooks"));
    assert!(hooks.join("pre-commit").exists());
}
