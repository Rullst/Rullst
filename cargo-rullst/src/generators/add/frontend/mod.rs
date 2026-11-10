//! `cargo rullst add tailwind` and `cargo rullst add alpine`: opt-in
//! frontend tools. Unlike the facade capabilities they write files: the
//! Tailwind input stylesheet or the vendored Alpine.js CSP build with an
//! example component, the layout link when the layout is recognisable
//! ([`layout`]), `.gitignore` and a generated Dockerfile. Each tool is a
//! pinned download verified by checksum ([`crate::generators::frontend`]).
//! Downloads happen before anything is written; running the command again
//! changes nothing.

mod alpine;
#[cfg(test)]
mod csp_tests;
#[cfg(test)]
mod kit_tests;
mod layout;
mod project;
mod report;
mod tailwind;
#[cfg(test)]
mod tests;

use crate::generators::frontend::tailwind::{ToolError, Verified};
use crate::generators::frontend::tailwind_release::Asset;
use crate::generators::frontend::{Fetcher, alpine::VendorError};
use crate::ui::style::Style;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// A frontend tool `cargo rullst add` sets up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Kit {
    pub name: &'static str,
    pub summary: &'static str,
    /// `(command, purpose)`, at most three.
    pub hints: &'static [(&'static str, &'static str)],
}

pub(crate) const KITS: [Kit; 2] = [
    Kit {
        name: "tailwind",
        summary: "Tailwind CSS v4 through its pinned standalone CLI (no Node.js)",
        hints: &[
            ("cargo rullst dev", "rebuilds static/app.css while you edit"),
            (
                "cargo rullst build --release",
                "minifies it before pre-compression",
            ),
        ],
    },
    Kit {
        name: "alpine",
        summary: "the vendored Alpine.js CSP build for client-only state",
        hints: &[(
            "cargo rullst dev",
            "then add x-data=\"charCounter(160)\" to a page",
        )],
    },
];

/// The `<capability>` values of the kits, in `--help` order.
pub(crate) const NAMES: [&str; 2] = ["tailwind", "alpine"];

pub(crate) fn find(name: &str) -> Option<&'static Kit> {
    KITS.iter().find(|kit| kit.name == name)
}

/// One text file the command creates or changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    pub path: String,
    /// `None` when the file is created.
    pub before: Option<String>,
    pub after: String,
    pub summary: String,
}

/// Wiring printed instead of written: where it goes and the exact code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manual {
    pub place: String,
    pub code: String,
}

/// A pinned download the kit needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Download {
    /// The Tailwind CLI into the user cache (`cached`: already there).
    TailwindCli { cached: bool },
    /// The Alpine CSP build into `static/vendor/`.
    AlpineCsp { replaces: bool },
}

/// Everything one kit would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub kit: &'static Kit,
    pub changes: Vec<Change>,
    pub download: Option<Download>,
    pub manual: Vec<Manual>,
    /// Facts printed with the report (for example a file left as it is).
    pub notes: Vec<String>,
}

/// Why the command stopped before writing anything (or, for `Write`, while
/// writing).
#[derive(Debug, thiserror::Error)]
pub(crate) enum FrontendError {
    #[error("{file} {why}")]
    Conflict {
        file: &'static str,
        why: &'static str,
        fix: &'static str,
    },
    #[error("could not read {file}: {kind}")]
    Read { file: String, kind: ErrorKind },
    #[error("could not write {file}: {kind}")]
    Write { file: String, kind: ErrorKind },
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error(transparent)]
    Vendor(#[from] VendorError),
}

/// Where the Tailwind CLI comes from: the pinned asset of the host and the
/// user cache in the CLI; a fixture in tests.
pub(crate) struct Toolchain<'a, F: Fetcher> {
    pub fetcher: &'a F,
    /// `None` when Tailwind publishes no CLI for the host.
    pub tailwind: Option<&'a Asset>,
    pub cache_base: Option<PathBuf>,
    /// Run the first Tailwind build after writing the files.
    pub build: bool,
    /// The expected npm integrity of the Alpine tarball (the pinned one).
    pub alpine_integrity: &'a str,
}

/// What a run did besides the planned text changes.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub tailwind: Option<Verified>,
    pub built: Option<Result<(), String>>,
}

/// Why `execute` stopped.
#[derive(Debug)]
pub(crate) enum Failure {
    Project,
    Frontend(FrontendError),
}

/// Plans and (unless `dry_run`) applies `kit` at `root`; returns the report.
pub(crate) fn execute<F: Fetcher>(
    root: &Path,
    kit: &'static Kit,
    dry_run: bool,
    style: Style,
    toolchain: &Toolchain<'_, F>,
) -> Result<String, Failure> {
    let project = project::Project::read(root).ok_or(Failure::Project)?;
    let plan = match kit.name {
        "tailwind" => tailwind::plan(root, &project, toolchain),
        _ => alpine::plan(root, &project),
    }
    .map_err(Failure::Frontend)?;
    let outcome = if dry_run {
        Outcome::default()
    } else {
        apply(root, &plan, toolchain).map_err(Failure::Frontend)?
    };
    Ok(report::render(&plan, &outcome, dry_run, style))
}

