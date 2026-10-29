//! `cargo rullst ai …` and `cargo rullst green …` run the optional
//! `cargo-rullst-extras` executable found on `PATH`, the way Cargo finds its
//! own `cargo-<name>` plugins. Arguments pass through unparsed and the
//! extras' exit status becomes this process's exit status. On Unix the
//! extras replace this process, as Cargo runs its plugins, so Ctrl+C reaches
//! only them; elsewhere this process waits and leaves Ctrl+C to them.

use crate::ui::error_report::{self, AlreadyReported, Friendly};
use crate::ui::style::Style;
use clap::{Arg, ArgAction, Command};
use std::error::Error;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(any(not(unix), test))]
use std::process::ExitStatus;

/// The extras executable, without the platform's executable suffix.
pub(crate) const BINARY: &str = "cargo-rullst-extras";
/// The subcommands the extras provide, with their one-line description.
pub(crate) const SUBCOMMANDS: [(&str, &str); 2] = [
    (
        "ai",
        "(extras) Terminal assistant that knows Rullst and your project",
    ),
    (
        "green",
        "(extras) Measured size, memory and CPU figures, and an optional SCI carbon estimate",
    ),
];
/// Names this CLI's executable for the extras, which run their allowlisted
/// `cargo rullst` commands through it.
pub(crate) const CLI_VARIABLE: &str = "RULLST_CLI";
const DOCS: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#extras-cargo-rullst-extras";

/// The exact command that installs the extras matching this CLI.
pub(crate) fn install_command() -> String {
    format!(
        "cargo install {BINARY} --version ={} --locked",
        env!("CARGO_PKG_VERSION")
    )
}

/// Whether `name` is a subcommand the extras provide.
pub(crate) fn is_extras(name: &str) -> bool {
    SUBCOMMANDS
        .iter()
        .any(|(subcommand, _)| *subcommand == name)
}

/// The entries shown by `--help`, completions and the command palette. Their
/// arguments, `--help` included, are passed to the extras unparsed.
pub(crate) fn commands() -> impl Iterator<Item = Command> {
    SUBCOMMANDS.iter().map(|(name, about)| {
        Command::new(*name)
            .about(*about)
            .disable_help_flag(true)
            .arg(
                Arg::new("arguments")
                    .value_name("ARGS")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true)
                    .action(ArgAction::Append)
                    .help("Passed to cargo-rullst-extras unchanged"),
            )
    })
}

/// The first `cargo-rullst-extras` executable in the absolute directories of
/// `path` (a `PATH` value). Relative entries, including the empty entry that
/// means the current directory, are skipped so that a checked-out project
/// cannot plant one.
pub(crate) fn find(path: Option<&OsStr>) -> Option<PathBuf> {
    let name = format!("{BINARY}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path?)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&name))
        .find(|candidate| executable(candidate))
}

/// Whether the extras are installed on this process's `PATH`.
pub(crate) fn installed() -> bool {
    find(std::env::var_os("PATH").as_deref()).is_some()
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// The report printed when the extras are not installed.
pub(crate) fn missing_report(subcommand: &str) -> Friendly {
    Friendly {
        title: format!("`cargo rullst {subcommand}` needs cargo-rullst-extras"),
        happened: format!(
            "`cargo rullst {subcommand}` is part of the optional cargo-rullst-extras package, and no `{BINARY}` executable was found on PATH."
        ),
        fix: Some(format!(
            "Install the extras that match this CLI, then run the command again:\n{}",
            install_command()
        )),
        docs: Some(DOCS),
    }
}

/// Runs `cargo-rullst-extras <subcommand> <arguments…>` from the process's
/// `PATH`.
pub(crate) fn run(subcommand: &str, arguments: &[String]) -> Result<(), Box<dyn Error>> {
    run_with(std::env::var_os("PATH").as_deref(), subcommand, arguments)
}

/// [`run`] with an explicit `PATH` value.
pub(crate) fn run_with(
    path: Option<&OsStr>,
    subcommand: &str,
    arguments: &[String],
) -> Result<(), Box<dyn Error>> {
    let Some(program) = find(path) else {
        eprint!(
            "{}",
            error_report::render(
                &missing_report(subcommand),
                &[],
                None,
                false,
                Style::stderr()
            )
        );
        return Err(AlreadyReported::new(1, format!("{BINARY} is not installed")).into());
    };
    let mut command = std::process::Command::new(&program);
    command.arg(subcommand).args(arguments);
    if let Ok(cli) = std::env::current_exe() {
        command.env(CLI_VARIABLE, cli);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Returns only when the extras could not be started.
        let error = command.exec();
        Err(start_failure(&program, error).into())
    }
    #[cfg(not(unix))]
    {
        // While this listener lives, Ctrl+C does not end this process; the
        // extras, attached to the same console, receive it and decide.
        #[cfg(windows)]
        let _interrupts = tokio::signal::windows::ctrl_c();
        let status = command
            .status()
            .map_err(|error| start_failure(&program, error))?;
        passthrough(status, subcommand)
    }
}

fn start_failure(program: &Path, error: std::io::Error) -> std::io::Error {
    std::io::Error::new(
        error.kind(),
        format!("could not start {}: {error}", program.display()),
    )
}

/// The extras' exit status as this process's: the same code, or 128 plus
/// the signal number when a signal ended them.
#[cfg(any(not(unix), test))]
fn passthrough(status: ExitStatus, subcommand: &str) -> Result<(), Box<dyn Error>> {
    if status.success() {
        return Ok(());
    }
    Err(AlreadyReported::new(
        exit_code(status),
        format!("`cargo rullst {subcommand}` failed ({status})"),
    )
    .into())
}

#[cfg(any(not(unix), test))]
fn exit_code(status: ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code).unwrap_or(1);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return u8::try_from(128 + signal).unwrap_or(1);
        }
    }
    1
}

#[cfg(test)]
#[path = "forward_tests.rs"]
mod tests;
