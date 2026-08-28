use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MANAGED_MARKER: &str = "# Managed by cargo-rullst hook:install";
const ORIGINAL_SUFFIX: &str = "rullst-original";

const PRE_COMMIT_SCRIPT: &str = r#"#!/bin/sh
# Managed by cargo-rullst hook:install
original_hook="${0}.rullst-original"
if [ -x "$original_hook" ]; then
    "$original_hook" "$@" || exit $?
fi

echo "🛡️ Running Rullst pre-commit quality & security checks..."

echo "  🎨 Checking code formatting (rustfmt)..."
if ! cargo fmt --all -- --check; then
    echo "❌ rustfmt formatting check failed. Run 'cargo fmt --all' to fix formatting."
    exit 1
fi

echo "  🔍 Running Clippy with zero-warnings policy..."
if ! cargo clippy --workspace --all-targets -- -D warnings; then
    echo "❌ Clippy warnings detected. Fix all warnings before committing."
    exit 1
fi

echo "  🛡️ Running Rullst static IDOR / BOLA route audit..."
if [ -f "cargo-rullst/Cargo.toml" ]; then
    cargo run --quiet -p cargo-rullst --bin rullst -- audit --idor
else
    cargo rullst audit --idor
fi
if [ $? -ne 0 ]; then
    echo "❌ IDOR / BOLA authorization check failed. Classify each parameterized route and enforce its owner, role, or admin boundary."
    exit 1
fi

echo "✅ All Rullst pre-commit checks passed cleanly!"
"#;

const COMMIT_MSG_SCRIPT: &str = r#"#!/bin/sh
# Managed by cargo-rullst hook:install
original_hook="${0}.rullst-original"
if [ -x "$original_hook" ]; then
    "$original_hook" "$@" || exit $?
fi

commit_regex='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9_-]+\))?!?: .{1,80}$'
merge_regex='^Merge branch .*'
commit_message=$(head -n 1 "$1")

if echo "$commit_message" | grep -qE "$merge_regex"; then
    exit 0
fi
if ! echo "$commit_message" | grep -qE "$commit_regex"; then
    echo "❌ ERROR: Invalid commit message format."
    echo "   Commit message must follow Conventional Commits: <type>(<scope>): <description>"
    echo "   Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"
    echo "   Example: fix(auth): use spawn_blocking for async password hashing"
    exit 1
fi
"#;

#[derive(Debug, thiserror::Error)]
pub enum HookInstallError {
    #[error("`{0}` is not inside a Git worktree; initialize or enter a repository first")]
    NotGitWorktree(PathBuf),
    #[error("Git metadata file `{0}` does not contain a valid `gitdir:` target")]
    InvalidGitMetadata(PathBuf),
    #[error(
        "cannot preserve existing hook `{hook}` because backup `{backup}` already exists; reconcile them manually"
    )]
    BackupConflict { hook: PathBuf, backup: PathBuf },
    #[error("failed to {operation} `{path}`: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> HookInstallError {
    HookInstallError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

fn find_worktree(start: &Path) -> Result<PathBuf, HookInstallError> {
    let absolute = start
        .canonicalize()
        .map_err(|error| io_error("resolve working directory", start, error))?;
    absolute
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
        .ok_or(HookInstallError::NotGitWorktree(absolute))
}

fn metadata_target(metadata_file: &Path) -> Result<PathBuf, HookInstallError> {
    let contents = fs::read_to_string(metadata_file)
        .map_err(|error| io_error("read Git metadata", metadata_file, error))?;
    let target = contents
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HookInstallError::InvalidGitMetadata(metadata_file.to_path_buf()))?;
    let target = PathBuf::from(target);
    if target.is_absolute() {
        Ok(target)
    } else {
        let parent = metadata_file
            .parent()
            .ok_or_else(|| HookInstallError::InvalidGitMetadata(metadata_file.to_path_buf()))?;
        Ok(parent.join(target))
    }
}

fn hooks_directory(worktree: &Path) -> Result<PathBuf, HookInstallError> {
    let metadata = worktree.join(".git");
    if metadata.is_dir() {
        return Ok(metadata.join("hooks"));
    }
    if !metadata.is_file() {
        return Err(HookInstallError::NotGitWorktree(worktree.to_path_buf()));
    }
    let git_dir = metadata_target(&metadata)?;
    let common_metadata = git_dir.join("commondir");
    if !common_metadata.is_file() {
        return Ok(git_dir.join("hooks"));
    }
    let common = fs::read_to_string(&common_metadata)
        .map_err(|error| io_error("read Git common directory", &common_metadata, error))?;
    let common = PathBuf::from(common.trim());
    let common = if common.is_absolute() {
        common
    } else {
        git_dir.join(common)
    };
    Ok(common.join("hooks"))
}

fn backup_path(hook: &Path) -> PathBuf {
    let name = hook
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("hook");
    hook.with_file_name(format!("{name}.{ORIGINAL_SUFFIX}"))
}

fn is_managed(hook: &Path) -> Result<bool, HookInstallError> {
    if !hook.exists() {
        return Ok(false);
    }
    let contents = fs::read(hook).map_err(|error| io_error("read existing hook", hook, error))?;
    Ok(contents
        .windows(MANAGED_MARKER.len())
        .any(|window| window == MANAGED_MARKER.as_bytes()))
}

