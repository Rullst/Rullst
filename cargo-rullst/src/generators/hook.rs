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
if ! cargo clippy --workspace --all-features --all-targets -- -D warnings; then
    echo "❌ Clippy warnings detected. Fix all warnings before committing."
    exit 1
fi

echo "  🛡️ Running Rullst bounded unsafe and IDOR / BOLA source audit..."
if [ -f "cargo-rullst/Cargo.toml" ]; then
    cargo run --quiet -p cargo-rullst --bin rullst -- audit --idor
else
    cargo rullst audit --idor
fi
if [ $? -ne 0 ]; then
    echo "❌ Source security check failed. Remove undocumented unsafe code and classify parameterized routes with enforced owner, role, or admin boundaries."
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
# Subjects Git itself writes for merges, reverts and autosquash commits.
git_generated_regex='^(Merge (branch|branches|remote-tracking branch|remote-tracking branches|tag|tags|pull request|commit) |Revert "|(fixup|squash|amend)! )'
commit_message=$(head -n 1 "$1")

if echo "$commit_message" | grep -qE "$git_generated_regex"; then
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
    let git_dir_target = metadata_target(&metadata)?;
    let git_dir = git_dir_target
        .canonicalize()
        .map_err(|error| io_error("resolve Git directory", &git_dir_target, error))?;
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
    let common = common
        .canonicalize()
        .map_err(|error| io_error("resolve Git common directory", &common, error))?;
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

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), HookInstallError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata =
        fs::metadata(path).map_err(|error| io_error("read hook permissions", path, error))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| io_error("set executable hook permissions", path, error))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), HookInstallError> {
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

/// Returns the effective `core.hooksPath` (local, global or system Git
/// configuration), resolved against the worktree like Git does.
///
/// `None` when it is unset or Git cannot be run; installation then uses the
/// metadata-derived hooks directory.
fn configured_hooks_path(worktree: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(["config", "--path", "--get", "core.hooksPath"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim_end_matches(['\n', '\r']);
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    Some(if path.is_absolute() {
        path
    } else {
        worktree.join(path)
    })
}

fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn install_git_hooks_at(start: &Path) -> Result<PathBuf, HookInstallError> {
    install_git_hooks_with(start, configured_hooks_path)
}

fn install_git_hooks_with(
    start: &Path,
    hooks_path: impl FnOnce(&Path) -> Option<PathBuf>,
) -> Result<PathBuf, HookInstallError> {
    let worktree = find_worktree(start)?;
    let hooks_dir = hooks_directory(&worktree)?;
    // Git runs hooks only from core.hooksPath when it is set (Husky, shared
    // team hooks), so wrappers in the default directory would never execute.
    if let Some(configured) = hooks_path(&worktree)
        && !same_directory(&configured, &hooks_dir)
    {
        return Err(HookInstallError::Io {
            operation: "install managed hooks because core.hooksPath selects",
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "Git runs only the hooks in that directory, so wrappers in `{}` would never execute; unset core.hooksPath or invoke cargo fmt, Clippy and `cargo rullst audit --idor` from that hook manager",
                    hooks_dir.display()
                ),
            ),
            path: configured,
        });
    }
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
#[path = "hook_tests.rs"]
mod tests;
