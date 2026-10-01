//! The executable layer around the public `Commands` enum: the shared
//! friendly error report at the process boundary.

use crate::ui::error_report;
use crate::ui::style::Style;

/// The binaries' entry point: runs the CLI and renders any failure with the
/// shared friendly report. Returns the process exit status.
#[doc(hidden)]
pub fn run_and_report() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    error_report::install_panic_hook(error_report::verbose_requested(&arguments));
    if Style::stdout().is_plain() {
        // Generators colour through `colored`; keep pipes, CI and NO_COLOR plain.
        colored::control::set_override(false);
    }
    match crate::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            std::process::ExitCode::from(error_report::report(error.as_ref(), &arguments))
        }
    }
}
