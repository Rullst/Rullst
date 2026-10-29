//! Commands removed from the CLI. They stay hidden from help, completions,
//! the command palette and the home screen; running one prints what replaces
//! it and exits with status 1, whatever arguments follow (`--help` included).

use crate::ui::error_report::{self, AlreadyReported, Friendly};
use crate::ui::style::Style;
use clap::{Arg, ArgAction, Command};
use std::error::Error;

/// One removed command and what replaces it.
pub(crate) struct Removed {
    pub(crate) name: &'static str,
    /// The release that removed it.
    pub(crate) release: &'static str,
    pub(crate) instead: &'static str,
    pub(crate) docs: &'static str,
}

pub(crate) const REMOVED: [Removed; 1] = [Removed {
    name: "ai",
    release: "13.0.0-alpha.4",
    instead: "Use your editor's or terminal's AI assistant with the project's AGENTS.md (`cargo rullst generate:ai-context` writes it with a .llms.txt inventory) and the framework's published index, https://rullst.github.io/Rullst/llms.txt.",
    docs: "https://rullst.github.io/Rullst/book/migration-v13.html#upgrading-an-alpha3-application-to-alpha4",
}];

/// The removed command called `name`, if any.
pub(crate) fn find(name: &str) -> Option<&'static Removed> {
    REMOVED.iter().find(|removed| removed.name == name)
}

/// Whether `name` is a removed command.
pub(crate) fn is_removed(name: &str) -> bool {
    find(name).is_some()
}

/// Hidden entries that accept and ignore any arguments, so that a global
/// flag before the name (`cargo rullst -v ai …`) reaches the same report.
pub(crate) fn commands() -> impl Iterator<Item = Command> {
    REMOVED.iter().map(|removed| {
        Command::new(removed.name)
            .hide(true)
            .disable_help_flag(true)
            .arg(
                Arg::new("arguments")
                    .value_name("ARGS")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true)
                    .action(ArgAction::Append),
            )
    })
}

/// The friendly report for a removed command.
pub(crate) fn friendly(removed: &Removed) -> Friendly {
    Friendly {
        title: format!("`cargo rullst {}` was removed", removed.name),
        happened: format!(
            "`cargo rullst {}` was removed in {}.",
            removed.name, removed.release
        ),
        fix: Some(removed.instead.to_string()),
        docs: Some(removed.docs),
    }
}

/// Prints the report for the removed command `name` on stderr; the returned
/// error carries exit status 1 and is not printed again.
pub(crate) fn report(name: &str) -> Box<dyn Error> {
    if let Some(removed) = find(name) {
        eprint!(
            "{}",
            error_report::render(&friendly(removed), &[], None, false, Style::stderr())
        );
    }
    AlreadyReported::new(1, format!("`cargo rullst {name}` was removed")).into()
}

#[cfg(test)]
#[path = "removed_tests.rs"]
mod tests;
