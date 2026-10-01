//! The context-aware home shown after the opening. Inside a Rullst project it
//! summarizes the project from local files; outside one it leads with project
//! creation and the docs. Menu entries live in [`actions`] as data so later
//! surfaces (for example a command palette) can reuse them.

mod actions;
mod git;
mod project;

pub(super) use actions::{HomeAction, home_entries};
pub(super) use project::{Database, Migrations, Project};
pub(crate) use project::{find_project, relative_root};

use super::palette::{self, ColorDepth, Rgb};
use std::io::{self, Write};
use std::path::Path;

pub(super) const START_HERE_URL: &str = "https://rullst.github.io/Rullst/book/start-here.html";
pub(super) const CLI_REFERENCE_URL: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html";

const LABEL_COLOR: Rgb = (150, 155, 175);
const VALUE_COLOR: Rgb = (240, 240, 248);
const MARGIN: &str = "  ";

/// Where `cargo rullst` was started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Home {
    Project(Box<Project>),
    Outside,
}

impl Home {
    /// Inspects the working directory and its ancestors.
    pub(super) fn detect() -> Self {
        let Ok(start) = std::env::current_dir() else {
            return Self::Outside;
        };
        Project::detect(&start, |name| std::env::var(name).ok())
            .map_or(Self::Outside, |project| Self::Project(Box::new(project)))
    }

    /// Project commands run at the project root even from a subdirectory;
    /// `new` always runs where the user is.
    pub(super) fn command_directory(&self, arguments: &[String]) -> Option<&Path> {
        match self {
            Self::Project(project)
                if project.relative_root.is_some()
                    && arguments.get(1).map(String::as_str) != Some("new") =>
            {
                Some(&project.root)
            }
            _ => None,
        }
    }
}

struct SummaryLine {
    label: &'static str,
    value: String,
    strong: bool,
}

fn line(label: &'static str, value: impl Into<String>) -> SummaryLine {
    SummaryLine {
        label,
        value: value.into(),
        strong: false,
    }
}

fn summary_lines(home: &Home) -> Vec<SummaryLine> {
    let Home::Project(project) = home else {
        return vec![
            line("Project", "none in this directory or its parents"),
            line("Start here", START_HERE_URL),
            line("CLI reference", CLI_REFERENCE_URL),
        ];
    };
    let mut lines = vec![SummaryLine {
        strong: true,
        ..line("Project", project.name.clone())
    }];
    if let Some(root) = &project.relative_root {
        lines.push(line("Root", root.clone()));
    }
    let features = if project.features.is_empty() {
        "none".to_string()
    } else {
        project.features.join(" · ")
    };
    lines.push(line("Features", features));
    lines.push(line(
        "Database",
        match &project.database {
            Database::Configured { kind, source } => format!("{kind} · {source}"),
            Database::NotConfigured => "not configured".to_string(),
            Database::Unreadable(reason) => (*reason).to_string(),
        },
    ));
    if let Some(Migrations { count, latest }) = &project.migrations {
        let value = match latest {
            Some(latest) => format!("{count} defined · latest {latest}"),
            None => "none yet".to_string(),
        };
        lines.push(line("Migrations", value));
    }
    if let Some(branch) = &project.git_branch {
        lines.push(line("Git branch", branch.clone()));
    }
    lines
}

fn paint(text: &str, color: Rgb, bold: bool, depth: ColorDepth) -> String {
    if depth == ColorDepth::None {
        return text.to_string();
    }
    let weight = if bold { "\x1b[1m" } else { "" };
    format!("{weight}{}{text}\x1b[0m", palette::foreground(color, depth))
}

/// The summary block, coloured at `depth`.
pub(super) fn write_summary(
    out: &mut impl Write,
    home: &Home,
    depth: ColorDepth,
) -> io::Result<()> {
    let lines = summary_lines(home);
    let width = lines.iter().map(|line| line.label.len()).max().unwrap_or(0) + 2;
    let margin = if depth == ColorDepth::None {
        ""
    } else {
        MARGIN
    };
    for SummaryLine {
        label,
        value,
        strong,
    } in lines
    {
        let label = format!("{label:<width$}");
        writeln!(
            out,
            "{margin}{}{}",
            paint(&label, LABEL_COLOR, false, depth),
            paint(&value, VALUE_COLOR, strong, depth)
        )?;
    }
    writeln!(out)
}

/// Non-interactive equivalents of the menu, for pipes, CI and dumb terminals.
pub(super) fn write_next_steps(out: &mut impl Write, home: &Home) -> io::Result<()> {
    let steps: &[(&str, &str)] = match home {
        Home::Project(_) => &[
            ("cargo rullst dev", "Start the dev server with hot reload"),
            ("cargo rullst dash", "Open the live development dashboard"),
            (
                "cargo rullst make:model <Name> -m",
                "Scaffold a model and its migration",
            ),
            ("cargo rullst db:migrate", "Run pending migrations"),
            ("cargo rullst doctor", "Check the toolchain and environment"),
            ("cargo rullst deploy", "Deploy with the guided PaaS flow"),
        ],
        Home::Outside => &[("cargo rullst new <name>", "Create a new Rullst application")],
    };
    match home {
        Home::Project(project) => match &project.relative_root {
            Some(root) => writeln!(out, "Next steps (from {root})")?,
            None => writeln!(out, "Next steps")?,
        },
        Home::Outside => writeln!(out, "Get started")?,
    }
    let width = steps
        .iter()
        .map(|(command, _)| command.len())
        .max()
        .unwrap_or(0)
        + 2;
    for (command, description) in steps {
        writeln!(out, "  {command:<width$}{description}")?;
    }
    writeln!(out)?;
    writeln!(out, "Run `cargo rullst --help` to list every command.")?;
    out.flush()
}

#[cfg(test)]
mod tests;
