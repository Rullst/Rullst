//! What `cargo rullst ui add` would write: each component's files, the
//! `pub mod` lines of `src/ui/mod.rs` and the `mod ui;` declaration of the
//! crate root. A file that exists with other content is a conflict unless
//! `--force` is given; nothing is written while one remains.

use super::catalog::Component;
use std::io::ErrorKind;
use std::path::Path;

/// `src/ui/mod.rs` when the command creates it.
pub(crate) const MODULE_HEADER: &str =
    "//! UI components copied by `cargo rullst ui add`. The application owns these
//! files: change them as needed. Running `cargo rullst ui add` again leaves a
//! changed file alone unless `--force` is given.
//! Guide: https://rullst.github.io/Rullst/book/frontend.html#components

// A component's helpers are kept even when the application uses some of them.
#![allow(dead_code)]
";

pub(crate) const MODULE_PATH: &str = "src/ui/mod.rs";

/// One file the command creates or changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    pub path: String,
    /// `None` when the file is created.
    pub before: Option<String>,
    pub after: String,
    pub summary: String,
}

/// Everything one invocation would do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub changes: Vec<Change>,
    /// Files that already hold this CLI's copy.
    pub current: Vec<String>,
    /// Files whose content differs from this CLI's copy (refused without
    /// `--force`, replaced with it).
    pub modified: Vec<String>,
    /// Wiring the command could not do itself, with the line to add.
    pub manual: Vec<(String, String)>,
}

/// Why planning or applying stopped.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlanError {
    #[error("could not read {file}: {kind}")]
    Read { file: String, kind: ErrorKind },
    #[error("could not write {file}: {kind}")]
    Write { file: String, kind: ErrorKind },
}

/// Reads a project file; a missing file is `None`.
fn read(root: &Path, relative: &str) -> Result<Option<String>, PlanError> {
    match std::fs::read_to_string(root.join(relative)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(PlanError::Read {
            file: relative.to_owned(),
            kind: error.kind(),
        }),
    }
}

/// Plans one copied file: created, current, or modified (replaced with
/// `force`).
fn copy(
    root: &Path,
    plan: &mut Plan,
    path: String,
    contents: &str,
    summary: String,
    force: bool,
) -> Result<(), PlanError> {
    match read(root, &path)? {
        None => plan.changes.push(Change {
            path,
            before: None,
            after: contents.to_owned(),
            summary,
        }),
        Some(existing) if existing == contents => plan.current.push(path),
        Some(existing) => {
            if force {
                plan.changes.push(Change {
                    path: path.clone(),
                    before: Some(existing),
                    after: contents.to_owned(),
                    summary: "replace the changed copy (--force)".to_owned(),
                });
            }
            plan.modified.push(path);
        }
    }
    Ok(())
}

/// Whether `source` declares module `name` (`mod name;` or `mod name {}`).
fn declares(source: &str, name: &str) -> Option<bool> {
    let file = syn::parse_file(source).ok()?;
    Some(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == name)),
    )
}

/// `src/ui/mod.rs` with a `pub mod` line for each of `components`.
fn registration(
    root: &Path,
    plan: &mut Plan,
    components: &[&'static Component],
) -> Result<(), PlanError> {
    let Some(before) = read(root, MODULE_PATH)? else {
        let mut after = MODULE_HEADER.to_owned();
        let mut names: Vec<&str> = components.iter().map(|component| component.name).collect();
        names.sort_unstable();
        after.push('\n');
        for name in names {
            after.push_str(&format!("pub mod {name};\n"));
        }
        plan.changes.push(Change {
            path: MODULE_PATH.to_owned(),
            before: None,
            after,
            summary: "declare the components".to_owned(),
        });
        return Ok(());
    };
    let mut after = before.clone();
    let mut added = Vec::new();
    for component in components {
        match declares(&after, component.name) {
            Some(true) => {}
            Some(false) => {
                after = crate::generators::insert_module_declaration(
                    &after,
                    component.name,
                    &format!("pub mod {};", component.name),
                );
                added.push(component.name);
            }
            None => {
                plan.manual.push((
                    format!("{MODULE_PATH} (it does not parse as Rust)"),
                    format!("pub mod {};", component.name),
                ));
            }
        }
    }
    if !added.is_empty() {
        plan.changes.push(Change {
            path: MODULE_PATH.to_owned(),
            before: Some(before),
            after,
            summary: format!("declare {}", added.join(", ")),
        });
    }
    Ok(())
}

/// `mod ui;` in the crate root: `pub mod ui;` in `src/lib.rs` when the
/// project has one (its binary reaches it through the library), else
/// `mod ui;` in `src/main.rs`.
fn crate_root(root: &Path, plan: &mut Plan) -> Result<(), PlanError> {
    let (path, declaration) = if root.join("src/lib.rs").is_file() {
        ("src/lib.rs", "pub mod ui;")
    } else {
        ("src/main.rs", "mod ui;")
    };
    let Some(before) = read(root, path)? else {
        plan.manual.push((
            "the crate root (no src/main.rs or src/lib.rs)".to_owned(),
            declaration.to_owned(),
        ));
        return Ok(());
    };
    match declares(&before, "ui") {
        Some(true) => {}
        Some(false) => {
            let after = crate::generators::insert_module_declaration(
                &before,
                "ui",
                &format!("// UI components (cargo rullst ui add).\n{declaration}"),
            );
            plan.changes.push(Change {
                path: path.to_owned(),
                before: Some(before),
                after,
                summary: format!("declare `{declaration}`"),
            });
        }
        None => plan.manual.push((
            format!("{path} (it does not parse as Rust)"),
            declaration.to_owned(),
        )),
    }
    Ok(())
}

/// The plan for copying `components` into the project at `root`.
pub(crate) fn plan(
    root: &Path,
    components: &[&'static Component],
    force: bool,
) -> Result<Plan, PlanError> {
    let mut plan = Plan::default();
    for component in components {
        copy(
            root,
            &mut plan,
            component.source_path(),
            component.source,
            format!("copy the {} component", component.name),
            force,
        )?;
        if let (Some(path), Some(script)) = (component.script_path(), component.script) {
            copy(
                root,
                &mut plan,
                path,
                script,
                format!("copy its module (loaded by the {} markup)", component.name),
                force,
            )?;
        }
    }
    registration(root, &mut plan, components)?;
    crate_root(root, &mut plan)?;
    Ok(plan)
}

/// Writes every planned change.
pub(crate) fn apply(root: &Path, plan: &Plan) -> Result<(), PlanError> {
    for change in &plan.changes {
        let path = root.join(&change.path);
        let failed = |error: std::io::Error| PlanError::Write {
            file: change.path.clone(),
            kind: error.kind(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        std::fs::write(&path, &change.after).map_err(failed)?;
    }
    Ok(())
}
