//! `cargo rullst tour`: a short, skippable walkthrough of the main commands.
//! Each step explains a command family and offers one read-only example
//! (`--help` or a dry run) that runs only when the user picks it. The tour
//! needs no network and never changes a project. `--list`, pipes, CI and
//! `TERM=dumb` print the steps instead of prompting.

mod steps;
#[cfg(test)]
mod tests;

use crate::ui::screen::{Answer, Choice, Line, Screen, Selection, Terminal, Tone};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::io;
use steps::{STEPS, Step, shown};

const TITLE: &str = "Rullst tour";

pub(crate) fn command() -> Command {
    Command::new("tour")
        .about("A short guided walkthrough of the main commands (offline; changes nothing)")
        .arg(
            Arg::new("list")
                .long("list")
                .action(ArgAction::SetTrue)
                .help("Prints every step and its example command without prompting"),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::detect();
    if matches.get_flag("list") || !terminal.interactive() {
        terminal.print(&list_lines())?;
        return Ok(());
    }
    guided(&mut TerminalTour { terminal })?;
    Ok(())
}

/// The whole tour as plain text, for `--list` and non-interactive terminals.
pub(crate) fn list_lines() -> Vec<Line> {
    let mut lines = vec![
        Line::new()
            .push(Tone::Brand, TITLE)
            .push(Tone::Muted, format!(" · {} steps", STEPS.len())),
        Line::new().push(
            Tone::Muted,
            "Run `cargo rullst tour` in a terminal for the guided version; examples run only when you pick them.",
        ),
    ];
    for (index, step) in STEPS.iter().enumerate() {
        lines.push(Line::new());
        lines.push(
            Line::new()
                .push(Tone::Accent, format!("{}. ", index + 1))
                .push(Tone::Strong, step.title)
                .push(Tone::Muted, format!("  ({})", step.command)),
        );
        for text in step.explanation {
            lines.push(Line::new().push(Tone::Value, format!("   {text}")));
        }
        for (position, command) in step.try_commands.iter().enumerate() {
            let label = if position == 0 {
                "   Try      "
            } else {
                "            "
            };
            lines.push(
                Line::new()
                    .push(Tone::Label, label)
                    .push(Tone::Value, *command),
            );
        }
        lines.push(
            Line::new()
                .push(Tone::Label, "   Example  ")
                .push(Tone::Value, shown(step.example.arguments))
                .push(Tone::Muted, format!("  ({})", step.example.effect)),
        );
    }
    lines
}

/// The terminal side of the tour; tests script it.
pub(crate) trait TourUi {
    fn choose(&mut self, screen: &Screen) -> io::Result<Answer>;
    fn print(&mut self, lines: &[Line]) -> io::Result<()>;
    /// Whether this CLI build has the subcommand `arguments` starts with.
    fn available(&mut self, arguments: &[&str]) -> bool;
    /// Runs this CLI with `arguments`, attached to the terminal; the exit code.
    fn run(&mut self, arguments: &[&str]) -> io::Result<Option<i32>>;
}

/// What the user picked on a step screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Next,
    Example,
    Previous,
    Quit,
}

fn step_screen(index: usize, step: &Step) -> (Screen, Vec<Action>) {
    let last = index + 1 == STEPS.len();
    let mut body = vec![Line::new().push(Tone::Strong, step.title), Line::new()];
    body.extend(
        step.explanation
            .iter()
            .map(|text| Line::new().push(Tone::Value, *text)),
    );
    body.push(Line::new());
    for (position, command) in step.try_commands.iter().enumerate() {
        let label = if position == 0 { "Try  " } else { "     " };
        body.push(
            Line::new()
                .push(Tone::Label, label)
                .push(Tone::Value, *command),
        );
    }
    let mut actions = vec![
        (
            Action::Next,
            Choice::new(if last { "Finish" } else { "Next" }, ""),
        ),
        (
            Action::Example,
            Choice::new(
                format!("Run `{}`", shown(step.example.arguments)),
                step.example.effect,
            ),
        ),
    ];
    if index > 0 {
        actions.push((Action::Previous, Choice::new("Previous", "")));
    }
    actions.push((Action::Quit, Choice::new("Quit the tour", "")));
    let screen = Screen {
        title: TITLE.to_string(),
        crumb: format!("Step {} of {} · {}", index + 1, STEPS.len(), step.command),
        body,
        question: String::new(),
        choices: actions.iter().map(|(_, choice)| choice.clone()).collect(),
        selection: Selection::One { initial: 0 },
        can_go_back: index > 0,
        echo: None,
    };
    (
        screen,
        actions.into_iter().map(|(action, _)| action).collect(),
    )
}

