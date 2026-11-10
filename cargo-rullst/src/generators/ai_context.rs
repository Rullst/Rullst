//! Bounded, deterministic project inventory without embedded source or secrets.
use super::consumer_files::{self, Edit};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Serialize;
use std::{fs, path::Path};

mod metadata;
mod render;
mod scan;
#[cfg(test)]
mod tests;

const SCHEMA: &str = "rullst.project-context.v1";
const TEXT_MARKER: &str = "<!-- rullst.project-context.v1; generated inventory -->\n";
const LEGACY_MARKER: &str = "# Rullst Project Context\n\nThis file provides context for LLMs";
const MAX_OUTPUT: usize = 256 * 1024;
/// The generated files, which Git for Windows would otherwise check out
/// with CRLF line endings (`core.autocrlf=true`).
const LF_OUTPUTS: [&str; 2] = [".llms.txt", ".rullst/context-map.json"];
const ATTRIBUTES_COMMENT: &str =
    "# cargo rullst generate:ai-context output: LF on every checkout, so --check matches";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextError {
    #[error("project context requires a valid bounded Cargo manifest")]
    Manifest,
    #[error("project context configuration keys or structure are invalid")]
    Configuration,
    #[error("project context exceeds its file, depth, entry, read or output budget")]
    Limit,
    #[error("project context refuses a link, special file or unrecognized generated output")]
    UnsafePath,
    /// An output path that is not a recognized generated file.
    #[error("project context refuses `{path}`: {reason}")]
    Refused { path: String, reason: &'static str },
    #[error("project context is missing, modified or stale; regenerate it")]
    Stale,
    #[error("project context input changed while being inspected")]
    Changed,
    #[error("project context filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("project context could not be encoded")]
    Encoding,
}

#[derive(Serialize)]
struct ProjectMap {
    schema: &'static str,
    generator_version: &'static str,
    project: Option<String>,
    dependencies: Vec<String>,
    declared_features: Vec<String>,
    dependency_features: std::collections::BTreeMap<String, Vec<String>>,
    dependency_requirements: std::collections::BTreeMap<String, Vec<String>>,
    source_scope: &'static str,
    configuration_keys: std::collections::BTreeMap<String, Vec<String>>,
    files: Vec<scan::SourceFile>,
    excluded_entries: usize,
    input_sha256: String,
    exclusions: &'static [&'static str],
}

pub(crate) fn command(command: Command) -> Command {
    command
        .about("Generate a bounded project map and preserve project instructions")
        .arg(
            Arg::new("check")
                .long("check")
                .action(ArgAction::SetTrue)
                .help(
                    "Fail if the generated inventory is missing, altered or stale; write nothing",
                ),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), ContextError> {
    if matches.get_flag("check") {
        check_ai_context(None)?;
        println!("Project context matches the current inventoried inputs.");
    } else {
        let attributes = generate(Path::new("."))?;
        println!(
            "Project map written to .llms.txt and .rullst/context-map.json; existing AGENTS.md preserved.{}",
            attributes_note(attributes)
        );
    }
    Ok(())
}

fn attributes_note(added: bool) -> &'static str {
    if added {
        "\n.gitattributes now keeps both files LF on every checkout."
    } else {
        ""
    }
}

/// Generates a bounded inventory; existing project instructions are preserved.
pub fn generate_ai_context(base_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let attributes = generate(base_path.unwrap_or_else(|| Path::new(".")))?;
    println!(
        "Project context generated (.llms.txt and .rullst/context-map.json).{}",
        attributes_note(attributes)
    );
    Ok(())
}

/// [`generate_ai_context`] without progress output (used by previews).
pub(crate) fn write_project_context(base_path: &Path) -> Result<(), ContextError> {
    generate(base_path).map(|_| ())
}

pub(crate) fn refresh_after_scaffold() {
    if let Err(error) = generate_ai_context(None) {
        eprintln!("Scaffold completed, but project context was not refreshed: {error}");
    }
}

/// Verifies current input identity and both output representations without writes.
pub fn check_ai_context(base_path: Option<&Path>) -> Result<(), ContextError> {
    let root = fs::canonicalize(base_path.unwrap_or_else(|| Path::new(".")))?;
    let (_, json, text) = outputs(&root)?;
    for (name, expected) in [(".llms.txt", text), (".rullst/context-map.json", json)] {
        match scan::read_output(&root.join(name), name)? {
            Some(current) if lf(&current) == expected => {}
            _ => return Err(ContextError::Stale),
        }
    }
    Ok(())
}

