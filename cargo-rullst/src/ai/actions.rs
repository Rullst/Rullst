//! Turns parsed proposals into validated, previewable operations and applies
//! them. Paths go through [`super::paths`], commands through
//! [`super::commands`]; nothing here trusts the model's text.

use super::commands::{self, Invocation};
use super::diff;
use super::paths::{self, ProjectPath};
use super::protocol::Action;
use super::term::{Style, sanitize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Largest existing file the assistant may replace or edit.
const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// The snapshot `cargo rullst inspect schema` prints when it exists.
const SCHEMA_SNAPSHOT: &str = "rullst-schema.json";

/// Planned file contents for a plan that is shown but not executed, so later
/// previews in the same plan build on earlier ones.
pub(super) type Overlay = HashMap<PathBuf, String>;

/// A validated operation ready for review.
pub(super) enum Prepared {
    File {
        target: ProjectPath,
        old: Option<String>,
        new: String,
    },
    Command(Invocation),
    /// `cargo rullst new`, run in `parent` to create `directory`.
    NewProject {
        invocation: Invocation,
        parent: PathBuf,
        directory: PathBuf,
    },
}

impl Prepared {
    /// Whether applying it changes project files.
    pub(super) fn mutates(&self) -> bool {
        match self {
            Self::File { .. } => true,
            Self::Command(invocation) => invocation.mutates(),
            Self::NewProject { .. } => false,
        }
    }

    /// Confirmed on its own even after "all" for the turn.
    pub(super) fn always_confirm(&self) -> bool {
        match self {
            Self::File { .. } => false,
            Self::Command(invocation) => invocation.always_confirm(),
            Self::NewProject { .. } => true,
        }
    }

    pub(super) fn is_check(&self) -> bool {
        matches!(self, Self::Command(invocation) if invocation.kind == commands::CommandKind::Cargo)
    }

    /// A one-line summary used in results sent back to the model.
    pub(super) fn summary(&self) -> String {
        match self {
            Self::File {
                target, old: None, ..
            } => format!("create {}", target.display),
            Self::File { target, .. } => format!("update {}", target.display),
            Self::Command(invocation) | Self::NewProject { invocation, .. } => invocation.display(),
        }
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "the file could not be read")?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err("the existing file is larger than 1 MiB".to_string());
    }
    let bytes = std::fs::read(path).map_err(|_| "the file could not be read")?;
    String::from_utf8(bytes).map_err(|_| "the existing file is not UTF-8 text".to_string())
}

fn current(target: &ProjectPath, overlay: &Overlay) -> Result<Option<String>, String> {
    if let Some(planned) = overlay.get(&target.absolute) {
        return Ok(Some(planned.clone()));
    }
    if target.exists {
        read_text(&target.absolute).map(Some)
    } else {
        Ok(None)
    }
}

/// `cargo rullst new` outside a project, creating a directory in `cwd` that
/// must not exist yet.
fn prepare_new(args: &[String], root: Option<&Path>, cwd: &Path) -> Result<Prepared, String> {
    if root.is_some() {
        return Err("`new` is only available outside a project".to_string());
    }
    let (invocation, name) =
        commands::validate_new(args.to_vec()).map_err(|error| error.to_string())?;
    let directory = cwd.join(&name);
    if std::fs::symlink_metadata(&directory).is_ok() {
        return Err(format!("`{name}` already exists in the current directory"));
    }
    Ok(Prepared::NewProject {
        invocation,
        parent: cwd.to_path_buf(),
        directory,
    })
}

