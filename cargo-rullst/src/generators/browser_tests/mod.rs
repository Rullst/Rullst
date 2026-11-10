//! `cargo rullst add browser-tests` (13.0.0-alpha.5): pinned Playwright and
//! axe-core checks that open every page of the generated `PAGES` list in
//! Chromium, Firefox and WebKit, and a GitHub workflow that runs them against
//! the started application.
//!
//! Like the other `add` capabilities it never rewrites a file: a file that
//! already differs from the generated one is kept and reported, and running
//! the command again changes nothing.

use crate::ui::style::{self, Style};
use std::io::ErrorKind;
use std::path::Path;

/// The `cargo rullst add` name.
pub(crate) const NAME: &str = "browser-tests";
/// One line for `cargo rullst add --help`.
pub(crate) const SUMMARY: &str =
    "Playwright and axe-core checks of every page in Chromium, Firefox and WebKit";

/// Each generated file and its content, in the order they are reported.
pub(crate) const FILES: [(&str, &str); 5] = [
    (
        "tests/browser/package.json",
        include_str!("assets/package.json"),
    ),
    (
        "tests/browser/package-lock.json",
        include_str!("assets/package-lock.json"),
    ),
    ("tests/browser/pages.mjs", include_str!("assets/pages.mjs")),
    ("tests/browser/.gitignore", include_str!("assets/gitignore")),
    (
        ".github/workflows/browser-tests.yml",
        include_str!("assets/browser-tests.yml"),
    ),
];

/// The `src/security_tests.rs` declaration the browser tests read.
const PAGE_LIST: &str = "const PAGES: &[(&str, u16)] = &[";

/// What happens to one generated file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Create,
    /// Already identical.
    Unchanged,
    /// Present with other content, which is kept.
    Kept,
}

/// Everything `add browser-tests` would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) files: Vec<(&'static str, &'static str, State)>,
    /// Whether `src/security_tests.rs` declares the `PAGES` list.
    pub(crate) page_list: bool,
}

/// Why the command stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    /// No `Cargo.toml` with a `rullst` dependency.
    NotAProject,
    InvalidManifest,
    Read {
        file: &'static str,
        kind: ErrorKind,
    },
    Write {
        file: &'static str,
        kind: ErrorKind,
    },
}

fn read(root: &Path, file: &'static str) -> Result<Option<String>, Error> {
    match std::fs::read_to_string(root.join(file)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::Read {
            file,
            kind: error.kind(),
        }),
    }
}

/// Plans the files for the project at `root`.
pub(crate) fn plan(root: &Path) -> Result<Plan, Error> {
    let manifest = read(root, "Cargo.toml")?.ok_or(Error::NotAProject)?;
    let manifest: toml::Value = toml::from_str(&manifest).map_err(|_| Error::InvalidManifest)?;
    if manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("rullst"))
        .is_none()
    {
        return Err(Error::NotAProject);
    }
    let mut files = Vec::new();
    for (path, content) in FILES {
        let state = match read(root, path)? {
            None => State::Create,
            Some(existing) if existing == content => State::Unchanged,
            Some(_) => State::Kept,
        };
        files.push((path, content, state));
    }
    let page_list = read(root, "src/security_tests.rs")?
        .is_some_and(|source| source.contains(PAGE_LIST));
    Ok(Plan { files, page_list })
}

/// Writes the files the plan creates.
pub(crate) fn apply(root: &Path, plan: &Plan) -> Result<(), Error> {
    for (path, content, state) in &plan.files {
        if *state != State::Create {
            continue;
        }
        let target = root.join(path);
        let written = target
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&target, content));
        written.map_err(|error| Error::Write {
            file: path,
            kind: error.kind(),
        })?;
    }
    Ok(())
}

/// Plans and (unless `dry_run`) applies the capability at `root`, returning
/// the report to print.
pub(crate) fn execute(root: &Path, dry_run: bool, style: Style) -> Result<String, Error> {
    let plan = plan(root)?;
    if !dry_run {
        apply(root, &plan)?;
    }
    Ok(report(&plan, dry_run, style))
}

fn report(plan: &Plan, dry_run: bool, style: Style) -> String {
    let mut out = format!(
        "{} {}\n",
        style.bold("◆", style::ACCENT),
        style.bold(&format!("cargo rullst add {NAME}"), style::BRIGHT)
    );
    for (path, _, state) in &plan.files {
        let (mark, summary) = match (state, dry_run) {
            (State::Create, true) => ("~", "would create"),
            (State::Create, false) => ("✓", "created"),
            (State::Unchanged, _) => ("·", "unchanged"),
            (State::Kept, _) => ("!", "kept: it differs from the generated file"),
        };
        let color = if *state == State::Kept {
            style::WARN
        } else {
            style::PASS
        };
        out.push_str(&format!(
            "  {} {:<36} {}\n",
            style.paint(mark, color),
            path,
            style.paint(summary, style::MUTED)
        ));
    }
    if !plan.page_list {
        out.push_str(&format!(
            "\n  {}\n",
            style.paint(
                "src/security_tests.rs declares no `const PAGES: &[(&str, u16)]` list (projects generated before 13.0.0-alpha.5): only `/` is visited until you add one.",
                style::WARN
            )
        ));
    }
    out.push_str(&format!(
        "\n{}\n",
        style.bold("Run them locally", style::BRIGHT)
    ));
    for line in [
        "npm ci --prefix tests/browser",
        "node tests/browser/node_modules/playwright/cli.js install --with-deps chromium firefox webkit",
        "cargo run                      # in another terminal",
        "node tests/browser/pages.mjs",
    ] {
        out.push_str(&format!("    {line}\n"));
    }
    out.push_str(&format!(
        "  {}\n",
        style.paint(
            ".github/workflows/browser-tests.yml runs them on every push to main and pull request. Add each new page to PAGES in src/security_tests.rs.",
            style::MUTED
        )
    ));
    if dry_run {
        out.push_str(&format!(
            "\n{}\n",
            style.paint(
                "Dry run: nothing was written. Run the same command without --dry-run to apply it.",
                style::MUTED
            )
        ));
    }
    out
}

#[cfg(test)]
mod tests;
