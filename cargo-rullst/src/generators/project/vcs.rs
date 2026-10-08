// cargo-rullst/src/generators/project/vcs.rs — `cargo rullst new --vcs`: initialise a Git
// repository like `cargo new`, so the committed-secrets audit has a work tree to inspect.

use std::path::Path;
use std::process::{Command, Stdio};

/// The version control `cargo rullst new` sets up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Vcs {
    /// `git init` unless the destination is already inside a Git work tree.
    #[default]
    Git,
    /// No repository.
    None,
}

impl Vcs {
    /// The `--vcs` values, as clap lists them.
    pub(crate) const VALUES: [&'static str; 2] = ["git", "none"];

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "git" => Some(Self::Git),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// What [`initialize`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VcsOutcome {
    Initialized,
    /// The destination already belongs to an enclosing repository.
    InsideWorkTree,
    GitUnavailable,
    Failed(String),
    Disabled,
}

impl VcsOutcome {
    /// The line printed after the project files are written, if any.
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Self::Initialized => Some(
                "  ✅ Initialized a Git repository (.gitignore keeps .env and target/ out)."
                    .to_string(),
            ),
            Self::InsideWorkTree => Some(
                "  ℹ️ The project is inside an existing Git work tree; skipped `git init`."
                    .to_string(),
            ),
            Self::GitUnavailable => Some(
                "  ⚠️ Git was not found, so no repository was created. Install Git and run `git init` \
                 in the project; `cargo rullst audit` checks committed secrets through Git."
                    .to_string(),
            ),
            Self::Failed(reason) => Some(format!(
                "  ⚠️ `git init` failed ({reason}); the project files were kept. Run `git init` in the project."
            )),
            Self::Disabled => None,
        }
    }
}

const GITIGNORE: &str = "# Rust build artifacts\n/target\n\n# Rullst: Environment & Secrets\n.env\n.env.*\n!.env.example\n";

/// Writes a minimal `.gitignore` keeping `.env` and `target/` out when the
/// project has none; an existing file is never changed.
pub(crate) fn ensure_gitignore(project: &Path) -> std::io::Result<bool> {
    let path = project.join(".gitignore");
    if path.exists() {
        return Ok(false);
    }
    std::fs::write(path, GITIGNORE)?;
    Ok(true)
}

fn git() -> Command {
    let mut command = Command::new("git");
    command.stdin(Stdio::null()).stderr(Stdio::piped());
    command
}

/// Runs `git init` in `project` for [`Vcs::Git`], unless `project` is already
/// inside a Git work tree. Failures are reported, never fatal: the project
/// files are complete without a repository.
pub(crate) fn initialize(project: &Path, vcs: Vcs) -> VcsOutcome {
    if vcs == Vcs::None {
        return VcsOutcome::Disabled;
    }
    let inside = git()
        .arg("-C")
        .arg(project)
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::piped())
        .output();
    match inside {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return VcsOutcome::GitUnavailable;
        }
        Err(error) => return VcsOutcome::Failed(error.kind().to_string()),
        Ok(output) if output.status.success() && output.stdout.trim_ascii() == b"true" => {
            return VcsOutcome::InsideWorkTree;
        }
        Ok(_) => {}
    }
    match git()
        .args(["init", "--quiet"])
        .arg(project)
        .stdout(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => VcsOutcome::Initialized,
        Ok(output) => VcsOutcome::Failed(
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .map_or_else(|| output.status.to_string(), str::to_string),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => VcsOutcome::GitUnavailable,
        Err(error) => VcsOutcome::Failed(error.kind().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn values_parse_and_git_is_the_default() {
        assert_eq!(Vcs::default(), Vcs::Git);
        assert_eq!(Vcs::parse("git"), Some(Vcs::Git));
        assert_eq!(Vcs::parse("none"), Some(Vcs::None));
        assert_eq!(Vcs::parse("hg"), None);
        assert!(Vcs::VALUES.iter().all(|value| Vcs::parse(value).is_some()));
        assert_eq!(VcsOutcome::Disabled.message(), None);
    }

    #[test]
    fn git_init_runs_once_and_never_nests_or_runs_for_none() {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("app");
        std::fs::create_dir_all(&project).unwrap();
        assert_eq!(initialize(&project, Vcs::None), VcsOutcome::Disabled);
        assert!(!project.join(".git").exists());
        if !git_available() {
            assert_eq!(initialize(&project, Vcs::Git), VcsOutcome::GitUnavailable);
            return;
        }
        let first = initialize(&project, Vcs::Git);
        if first == VcsOutcome::InsideWorkTree {
            // The temporary directory itself is inside a repository here.
            return;
        }
        assert_eq!(first, VcsOutcome::Initialized);
        assert!(project.join(".git").is_dir());

        let nested = project.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(initialize(&nested, Vcs::Git), VcsOutcome::InsideWorkTree);
        assert!(!nested.join(".git").exists());
    }

    #[test]
    fn a_gitignore_is_written_only_when_missing() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(ensure_gitignore(temporary.path()).unwrap());
        let written = std::fs::read_to_string(temporary.path().join(".gitignore")).unwrap();
        assert!(written.lines().any(|line| line == ".env"));
        assert!(written.lines().any(|line| line == "/target"));

        std::fs::write(temporary.path().join(".gitignore"), "custom\n").unwrap();
        assert!(!ensure_gitignore(temporary.path()).unwrap());
        assert_eq!(
            std::fs::read_to_string(temporary.path().join(".gitignore")).unwrap(),
            "custom\n"
        );
    }
}