/// Validates one proposal against the project root (or, for `new`, the
/// current directory outside a project).
pub(super) fn prepare(
    action: &Action,
    root: Option<&Path>,
    cwd: &Path,
    overlay: &Overlay,
) -> Result<Prepared, String> {
    if let Action::RunRullst { args } = action
        && args.first().map(String::as_str) == Some("new")
    {
        return prepare_new(args, root, cwd);
    }
    let root = root.ok_or(
        "file and command actions are unavailable outside a Rust project; propose `new` first",
    )?;
    match action {
        Action::WriteFile { path, content } => {
            let target = paths::resolve(root, path).map_err(|error| error.to_string())?;
            let old = current(&target, overlay)?;
            Ok(Prepared::File {
                target,
                old,
                new: content.clone(),
            })
        }
        Action::EditFile {
            path,
            find,
            replace,
        } => {
            let target = paths::resolve(root, path).map_err(|error| error.to_string())?;
            let old = current(&target, overlay)?
                .ok_or("the file does not exist; use write_file to create it")?;
            let new = match old.matches(find.as_str()).count() {
                0 => return Err("the `find` text does not occur in the file".to_string()),
                1 => old.replacen(find.as_str(), replace, 1),
                count => {
                    return Err(format!(
                        "the `find` text occurs {count} times; include more surrounding text"
                    ));
                }
            };
            if new.len() as u64 > MAX_FILE_BYTES {
                return Err("the edited file would exceed 1 MiB".to_string());
            }
            Ok(Prepared::File {
                target,
                old: Some(old),
                new,
            })
        }
        Action::RunRullst { args } => {
            let invocation =
                commands::validate_rullst(args.clone()).map_err(|error| error.to_string())?;
            for (kind, value) in commands::path_values(args) {
                let checked = match kind {
                    commands::PathKind::File => paths::resolve(root, value),
                    commands::PathKind::Directory => paths::resolve_directory(root, value),
                };
                checked.map_err(|error| format!("`{}`: {error}", sanitize(value)))?;
            }
            match args.first().map(String::as_str) {
                Some("db:migrate") => super::environment::ensure_migration_allowed(root)?,
                // `inspect schema` prints a project-provided snapshot in full.
                Some("inspect") if args.get(1).map(String::as_str) == Some("schema") => {
                    paths::resolve(root, SCHEMA_SNAPSHOT).map_err(|error| error.to_string())?;
                }
                _ => {}
            }
            Ok(Prepared::Command(invocation))
        }
        Action::Cargo { args } => commands::validate_cargo(args.clone())
            .map(Prepared::Command)
            .map_err(|error| error.to_string()),
    }
}

/// Renders the review block for one operation.
pub(super) fn preview(prepared: &Prepared, index: usize, total: usize, style: Style) -> String {
    let mut output = String::new();
    match prepared {
        Prepared::File { target, old, new } => {
            let (diff, stats) = diff::render(old.as_deref().unwrap_or(""), new, style);
            let verb = if old.is_some() { "update" } else { "create" };
            output.push_str(&style.bold(&format!(
                "[{index}/{total}] {verb} {} (+{} -{})",
                sanitize(&target.display),
                stats.added,
                stats.removed
            )));
            output.push('\n');
            let runs =
                |text: &str| target.display.ends_with(".rs") && paths::runs_during_cargo(text);
            if target.sensitive || runs(new) || old.as_deref().is_some_and(runs) {
                output.push_str(&style.yellow(
                    "  ! build configuration, a test or code that runs during `cargo check` or `cargo test`; review carefully",
                ));
                output.push('\n');
            }
            if old.as_deref() == Some(new.as_str()) {
                output.push_str(&style.dim("  (no change)"));
                output.push('\n');
            }
            output.push_str(&diff);
        }
        Prepared::Command(invocation) => {
            output.push_str(&style.bold(&format!("[{index}/{total}] run")));
            output.push('\n');
            output.push_str(&style.cyan(&format!("  $ {}", invocation.display())));
            output.push('\n');
            if invocation.always_confirm() {
                output.push_str(&style.yellow(
                    "  ! changes the development or test database; a git checkpoint cannot undo it",
                ));
                output.push('\n');
            } else if invocation.mutates() {
                output.push_str(&style.dim("  (may create or change project files)"));
                output.push('\n');
            }
        }
        Prepared::NewProject {
            invocation,
            directory,
            ..
        } => {
            output.push_str(&style.bold(&format!(
                "[{index}/{total}] create project {}",
                sanitize(&directory.display().to_string())
            )));
            output.push('\n');
            output.push_str(&style.cyan(&format!("  $ {}", invocation.display())));
            output.push('\n');
            output.push_str(&style.dim("  (the session continues inside the new project)"));
            output.push('\n');
        }
    }
    output
}

/// Records a planned file change in the overlay.
pub(super) fn plan(prepared: &Prepared, overlay: &mut Overlay) {
    if let Prepared::File { target, new, .. } = prepared {
        overlay.insert(target.absolute.clone(), new.clone());
    }
}

