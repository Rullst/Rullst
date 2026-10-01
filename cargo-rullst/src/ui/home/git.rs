//! Reads the current Git branch from `HEAD` without starting a `git` process.

use std::path::{Path, PathBuf};

const ANCESTOR_LIMIT: usize = 64;
const HEAD_LIMIT: u64 = 4096;

/// The `HEAD` file of the repository containing `start`, following a linked
/// worktree's `.git` file (`gitdir: <path>`).
fn head_path(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .take(ANCESTOR_LIMIT)
        .find_map(|directory| {
            let dot_git = directory.join(".git");
            let metadata = std::fs::metadata(&dot_git).ok()?;
            if metadata.is_dir() {
                return Some(dot_git.join("HEAD"));
            }
            let pointer = super::project::read_small_file(&dot_git, HEAD_LIMIT).ok()??;
            let target = pointer.trim().strip_prefix("gitdir:")?.trim();
            let target = Path::new(target);
            let git_dir = if target.is_absolute() {
                target.to_path_buf()
            } else {
                directory.join(target)
            };
            Some(git_dir.join("HEAD"))
        })
}

/// Interprets `HEAD` content: a branch name, another symbolic reference, or a
/// detached commit abbreviated to seven characters.
pub(super) fn describe_head(head: &str) -> Option<String> {
    let head = head.trim();
    if let Some(reference) = head.strip_prefix("ref:") {
        let reference = reference.trim();
        let name = reference
            .strip_prefix("refs/heads/")
            .or_else(|| reference.strip_prefix("refs/"))
            .unwrap_or(reference);
        return (!name.is_empty()).then(|| super::project::display_safe(name));
    }
    let is_object_id = matches!(head.len(), 40 | 64) && head.chars().all(|c| c.is_ascii_hexdigit());
    head.get(..7)
        .filter(|_| is_object_id)
        .map(|short| format!("detached at {short}"))
}

/// The branch of the repository that contains `root`, if any.
pub(super) fn branch(root: &Path) -> Option<String> {
    let head = super::project::read_small_file(&head_path(root)?, HEAD_LIMIT).ok()??;
    describe_head(&head)
}
