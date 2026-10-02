//! The fuzzy command palette of the interactive home: type to filter every
//! command (including the subcommands attached at runtime) with its one-line
//! description, Enter runs it (asking for its required arguments first), Esc
//! returns to the home menu. The non-interactive equivalent is
//! `cargo rullst --help`.

mod score;
mod state;

pub(crate) use state::{Input, Outcome, PaletteState, frame};

use crate::ui::style::Style;
use clap::Command;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::{self, Write};

/// A required argument the palette asks for before running a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RequiredArgument {
    /// `--name` for an option; `None` for a positional value.
    pub long: Option<String>,
    /// The prompt shown to the user.
    pub prompt: String,
    /// Allowed values, offered as a menu when present.
    pub choices: Vec<String>,
}

/// One runnable command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaletteEntry {
    /// Words after the program, such as `["update", "check"]`.
    pub path: Vec<String>,
    /// `path` joined by spaces.
    pub name: String,
    pub about: String,
    pub aliases: Vec<String>,
    pub required: Vec<RequiredArgument>,
}

fn required_arguments(command: &Command) -> Vec<RequiredArgument> {
    command
        .get_arguments()
        .filter(|argument| argument.is_required_set() && !argument.is_global_set())
        .map(|argument| {
            let label = argument
                .get_value_names()
                .and_then(|names| names.first())
                .map_or_else(
                    || argument.get_id().to_string().to_uppercase(),
                    ToString::to_string,
                );
            let prompt = argument
                .get_help()
                .map_or_else(|| label.clone(), ToString::to_string);
            RequiredArgument {
                long: (!argument.is_positional())
                    .then(|| argument.get_long().map(|long| format!("--{long}")))
                    .flatten(),
                prompt,
                choices: argument
                    .get_possible_values()
                    .iter()
                    .filter(|value| !value.is_hide_set())
                    .map(|value| value.get_name().to_string())
                    .collect(),
            }
        })
        .collect()
}

fn collect(command: &Command, path: &[String], depth: usize, entries: &mut Vec<PaletteEntry>) {
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() || subcommand.get_name() == "help" {
            continue;
        }
        let mut sub_path = path.to_vec();
        sub_path.push(subcommand.get_name().to_string());
        let has_children = subcommand
            .get_subcommands()
            .any(|child| !child.is_hide_set() && child.get_name() != "help");
        if !(has_children && subcommand.is_subcommand_required_set()) {
            entries.push(PaletteEntry {
                name: sub_path.join(" "),
                about: subcommand
                    .get_about()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                aliases: subcommand.get_all_aliases().map(str::to_string).collect(),
                required: required_arguments(subcommand),
                path: sub_path.clone(),
            });
        }
        if has_children && depth < 2 {
            collect(subcommand, &sub_path, depth + 1, entries);
        }
    }
}

/// Every visible command of `command`, nested ones included, by name.
pub(crate) fn entries(command: &Command) -> Vec<PaletteEntry> {
    let mut entries = Vec::new();
    collect(command, &[], 0, &mut entries);
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

/// Indices into `entries` matching `query`, best first.
pub(crate) fn matches(query: &str, entries: &[PaletteEntry]) -> Vec<usize> {
    score::rank(
        query,
        entries.iter().map(|entry| {
            (
                entry.name.as_str(),
                entry.aliases.as_slice(),
                entry.about.as_str(),
            )
        }),
    )
}

pub(crate) fn input_for(key: KeyEvent) -> Input {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if control => Input::Interrupt,
        KeyCode::Char('u') if control => Input::ClearQuery,
        KeyCode::Char('p') if control => Input::Up,
        KeyCode::Char('n') if control => Input::Down,
        KeyCode::Char(_) if control => Input::Ignore,
        KeyCode::Char(character) => Input::Char(character),
        KeyCode::Backspace => Input::Backspace,
        KeyCode::Up | KeyCode::BackTab => Input::Up,
        KeyCode::Down | KeyCode::Tab => Input::Down,
        KeyCode::PageUp => Input::PageUp,
        KeyCode::PageDown => Input::PageDown,
        KeyCode::Home => Input::Home,
        KeyCode::End => Input::End,
        KeyCode::Enter => Input::Enter,
        KeyCode::Esc => Input::Escape,
        _ => Input::Ignore,
    }
}

/// Raw mode and a hidden cursor for the palette's lifetime; dropping it
/// restores the terminal on every path.
struct RawMode;

impl RawMode {
    fn enable() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b[?25l");
        let _ = stdout.flush();
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b[?25h");
        let _ = stdout.flush();
    }
}

/// Moves to the first line of the previous frame and clears from there.
fn clear(out: &mut impl Write, drawn: usize) -> io::Result<()> {
    if drawn > 1 {
        write!(out, "\x1b[{}A", drawn - 1)?;
    }
    write!(out, "\r\x1b[J")
}

/// Shows the palette until the user runs a command (its index in
/// `entries`) or presses Esc (`None`). Ctrl+C is an interrupted read.
pub(crate) fn run(entries: &[PaletteEntry], style: Style) -> io::Result<Option<usize>> {
    let _raw = RawMode::enable()?;
    let mut stdout = io::stdout();
    let mut state = PaletteState::default();
    let mut drawn = 0;
    loop {
        let found = matches(&state.query, entries);
        let (columns, lines) = crossterm::terminal::size().unwrap_or((80, 24));
        let rows = usize::from(lines).saturating_sub(4).clamp(3, 12);
        state.scroll(found.len(), rows);
        let frame = frame(&state, entries, &found, rows, usize::from(columns), style);
        clear(&mut stdout, drawn)?;
        write!(stdout, "{}", frame.join("\r\n"))?;
        stdout.flush()?;
        drawn = frame.len();
        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        let outcome = state.handle(input_for(key), found.len(), rows);
        if outcome == Outcome::Continue {
            continue;
        }
        clear(&mut stdout, drawn)?;
        stdout.flush()?;
        return match outcome {
            Outcome::Run => Ok(found.get(state.selected).copied()),
            Outcome::Interrupt => Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "read interrupted",
            )),
            _ => Ok(None),
        };
    }
}

#[cfg(test)]
mod tests;