/// Atomically writes `content`, creating missing parent directories. The
/// path is resolved again first, so a link created since review is refused.
fn write_file(root: &Path, target: &ProjectPath, content: &str) -> Result<(), String> {
    let fresh = paths::resolve(root, &target.display).map_err(|error| error.to_string())?;
    let parent = fresh
        .absolute
        .parent()
        .ok_or("the path has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "the parent directory could not be created")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".rullst-ai-")
        .tempfile_in(parent)
        .map_err(|_| "a temporary file could not be created")?;
    temporary
        .write_all(content.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| "the file could not be written")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&fresh.absolute)
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or(0o644);
        std::fs::set_permissions(temporary.path(), std::fs::Permissions::from_mode(mode))
            .map_err(|_| "file permissions could not be set")?;
    }
    temporary
        .persist(&fresh.absolute)
        .map_err(|_| "the file could not be replaced")?;
    Ok(())
}

/// The outcome of applying one operation.
pub(super) struct Applied {
    pub success: bool,
    /// Short result for the model (command output is attached separately).
    pub result: String,
    pub output: Option<String>,
    /// The project a successful `new` created.
    pub new_root: Option<PathBuf>,
}

fn failed(message: String) -> Applied {
    Applied {
        success: false,
        result: message,
        output: None,
        new_root: None,
    }
}

/// Runs a command in `directory`, streaming at most 400 output lines.
fn run_command<W: Write>(
    invocation: &Invocation,
    directory: &Path,
    out: &mut W,
    style: Style,
) -> Applied {
    let mut shown = 0usize;
    let result = super::process::run(invocation, directory, |line| {
        shown += 1;
        if shown <= 400 {
            let _ = writeln!(out, "  {}", style.dim(&sanitize(line)));
        } else if shown == 401 {
            let _ = writeln!(out, "  {}", style.dim("… further output hidden"));
        }
    });
    match result {
        Ok(outcome) => {
            let mark = if outcome.success {
                style.green(&format!("✓ {}", outcome.status))
            } else {
                style.red(&format!("✗ {}", outcome.status))
            };
            let _ = writeln!(out, "  {mark}");
            Applied {
                success: outcome.success,
                result: outcome.status,
                output: Some(outcome.output),
                new_root: None,
            }
        }
        Err(error) => {
            let message = format!("could not start: {error}");
            let _ = writeln!(out, "  {}", style.red(&format!("✗ {message}")));
            failed(message)
        }
    }
}

/// Applies an operation, streaming command output to `out`. File changes
/// and project commands need the project `root`.
pub(super) fn apply<W: Write>(
    prepared: &Prepared,
    root: Option<&Path>,
    out: &mut W,
    style: Style,
) -> Applied {
    let needs_root = || failed("no project is open".to_string());
    match prepared {
        Prepared::File { target, old, new } => {
            let Some(root) = root else {
                return needs_root();
            };
            match write_file(root, target, new) {
                Ok(()) => {
                    let lines = new.lines().count();
                    let verb = if old.is_some() { "updated" } else { "created" };
                    let _ = writeln!(
                        out,
                        "  {}",
                        style.green(&format!("✓ {verb} {}", target.display))
                    );
                    Applied {
                        success: true,
                        result: format!("{verb} ({lines} lines)"),
                        output: None,
                        new_root: None,
                    }
                }
                Err(error) => {
                    let _ = writeln!(out, "  {}", style.red(&format!("✗ {error}")));
                    failed(format!("failed: {error}"))
                }
            }
        }
        Prepared::Command(invocation) => match root {
            Some(root) => run_command(invocation, root, out, style),
            None => needs_root(),
        },
        Prepared::NewProject {
            invocation,
            parent,
            directory,
        } => {
            let mut applied = run_command(invocation, parent, out, style);
            let created = std::fs::symlink_metadata(directory.join("Cargo.toml"))
                .is_ok_and(|metadata| metadata.is_file());
            if applied.success && created {
                applied.new_root = std::fs::canonicalize(directory).ok();
            } else if applied.success {
                applied.success = false;
                applied.result = "the command succeeded but created no Cargo.toml".to_string();
            }
            applied
        }
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
