//! A git checkpoint taken before the assistant's first change in a session.
//!
//! The snapshot is built in a temporary copy of the index, so the user's
//! index, stash and working tree are untouched: `git add -A` records tracked
//! and untracked (non-ignored) files into the copy, `write-tree` and
//! `commit-tree` turn it into a commit, and `update-ref` stores it under
//! `refs/rullst/ai-checkpoints/<UTC timestamp>`. Such refs are local and are
//! not pushed by default. `.env*` files and `target/` are excluded so secrets
//! and build output never enter the object database through this path.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub(super) const REF_PREFIX: &str = "refs/rullst/ai-checkpoints/";

#[derive(Debug, thiserror::Error)]
pub(super) enum CheckpointError {
    #[error("git is not available")]
    GitMissing,
    #[error("the project is not inside a git repository")]
    NotARepository,
    #[error("git {0} failed")]
    Git(&'static str),
    #[error("checkpoint I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// A stored checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Checkpoint {
    pub reference: String,
    pub commit: String,
}

impl Checkpoint {
    /// Commands that review or restore the checkpoint, run from the project root.
    pub(super) fn restore_hint(&self) -> Vec<String> {
        vec![
            format!("review:  git diff {}", self.reference),
            format!(
                "restore: git restore --source={} --worktree -- .",
                self.reference
            ),
            "         (files created afterwards are not removed; see `git status`)".to_string(),
        ]
    }
}

fn git(directory: &Path, index: Option<&Path>, args: &[&str]) -> Result<Output, CheckpointError> {
    let mut command = Command::new("git");
    command
        .current_dir(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        // A fixed identity: commit-tree must not depend on (or reveal) the
        // user's configuration.
        .env("GIT_AUTHOR_NAME", "Rullst AI checkpoint")
        .env("GIT_AUTHOR_EMAIL", "rullst-ai@localhost")
        .env("GIT_COMMITTER_NAME", "Rullst AI checkpoint")
        .env("GIT_COMMITTER_EMAIL", "rullst-ai@localhost")
        .stdin(Stdio::null());
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    command.output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            CheckpointError::GitMissing
        } else {
            CheckpointError::Io(error)
        }
    })
}

fn stdout_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn checked(output: Output, step: &'static str) -> Result<String, CheckpointError> {
    if output.status.success() {
        Ok(stdout_line(&output))
    } else {
        Err(CheckpointError::Git(step))
    }
}

/// Creates a checkpoint of the work tree containing `root`.
pub(super) fn create(root: &Path, timestamp: &str) -> Result<Checkpoint, CheckpointError> {
    let toplevel = git(root, None, &["rev-parse", "--show-toplevel"])?;
    if !toplevel.status.success() {
        return Err(CheckpointError::NotARepository);
    }
    let toplevel = PathBuf::from(stdout_line(&toplevel));
    let index_path = checked(
        git(&toplevel, None, &["rev-parse", "--git-path", "index"])?,
        "rev-parse --git-path index",
    )?;
    let index_path = {
        let path = PathBuf::from(index_path);
        if path.is_absolute() {
            path
        } else {
            toplevel.join(path)
        }
    };
    let scratch = tempfile::tempdir()?;
    let temporary_index = scratch.path().join("index");
    if index_path.is_file() {
        std::fs::copy(&index_path, &temporary_index)?;
    }
    checked(
        git(
            &toplevel,
            Some(&temporary_index),
            &[
                "add",
                "-A",
                "--",
                ".",
                ":(exclude,glob)**/.env",
                ":(exclude,glob)**/.env.*",
                ":(exclude)target",
            ],
        )?,
        "add",
    )?;
    let tree = checked(
        git(&toplevel, Some(&temporary_index), &["write-tree"])?,
        "write-tree",
    )?;
    let head = git(
        &toplevel,
        None,
        &["rev-parse", "--verify", "-q", "HEAD^{commit}"],
    )?;
    let message = format!("Rullst AI checkpoint {timestamp}");
    let mut args = vec!["commit-tree", tree.as_str(), "-m", message.as_str()];
    let parent = stdout_line(&head);
    if head.status.success() && !parent.is_empty() {
        args.extend(["-p", parent.as_str()]);
    }
    let commit = checked(git(&toplevel, None, &args)?, "commit-tree")?;
    let mut reference = format!("{REF_PREFIX}{timestamp}");
    for suffix in 1..100 {
        let exists = git(
            &toplevel,
            None,
            &["rev-parse", "--verify", "-q", &reference],
        )?;
        if !exists.status.success() {
            break;
        }
        reference = format!("{REF_PREFIX}{timestamp}-{suffix}");
    }
    checked(
        git(&toplevel, None, &["update-ref", &reference, &commit])?,
        "update-ref",
    )?;
    Ok(Checkpoint {
        reference,
        commit: commit.chars().take(12).collect(),
    })
}

/// A UTC timestamp usable in a ref name.
pub(super) fn timestamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
#[path = "tests/checkpoint.rs"]
mod tests;