fn run_example<U: TourUi>(ui: &mut U, step: &Step) -> io::Result<()> {
    let subcommand = shown(step.example.arguments.get(..1).unwrap_or_default());
    let arguments = if ui.available(step.example.arguments) {
        step.example.arguments
    } else if let Some(fallback) = step
        .example
        .fallback
        .filter(|fallback| ui.available(fallback))
    {
        ui.print(&[Line::new().push(
            Tone::Muted,
            format!(
                "`{subcommand}` is not part of this build; showing `{}` instead.",
                shown(fallback)
            ),
        )])?;
        fallback
    } else {
        return ui.print(&[Line::new().push(
            Tone::Warning,
            format!("`{subcommand}` is not part of this build."),
        )]);
    };
    ui.print(&[Line::new().push(Tone::Muted, format!("$ {}", shown(arguments)))])?;
    let status = match ui.run(arguments) {
        Ok(Some(code)) => format!("exit status {code}"),
        Ok(None) => "stopped by a signal".to_string(),
        Err(error) => format!("could not start: {error}"),
    };
    ui.print(&[
        Line::new().push(
            Tone::Muted,
            format!("↳ `{}` finished ({status})", shown(arguments)),
        ),
        Line::new(),
    ])
}

/// Walks through [`STEPS`]. Nothing runs unless the user picks an example.
pub(crate) fn guided<U: TourUi>(ui: &mut U) -> io::Result<()> {
    let mut index = 0;
    while let Some(step) = STEPS.get(index) {
        let (screen, actions) = step_screen(index, step);
        let action = match ui.choose(&screen)? {
            Answer::One(choice) => actions.get(choice).copied().unwrap_or(Action::Next),
            Answer::Back => Action::Previous,
            Answer::Many(_) => Action::Next,
        };
        match action {
            Action::Next => index += 1,
            Action::Example => run_example(ui, step)?,
            Action::Previous => index = index.saturating_sub(1),
            Action::Quit => {
                return ui.print(&[Line::new().push(
                    Tone::Muted,
                    "Tour closed. `cargo rullst tour --list` prints every step.",
                )]);
            }
        }
    }
    ui.print(&[Line::new()
        .push(Tone::Accent, "◆ ")
        .push(Tone::Strong, "That's the tour. ")
        .push(Tone::Value, "Create your first app: ")
        .push(Tone::Accent, "cargo rullst new my_app")])
}

struct TerminalTour {
    terminal: Terminal,
}

impl TourUi for TerminalTour {
    fn choose(&mut self, screen: &Screen) -> io::Result<Answer> {
        self.terminal.choose(screen)
    }

    fn print(&mut self, lines: &[Line]) -> io::Result<()> {
        self.terminal.print(lines)
    }

    fn available(&mut self, arguments: &[&str]) -> bool {
        let (Ok(program), Some(subcommand)) = (std::env::current_exe(), arguments.first()) else {
            return false;
        };
        std::process::Command::new(program)
            .args(["help", subcommand])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn run(&mut self, arguments: &[&str]) -> io::Result<Option<i32>> {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args(arguments)
            .status()?;
        Ok(status.code())
    }
}
