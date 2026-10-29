//! The seam `cargo-rullst-extras` builds on: the main CLI's helpers it reuses.
//!
//! Not a stable API. The extras crate requires this crate with an exact
//! version (`=`), so both always come from the same release; anything here
//! may change in any release.

use crate::ui::error_report::{self, AlreadyReported, Friendly, ProjectRequired};
use crate::ui::style::Style;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub use crate::generators::ai_context::ContextError;
pub use crate::generators::build::{AssistFinding, AssistPlan, assist_plan};

/// The complete `cargo rullst` command tree, as parsing and the palette see it.
pub fn command() -> clap::Command {
    crate::command()
}

/// `text` with secret-looking values replaced, and how many were replaced;
/// `None` when the text cannot be scanned safely.
pub fn redact_secrets(text: &str) -> Option<(String, usize)> {
    crate::generators::audit_report::redact_secrets(text)
}

/// The application port `cargo rullst dev` waits for.
pub fn configured_port() -> std::io::Result<u16> {
    crate::generators::dev::configured_port()
}

/// `cargo build --release` of the application at `root`; the binary path.
pub async fn compile_release_in(root: &Path) -> std::io::Result<PathBuf> {
    crate::generators::dev::compile_release_in(root).await
}

/// The `rullst.project-context.v1` inventory of the project at `base`.
pub fn inventory_json(base: &Path) -> Result<String, ContextError> {
    crate::generators::ai_context::inventory_json(base)
}

/// `[package].name` of a Cargo manifest.
pub fn package_name(manifest: &Path) -> Option<String> {
    crate::generators::platform_name::package_name(manifest)
}

/// `text` without terminal controls or secret-looking values, for reports.
pub fn sanitize(text: &str) -> String {
    error_report::sanitize(text)
}

/// Prints a friendly report (title, what happened, how to fix it and a docs
/// link) on standard error. The returned error carries exit status 1 and is
/// not printed again by [`run_and_report`].
pub fn failure(title: &str, happened: String, fix: &str, docs: &'static str) -> Box<dyn Error> {
    let friendly = Friendly {
        title: title.to_string(),
        happened: happened.clone(),
        fix: Some(fix.to_string()),
        docs: Some(docs),
    };
    eprint!(
        "{}",
        error_report::render(&friendly, &[], None, false, Style::stderr())
    );
    AlreadyReported::new(1, happened).into()
}

/// The error for a project command run outside a Rullst project.
pub fn project_required() -> Box<dyn Error> {
    ProjectRequired.into()
}

/// Runs `run` with the process arguments and renders a failure with the same
/// friendly report as `cargo rullst`; the process exit status.
#[cfg_attr(mutants, mutants::skip)]
pub fn run_and_report(run: impl FnOnce(Vec<String>) -> Result<(), Box<dyn Error>>) -> ExitCode {
    let arguments: Vec<String> = std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    error_report::install_panic_hook(error_report::panic_details_requested(&arguments));
    let result = crate::cli::runtime::unicode_arguments(std::env::args_os()).and_then(run);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => ExitCode::from(error_report::report(error.as_ref(), &arguments)),
    }
}