/// `text` with CRLF line endings turned into LF, as Git for Windows checks
/// out text files with `core.autocrlf=true`.
fn lf(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains("\r\n") {
        text.replace("\r\n", "\n").into()
    } else {
        text.into()
    }
}

/// Returns whether `.gitattributes` gained the LF entries.
fn generate(base: &Path) -> Result<bool, ContextError> {
    // Canonicalize the selected root once (including macOS's /var alias), then
    // reject links beneath it. The caller owns this trusted workspace directory.
    let root = fs::canonicalize(base)?;
    let (_, json, text) = outputs(&root)?;
    let mut edits = Vec::new();
    for (name, content) in [(".llms.txt", text), (".rullst/context-map.json", json)] {
        let path = root.join(name);
        match scan::read_output(&path, name)? {
            Some(old) => {
                let current = lf(&old);
                let recognized = if name == ".llms.txt" {
                    current.starts_with(TEXT_MARKER) || current.starts_with(LEGACY_MARKER)
                } else {
                    serde_json::from_str::<serde_json::Value>(&current)
                        .ok()
                        .is_some_and(|map| map["schema"] == SCHEMA)
                };
                if !recognized {
                    return Err(ContextError::Refused {
                        path: name.to_string(),
                        reason: if name == ".llms.txt" {
                            "it does not start with the generated-inventory marker, so it may be your own file; review it, move it aside and run the command again"
                        } else {
                            "it is not a `rullst.project-context.v1` map, so it may be your own file; review it, move it aside and run the command again"
                        },
                    });
                }
                // A file that differs only in line endings is current.
                if current != content {
                    edits.push(Edit::replace(path, old, content));
                }
            }
            None => edits.push(Edit::create(path, content)?),
        }
    }
    let attributes = if edits.is_empty() {
        None
    } else {
        line_endings(&root)?
    };
    let added = attributes.is_some();
    edits.extend(attributes);
    // Instructions are user-owned after creation, even if they retain our marker.
    // Do not read their contents or follow an existing symlink.
    match fs::symlink_metadata(root.join("AGENTS.md")) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            edits.push(Edit::create(
                root.join("AGENTS.md"),
                render::instructions().into(),
            )?);
        }
        Err(error) => return Err(error.into()),
    }
    consumer_files::apply(&edits)?;
    Ok(added)
}

/// The `.gitattributes` edit that adds `<output> text eol=lf` for each
/// generated file it does not name yet. Existing lines, including a
/// different choice for these files, are kept; the entries are appended.
fn line_endings(root: &Path) -> Result<Option<Edit>, ContextError> {
    let path = root.join(".gitattributes");
    let existing = scan::read_output(&path, ".gitattributes")?;
    let named = |output: &str| {
        existing.as_deref().is_some_and(|text| {
            text.lines().any(|line| {
                line.split_whitespace()
                    .next()
                    .is_some_and(|pattern| pattern.trim_start_matches('/') == output)
            })
        })
    };
    let missing = LF_OUTPUTS
        .iter()
        .filter(|output| !named(output))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(None);
    }
    let newline = match &existing {
        Some(text) if text.contains("\r\n") => "\r\n",
        _ => "\n",
    };
    let mut addition = format!("{ATTRIBUTES_COMMENT}{newline}");
    for output in missing {
        addition.push_str(&format!("{output} text eol=lf{newline}"));
    }
    Ok(Some(match existing {
        Some(old) => {
            let mut new = old.clone();
            if !new.is_empty() {
                if !new.ends_with('\n') {
                    new.push_str(newline);
                }
                new.push_str(newline);
            }
            new.push_str(&addition);
            Edit::replace(path, old, new)
        }
        None => Edit::create(path, addition)?,
    }))
}

fn outputs(root: &Path) -> Result<(ProjectMap, String, String), ContextError> {
    let map = scan::collect(root)?;
    let json = serde_json::to_string_pretty(&map).map_err(|_| ContextError::Encoding)? + "\n";
    let text = render::text(&json);
    if json.len() > MAX_OUTPUT || text.len() > MAX_OUTPUT {
        return Err(ContextError::Limit);
    }
    Ok((map, json, text))
}