fn preflight_hook(hook: &Path) -> Result<(), HookInstallError> {
    let backup = backup_path(hook);
    if hook.exists() && !is_managed(hook)? && backup.exists() {
        return Err(HookInstallError::BackupConflict {
            hook: hook.to_path_buf(),
            backup,
        });
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), HookInstallError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata =
            fs::metadata(path).map_err(|error| io_error("read hook permissions", path, error))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|error| io_error("set executable hook permissions", path, error))?;
    }
    Ok(())
}

fn temporary_hook_path(hook: &Path, label: &str) -> PathBuf {
    let name = hook
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("hook");
    hook.with_file_name(format!(
        ".{name}.{label}-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ))
}

fn stage_hook(hook: &Path, script: &str) -> Result<PathBuf, HookInstallError> {
    let staged = temporary_hook_path(hook, "rullst-staged");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)
        .map_err(|error| io_error("create staged managed hook", &staged, error))?;
    if let Err(error) = file
        .write_all(script.as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&staged);
        return Err(io_error("write staged managed hook", &staged, error));
    }
    if let Err(error) = make_executable(&staged) {
        let _ = fs::remove_file(&staged);
        return Err(error);
    }
    Ok(staged)
}

fn install_hook(hook: &Path, script: &str) -> Result<(), HookInstallError> {
    let managed = is_managed(hook)?;
    if managed {
        let current = fs::read(hook).map_err(|error| io_error("read managed hook", hook, error))?;
        if current == script.as_bytes() {
            return make_executable(hook);
        }
    }
    let staged = stage_hook(hook, script)?;
    let backup = backup_path(hook);
    let previous = if hook.exists() {
        let previous = if managed {
            temporary_hook_path(hook, "rullst-previous")
        } else {
            backup.clone()
        };
        if let Err(error) = fs::rename(hook, &previous) {
            let _ = fs::remove_file(&staged);
            return Err(io_error("preserve existing hook as", &previous, error));
        }
        Some(previous)
    } else {
        None
    };
    if let Err(error) = fs::rename(&staged, hook) {
        if let Some(previous) = previous.as_ref() {
            let _ = fs::rename(previous, hook);
        }
        let _ = fs::remove_file(&staged);
        return Err(io_error("activate managed hook", hook, error));
    }
    if managed && let Some(previous) = previous {
        fs::remove_file(&previous)
            .map_err(|error| io_error("remove superseded managed hook", &previous, error))?;
    }
    Ok(())
}

fn install_git_hooks_at(start: &Path) -> Result<PathBuf, HookInstallError> {
    let worktree = find_worktree(start)?;
    let hooks_dir = hooks_directory(&worktree)?;
    fs::create_dir_all(&hooks_dir)
        .map_err(|error| io_error("create hooks directory", &hooks_dir, error))?;
    let pre_commit = hooks_dir.join("pre-commit");
    let commit_msg = hooks_dir.join("commit-msg");
    preflight_hook(&pre_commit)?;
    preflight_hook(&commit_msg)?;
    install_hook(&pre_commit, PRE_COMMIT_SCRIPT)?;
    install_hook(&commit_msg, COMMIT_MSG_SCRIPT)?;
    Ok(hooks_dir)
}

pub fn install_git_pre_commit_hook() -> Result<(), HookInstallError> {
    println!(
        "{}",
        "⚓ Installing Rullst Git Quality & Security Hooks..."
            .bright_cyan()
            .bold()
    );
    let current = std::env::current_dir()
        .map_err(|error| io_error("resolve current directory", Path::new("."), error))?;
    let hooks_dir = install_git_hooks_at(&current)?;
    println!(
        "  {} Managed hooks installed safely in '{}'; active hooks that existed were preserved and chained.",
        "[SUCCESS]".green().bold(),
        hooks_dir.display()
    );
    println!(
        "  {} The hooks enforce Conventional Commits, cargo fmt, strict Clippy, and the bounded IDOR scan. CI remains authoritative.",
        "[INFO]".blue()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
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
            install_git_hooks_at(&worktree.path),
            Err(HookInstallError::NotGitWorktree(_))
        ));
        assert!(!worktree.path.join(".git").exists());
    }

    #[test]
    fn installation_is_idempotent_and_executable() {
        let worktree = TempWorktree::new("idempotent");
        let hooks = worktree.init();
        install_git_hooks_at(&worktree.path).expect("first hook installation");
        let pre_commit = hooks.join("pre-commit");
        let first = fs::read(&pre_commit).expect("first managed hook");
        install_git_hooks_at(&worktree.path).expect("idempotent hook installation");
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

        install_git_hooks_at(&worktree.path).expect("preserving hook installation");
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
        install_git_hooks_at(&worktree.path).expect("idempotent preserved hook installation");
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
            install_git_hooks_at(&worktree.path),
            Err(HookInstallError::BackupConflict { .. })
        ));
        assert_eq!(
            fs::read_to_string(&pre_commit).expect("untouched custom hook"),
            "custom"
        );
        assert!(!hooks.join("commit-msg").exists());
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

        let hooks = install_git_hooks_at(&worktree.path).expect("linked hook installation");
        assert_eq!(hooks, linked.join("../..").join("hooks"));
        assert!(hooks.join("pre-commit").exists());
    }
}