/// Downloads first (a failure writes nothing), then writes the changes, the
/// vendored file and, for Tailwind, runs the first build.
fn apply<F: Fetcher>(
    root: &Path,
    plan: &Plan,
    toolchain: &Toolchain<'_, F>,
) -> Result<Outcome, FrontendError> {
    let mut outcome = Outcome::default();
    let mut vendored = None;
    match plan.download {
        Some(Download::TailwindCli { .. }) => {
            let asset = toolchain
                .tailwind
                .ok_or_else(crate::generators::frontend::tailwind::unsupported_host)?;
            let base = toolchain
                .cache_base
                .as_deref()
                .ok_or(ToolError::NoCacheDirectory)?;
            outcome.tailwind = Some(crate::generators::frontend::tailwind::ensure(
                toolchain.fetcher,
                base,
                asset,
            )?);
        }
        Some(Download::AlpineCsp { .. }) => {
            vendored = Some(crate::generators::frontend::alpine::download_with(
                toolchain.fetcher,
                toolchain.alpine_integrity,
            )?);
        }
        None => {}
    }
    for change in &plan.changes {
        write(root, &change.path, &change.after)?;
    }
    if let Some(code) = vendored {
        write(root, crate::generators::frontend::ALPINE_VENDORED, &code)?;
    }
    if let Some(verified) = &outcome.tailwind
        && toolchain.build
        && (!plan.changes.is_empty()
            || !root
                .join(crate::generators::frontend::TAILWIND_OUTPUT)
                .is_file())
    {
        outcome.built = Some(
            crate::generators::frontend::tailwind::run_build(&verified.path, root, false)
                .map_err(|error| error.to_string()),
        );
    }
    Ok(outcome)
}

fn write(root: &Path, relative: &str, contents: &str) -> Result<(), FrontendError> {
    let path = root.join(relative);
    let failed = |error: std::io::Error| FrontendError::Write {
        file: relative.to_owned(),
        kind: error.kind(),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(failed)?;
    }
    std::fs::write(&path, contents).map_err(failed)
}

/// Reads a project file; a missing file is `None`.
pub(crate) fn read(root: &Path, relative: &str) -> Result<Option<String>, FrontendError> {
    match std::fs::read_to_string(root.join(relative)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(FrontendError::Read {
            file: relative.to_owned(),
            kind: error.kind(),
        }),
    }
}

/// Runs a kit in the current project with the real downloads.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn run(
    root: &Path,
    kit: &'static Kit,
    dry_run: bool,
    style: Style,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::generators::frontend::{HttpFetcher, tailwind as cli};
    use crate::ui::error_report::{self, AlreadyReported, ProjectRequired};
    let toolchain = Toolchain {
        fetcher: &HttpFetcher,
        tailwind: cli::host_asset().ok(),
        cache_base: cli::user_cache_base(),
        build: true,
        alpine_integrity: crate::generators::frontend::alpine_release::INTEGRITY,
    };
    if !dry_run && kit.name == "tailwind" {
        let cached = toolchain
            .tailwind
            .zip(toolchain.cache_base.as_deref())
            .is_some_and(|(asset, base)| cli::binary_path(base, asset).is_file());
        if !cached {
            eprintln!(
                "Downloading the Tailwind CSS v{} standalone CLI (about 100 MB, once per user)...",
                crate::generators::frontend::tailwind_release::VERSION
            );
        }
    }
    match execute(root, kit, dry_run, style, &toolchain) {
        Ok(text) => {
            print!("{text}");
            Ok(())
        }
        Err(Failure::Project) => Err(ProjectRequired.into()),
        Err(Failure::Frontend(error)) => {
            eprint!(
                "{}",
                error_report::render(&friendly(&error), &[], None, false, Style::stderr())
            );
            Err(AlreadyReported::new(1, error.to_string()).into())
        }
    }
}

const DOCS: &str = "https://rullst.github.io/Rullst/book/frontend.html";

pub(crate) fn friendly(error: &FrontendError) -> crate::ui::error_report::Friendly {
    let (title, fix) = match error {
        FrontendError::Conflict { fix, .. } => ("A project file is in the way", (*fix).to_owned()),
        FrontendError::Read { .. } => (
            "A project file could not be read",
            "Check the file's permissions and that it is valid UTF-8, then run the command again.".to_owned(),
        ),
        FrontendError::Write { .. } => (
            "A project file could not be written",
            "Check the file's permissions and free disk space; running the command again is safe.".to_owned(),
        ),
        FrontendError::Tool(ToolError::ChecksumMismatch { path, .. }) => (
            "The Tailwind CSS CLI failed its checksum",
            format!(
                "Nothing was run or written. Delete {} and run the command again; if it fails again, report it.",
                path.display()
            ),
        ),
        FrontendError::Tool(ToolError::UnsupportedPlatform { .. }) => (
            "No Tailwind CSS CLI for this platform",
            "Build the stylesheet in a container (the generated Dockerfile does) or on a supported machine.".to_owned(),
        ),
        FrontendError::Tool(_) => (
            "The Tailwind CSS CLI could not be prepared",
            "Check the network and the user cache directory, then run the command again; nothing was written.".to_owned(),
        ),
        FrontendError::Vendor(_) => (
            "The Alpine.js CSP build could not be vendored",
            "Check the network, then run the command again; nothing was written.".to_owned(),
        ),
    };
    crate::ui::error_report::Friendly {
        title: title.to_owned(),
        happened: error.to_string(),
        fix: Some(fix),
        docs: Some(DOCS),
    }
}
