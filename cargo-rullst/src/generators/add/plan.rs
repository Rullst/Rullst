//! The file changes of one `cargo rullst add`, computed before anything is
//! written so `--dry-run` and a real run share them.

use super::capability::{Capability, EnvEntry};
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Value};

/// A failure the command explains itself (see `super::friendly`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AddError {
    UnknownCapability(String),
    /// `Cargo.toml` is not valid TOML; the 1-based line of the error.
    InvalidManifest {
        line: Option<usize>,
    },
    /// `rullst` is declared in a form features cannot be added to.
    UnsupportedDependency,
    Read {
        file: &'static str,
        kind: ErrorKind,
    },
    Write {
        file: &'static str,
        kind: ErrorKind,
    },
}

impl fmt::Display for AddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCapability(name) => write!(formatter, "`{name}` is not a capability"),
            Self::InvalidManifest { line: Some(line) } => {
                write!(formatter, "Cargo.toml is not valid TOML (line {line})")
            }
            Self::InvalidManifest { line: None } => {
                formatter.write_str("Cargo.toml is not valid TOML")
            }
            Self::UnsupportedDependency => formatter.write_str(
                "the `rullst` dependency is not a version string, inline table or table",
            ),
            Self::Read { file, kind } => write!(formatter, "could not read {file}: {kind}"),
            Self::Write { file, kind } => write!(formatter, "could not write {file}: {kind}"),
        }
    }
}

impl std::error::Error for AddError {}

/// One file the command creates or changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileChange {
    pub file: &'static str,
    /// `None` when the file is created.
    pub before: Option<String>,
    pub after: String,
    /// What changed, for the summary line.
    pub summary: String,
}

/// Everything `add` would do for one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddPlan {
    pub capability: &'static Capability,
    pub already_enabled: bool,
    pub changes: Vec<FileChange>,
}

/// The directory is not a Rullst project: it has no `Cargo.toml` declaring
/// a `rullst` dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    NotAProject,
}

fn read(root: &Path, file: &'static str) -> Result<Option<String>, AddError> {
    match std::fs::read_to_string(root.join(file)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AddError::Read {
            file,
            kind: error.kind(),
        }),
    }
}

fn parse_manifest(text: &str) -> Result<DocumentMut, AddError> {
    text.parse::<DocumentMut>()
        .map_err(|error| AddError::InvalidManifest {
            line: error
                .span()
                .and_then(|span| text.get(..span.start))
                .map(|prefix| prefix.matches('\n').count() + 1),
        })
}

/// The `[dependencies].rullst` entry, when the manifest declares one.
fn rullst_dependency(document: &DocumentMut) -> Option<&Item> {
    document.get("dependencies")?.as_table_like()?.get("rullst")
}

