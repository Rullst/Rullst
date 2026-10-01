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
}

impl Prepared {
    /// Whether applying it changes project files.
    pub(super) fn mutates(&self) -> bool {
        match self {
            Self::File { .. } => true,
            Self::Command(invocation) => invocation.mutates(),
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
            Self::Command(invocation) => invocation.display(),
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

/// Validates one proposal against the project root.
pub(super) fn prepare(
    action: &Action,
    root: Option<&Path>,
    overlay: &Overlay,
) -> Result<Prepared, String> {
    let root = root.ok_or("file and command actions are unavailable outside a Rust project")?;
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
        Action::RunRullst { args } => commands::validate_rullst(args.clone())
            .map(Prepared::Command)
            .map_err(|error| error.to_string()),
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
            if target.sensitive {
                output.push_str(&style.yellow(
                    "  ! build configuration or code that runs during `cargo check`; review carefully",
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
            if invocation.mutates() {
                output.push_str(&style.dim("  (may create or change project files)"));
                output.push('\n');
            }
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
}

/// Applies an operation, streaming command output to `out`.
pub(super) fn apply<W: Write>(
    prepared: &Prepared,
    root: &Path,
    out: &mut W,
    style: Style,
) -> Applied {
    match prepared {
        Prepared::File { target, old, new } => match write_file(root, target, new) {
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
                }
            }
            Err(error) => {
                let _ = writeln!(out, "  {}", style.red(&format!("✗ {error}")));
                Applied {
                    success: false,
                    result: format!("failed: {error}"),
                    output: None,
                }
            }
        },
        Prepared::Command(invocation) => {
            let mut shown = 0usize;
            let result = commands::run(invocation, root, |line| {
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
                    }
                }
                Err(error) => {
                    let message = format!("could not start: {error}");
                    let _ = writeln!(out, "  {}", style.red(&format!("✗ {message}")));
                    Applied {
                        success: false,
                        result: message,
                        output: None,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
