//! The one place CLI failures are printed. Every error that reaches the
//! process boundary is rendered as a short title, what happened, how to fix
//! it and a docs link, on standard error. Secrets and terminal escapes are
//! removed from every line; the error chain and `Debug` form appear only with
//! `-v`/`--verbose`, and panics print a short report unless `-v` or `RUST_BACKTRACE`
//! instead of a backtrace unless one of those is set.

mod classify;
mod redact;

pub(crate) use classify::{Facts, Friendly, classify};
pub(crate) use redact::sanitize;

use super::palette::Rgb;
use super::style::{self, Style};
use std::error::Error;
use std::fmt;

/// Exit status for usage errors (unknown command, invalid arguments), as clap uses.
pub(crate) const USAGE_EXIT: u8 = 2;
const LABEL_WIDTH: usize = 15;
const ISSUES_URL: &str = "https://github.com/Rullst/Rullst/issues";

/// A failure whose details the command already printed; only its exit
/// status remains. Library callers still get a meaningful message.
#[derive(Debug)]
pub(crate) struct AlreadyReported {
    code: u8,
    message: String,
}

impl AlreadyReported {
    pub(crate) fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code: code.max(1),
            message: message.into(),
        }
    }
}

impl fmt::Display for AlreadyReported {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AlreadyReported {}

/// A project command ran outside the root of a Rullst project.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProjectRequired;

impl fmt::Display for ProjectRequired {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "this command must be executed in the root of a valid Rullst project \
             (a Cargo.toml with a `rullst` dependency)",
        )
    }
}

impl Error for ProjectRequired {}

/// Whether `-v`/`--verbose` (before any `--`) asks for the underlying causes
/// and `Debug` form. `RUST_BACKTRACE` is often set globally by developers and
/// CI, so it only keeps the default panic hook (see [`panic_details_requested`]).
pub(crate) fn verbose_requested(arguments: &[String]) -> bool {
    verbose_flag(arguments)
}

/// Whether a panic should keep Rust's default message and backtrace.
pub(crate) fn panic_details_requested(arguments: &[String]) -> bool {
    verbose_flag(arguments) || backtrace_requested(std::env::var_os("RUST_BACKTRACE").as_deref())
}

fn verbose_flag(arguments: &[String]) -> bool {
    arguments
        .iter()
        .skip(1)
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| {
            argument == "--verbose"
                || argument
                    .strip_prefix('-')
                    .is_some_and(|flags| !flags.is_empty() && flags.chars().all(|c| c == 'v'))
        })
}

fn backtrace_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

/// The subcommand named on the command line, skipping Cargo's `rullst` argument.
pub(crate) fn command_name(arguments: &[String]) -> Option<String> {
    let mut rest = arguments.iter().skip(1).peekable();
    if rest
        .peek()
        .is_some_and(|first| first.as_str() == "rullst" || first.as_str() == "cli")
    {
        rest.next();
    }
    rest.find(|argument| !argument.starts_with('-'))
        .map(|name| sanitize(name).chars().take(64).collect())
}

/// The error chain below `error`, outermost cause first.
fn causes(error: &(dyn Error + 'static)) -> Vec<String> {
    let mut causes = Vec::new();
    let mut current = error.source();
    while let Some(cause) = current {
        if causes.len() == 16 {
            break;
        }
        causes.push(cause.to_string());
        current = cause.source();
    }
    causes
}

fn push_field(out: &mut String, style: Style, label: &str, value: &str, color: Rgb) {
    let padding = " ".repeat(LABEL_WIDTH.saturating_sub(label.chars().count()));
    let indent = " ".repeat(LABEL_WIDTH + 2);
    for (index, line) in value.lines().enumerate() {
        if index == 0 {
            out.push_str(&format!(
                "  {}{padding}{}\n",
                style.paint(label, style::MUTED),
                style.paint(line, color)
            ));
        } else {
            out.push_str(&format!("{indent}{}\n", style.paint(line, color)));
        }
    }
    if value.is_empty() {
        out.push_str(&format!("  {}\n", style.paint(label, style::MUTED)));
    }
}

/// The text printed for `friendly`. `causes` (the error chain) and `debug`
/// (the `Debug` form) are shown only when `verbose`.
pub(crate) fn render(
    friendly: &Friendly,
    causes: &[String],
    debug: Option<&str>,
    verbose: bool,
    style: Style,
) -> String {
    let mut out = String::new();
    let title = sanitize(&friendly.title);
    if style.is_plain() {
        out.push_str(&format!("error: {title}\n"));
    } else {
        out.push_str(&format!(
            "{} {}\n",
            style.bold("✗", style::FAIL),
            style.bold(&title, style::BRIGHT)
        ));
    }
    push_field(
        &mut out,
        style,
        "What happened",
        &sanitize(&friendly.happened),
        style::BRIGHT,
    );
    if let Some(fix) = &friendly.fix {
        push_field(&mut out, style, "How to fix", &sanitize(fix), style::BRIGHT);
    }
    if let Some(docs) = friendly.docs {
        push_field(&mut out, style, "Docs", docs, style::ACCENT);
    }
    if verbose {
        let numbered: Vec<String> = causes
            .iter()
            .enumerate()
            .map(|(index, cause)| format!("{}. {}", index + 1, sanitize(cause)))
            .collect();
        if !numbered.is_empty() {
            push_field(
                &mut out,
                style,
                "Caused by",
                &numbered.join("\n"),
                style::BRIGHT,
            );
        }
        if let Some(debug) = debug {
            push_field(&mut out, style, "Debug", &sanitize(debug), style::MUTED);
        }
    } else if !causes.is_empty() {
        out.push_str(&format!(
            "  {}\n",
            style.paint(
                "Run again with -v to see the underlying causes.",
                style::MUTED
            )
        ));
    }
    out
}

/// Prints `error` on standard error and returns the process exit status.
pub(crate) fn report(error: &(dyn Error + 'static), arguments: &[String]) -> u8 {
    if let Some(reported) = error.downcast_ref::<AlreadyReported>() {
        return reported.code;
    }
    let verbose = verbose_requested(arguments);
    let project_root = std::env::current_dir().ok().and_then(|current| {
        super::home::find_project(&current)
            .and_then(|(root, _, _)| super::home::relative_root(&current, &root))
    });
    let facts = Facts::from_error(error, command_name(arguments), project_root);
    let friendly = classify(&facts);
    let debug = format!("{error:?}");
    let text = render(
        &friendly,
        &causes(error),
        Some(&debug),
        verbose,
        Style::stderr(),
    );
    eprint!("{text}");
    1
}

fn panic_text(info: &std::panic::PanicHookInfo<'_>, style: Style) -> String {
    let payload = info
        .payload()
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic payload".to_string());
    let location = info
        .location()
        .map(|location| format!(" at {}:{}", location.file(), location.line()))
        .unwrap_or_default();
    let friendly = Friendly {
        title: "Internal error in cargo-rullst".to_string(),
        happened: format!("{payload}{location}"),
        fix: Some(format!(
            "This is a bug. Run the command again with RUST_BACKTRACE=1 and report it at {ISSUES_URL}."
        )),
        docs: None,
    };
    render(&friendly, &[], None, false, style)
}

/// Replaces the default panic message with a short report, unless details
/// were requested (then the default hook, with its backtrace, stays).
pub(crate) fn install_panic_hook(verbose: bool) {
    if verbose {
        return;
    }
    let style = Style::stderr();
    std::panic::set_hook(Box::new(move |info| {
        eprint!("{}", panic_text(info, style));
    }));
}

#[cfg(test)]
mod tests;
