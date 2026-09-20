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
        generate(Path::new("."))?;
        println!(
            "Project map written to .llms.txt and .rullst/context-map.json; existing AGENTS.md preserved."
        );
    }
    Ok(())
}

/// Generates a bounded inventory; existing project instructions are preserved.
pub fn generate_ai_context(base_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    generate(base_path.unwrap_or_else(|| Path::new(".")))?;
    println!("Project context generated (.llms.txt and .rullst/context-map.json).");
    Ok(())
}

/// Verifies current input identity and both output representations without writes.
pub fn check_ai_context(base_path: Option<&Path>) -> Result<(), ContextError> {
    let root = fs::canonicalize(base_path.unwrap_or_else(|| Path::new(".")))?;
    let (_, json, text) = outputs(&root)?;
    for (name, expected) in [(".llms.txt", text), (".rullst/context-map.json", json)] {
        match scan::read_output(&root.join(name))? {
            Some(current) if current == expected => {}
            _ => return Err(ContextError::Stale),
        }
    }
    Ok(())
}

fn generate(base: &Path) -> Result<(), ContextError> {
    // Canonicalize the selected root once (including macOS's /var alias), then
    // reject links beneath it. The caller owns this trusted workspace directory.
    let root = fs::canonicalize(base)?;
    let (_, json, text) = outputs(&root)?;
    let mut edits = Vec::new();
    for (name, content) in [(".llms.txt", text), (".rullst/context-map.json", json)] {
        let path = root.join(name);
        match scan::read_output(&path)? {
            Some(old) => {
                let recognized = if name == ".llms.txt" {
                    old.starts_with(TEXT_MARKER) || old.starts_with(LEGACY_MARKER)
                } else {
                    serde_json::from_str::<serde_json::Value>(&old)
                        .ok()
                        .is_some_and(|map| map["schema"] == SCHEMA)
                };
                if !recognized {
                    return Err(ContextError::UnsafePath);
                }
                if old != content {
                    edits.push(Edit::replace(path, old, content));
                }
            }
            None => edits.push(Edit::create(path, content)?),
        }
    }
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
    Ok(())
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