/// The features listed on the `rullst` dependency.
fn rullst_features(dependency: &Item) -> Result<Vec<String>, AddError> {
    let features = match dependency {
        Item::Value(Value::String(_)) => None,
        Item::Value(Value::InlineTable(table)) => table.get("features").and_then(Value::as_array),
        Item::Table(table) => table.get("features").and_then(Item::as_array),
        _ => return Err(AddError::UnsupportedDependency),
    };
    Ok(features
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

/// Plans `capability` for the project at `root`. `Ok(Err(Scope))` means
/// `root` is not a Rullst project (no `Cargo.toml` with a `rullst` dependency).
pub(crate) fn plan(
    root: &Path,
    capability: &'static Capability,
) -> Result<Result<AddPlan, Scope>, AddError> {
    let Some(manifest) = read(root, "Cargo.toml")? else {
        return Ok(Err(Scope::NotAProject));
    };
    let document = parse_manifest(&manifest)?;
    let Some(dependency) = rullst_dependency(&document) else {
        return Ok(Err(Scope::NotAProject));
    };
    let features = rullst_features(dependency)?;
    let already_enabled = capability.enabled_by(features.iter().map(String::as_str));

    let mut changes = Vec::new();
    if !already_enabled {
        let after =
            crate::generators::chat::ensure_rullst_features(&manifest, &[capability.feature])
                .map_err(|_| AddError::UnsupportedDependency)?;
        changes.push(FileChange {
            file: "Cargo.toml",
            before: Some(manifest),
            after,
            summary: format!("enable the `{}` feature of rullst", capability.feature),
        });
    }
    let example = read(root, ".env.example")?;
    if let Some(change) = env_example_change(capability, example) {
        changes.push(change);
    }
    if let Some(dotenv) = read(root, ".env")?
        && let Some(change) = dotenv_change(capability, dotenv)
    {
        changes.push(change);
    }
    Ok(Ok(AddPlan {
        capability,
        already_enabled,
        changes,
    }))
}

/// Keys assigned in a dotenv file; `with_comments` also counts `# KEY=…`.
pub(crate) fn assigned_keys(text: &str, with_comments: bool) -> Vec<&str> {
    text.lines()
        .filter_map(|line| {
            let mut line = line.trim_start();
            if let Some(rest) = line.strip_prefix('#') {
                if !with_comments {
                    return None;
                }
                line = rest.trim_start();
            }
            let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
            let (key, _) = line.split_once('=')?;
            let key = key.trim_end();
            (!key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
            .then_some(key)
        })
        .collect()
}

fn line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn render_entries(title: &str, capability: &str, entries: &[&EnvEntry], newline: &str) -> String {
    let mut block = format!("# ── {title} (added by `cargo rullst add {capability}`) ──{newline}");
    for entry in entries {
        for note in entry.note {
            block.push_str(&format!("# {note}{newline}"));
        }
        let prefix = if entry.commented { "# " } else { "" };
        block.push_str(&format!("{prefix}{}={}{newline}", entry.key, entry.value));
    }
    block
}

fn append_block(before: &str, block: &str, newline: &str) -> String {
    let mut after = before.to_string();
    if !after.is_empty() {
        if !after.ends_with('\n') {
            after.push_str(newline);
        }
        after.push_str(newline);
    }
    after.push_str(block);
    after
}

fn key_list(entries: &[&EnvEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            if entry.commented {
                format!("{} (commented)", entry.key)
            } else {
                entry.key.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Appends the capability's variables `.env.example` lacks (assigned or
/// commented out), creating the file when it is missing.
pub(crate) fn env_example_change(
    capability: &Capability,
    before: Option<String>,
) -> Option<FileChange> {
    let present = before
        .as_deref()
        .map(|text| assigned_keys(text, true))
        .unwrap_or_default();
    let missing: Vec<&EnvEntry> = capability
        .env
        .iter()
        .filter(|entry| !present.contains(&entry.key))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let newline = before.as_deref().map_or("\n", line_ending);
    let block = render_entries(capability.env_title, capability.name, &missing, newline);
    let after = append_block(before.as_deref().unwrap_or_default(), &block, newline);
    let verb = if before.is_some() {
        "add"
    } else {
        "create with"
    };
    Some(FileChange {
        file: ".env.example",
        summary: format!("{verb} {}", key_list(&missing)),
        before,
        after,
    })
}

/// `.env` changes only when it lacks a variable the application needs to
/// start in development; it then receives the same mock value.
pub(crate) fn dotenv_change(capability: &Capability, before: String) -> Option<FileChange> {
    let present = assigned_keys(&before, false);
    let missing: Vec<&EnvEntry> = capability
        .env
        .iter()
        .filter(|entry| entry.dev_required && !entry.commented && !present.contains(&entry.key))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let newline = line_ending(&before);
    let block = render_entries(capability.env_title, capability.name, &missing, newline);
    Some(FileChange {
        file: ".env",
        summary: format!(
            "add the development mock value of {} (needed to start)",
            key_list(&missing)
        ),
        after: append_block(&before, &block, newline),
        before: Some(before),
    })
}

/// Writes every change; `.env*` first so a failed manifest write can be
/// retried without duplicating entries.
pub(crate) fn apply(root: &Path, plan: &AddPlan) -> Result<Vec<PathBuf>, AddError> {
    let mut written = Vec::new();
    let mut ordered: Vec<&FileChange> = plan.changes.iter().collect();
    ordered.sort_by_key(|change| change.file == "Cargo.toml");
    for change in ordered {
        let path = root.join(change.file);
        std::fs::write(&path, &change.after).map_err(|error| AddError::Write {
            file: change.file,
            kind: error.kind(),
        })?;
        written.push(path);
    }
    Ok(written)
}
