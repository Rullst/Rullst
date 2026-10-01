//! Pure renderings of a plan: the review summary and the compact file tree.

use super::catalog::{blueprint, database_name, feature_name};
use super::plan::ProjectPlan;
use crate::blueprints::BLANK_BLUEPRINT_ID;
use crate::generators::project::ProjectIdentity;
use crate::ui::screen::{Line, Tone};
use std::collections::BTreeMap;

const LABEL_WIDTH: usize = 13;
/// Character budget for a tree line inside a summary or a preview panel.
pub(crate) const TREE_WIDTH: usize = 72;

#[derive(Default)]
struct Dir {
    dirs: BTreeMap<String, Dir>,
    files: Vec<String>,
}

impl Dir {
    fn insert(&mut self, path: &str) {
        match path.split_once('/') {
            Some((head, rest)) => self.dirs.entry(head.to_string()).or_default().insert(rest),
            None => self.files.push(path.to_string()),
        }
    }

    fn count(&self) -> usize {
        self.files.len() + self.dirs.values().map(Dir::count).sum::<usize>()
    }

    /// A directory holding a single file collapses into `dir/file`.
    fn single_file(&self) -> Option<String> {
        match (self.files.as_slice(), self.dirs.len()) {
            ([file], 0) => Some(file.clone()),
            ([], 1) => self
                .dirs
                .iter()
                .find_map(|(name, dir)| dir.single_file().map(|file| format!("{name}/{file}"))),
            _ => None,
        }
    }

    /// Subdirectories worth a line of their own, and the files of this level
    /// (including collapsed single-file directories), visible names first.
    fn split(&self) -> (Vec<(&String, &Dir)>, Vec<String>) {
        let mut dirs = Vec::new();
        let mut files = self.files.clone();
        for (name, dir) in &self.dirs {
            match dir.single_file() {
                Some(file) => files.push(format!("{name}/{file}")),
                None => dirs.push((name, dir)),
            }
        }
        files.sort_by_key(|name| (name.starts_with('.'), name.to_ascii_lowercase()));
        (dirs, files)
    }
}

fn plural(count: usize) -> String {
    if count == 1 {
        "1 file".to_string()
    } else {
        format!("{count} files")
    }
}

/// `names` joined by two spaces within `room` characters, ending with
/// `+N more` when some do not fit.
fn join_fitting(names: &[String], room: usize) -> String {
    let mut joined = String::new();
    for (index, name) in names.iter().enumerate() {
        let candidate = if joined.is_empty() {
            name.clone()
        } else {
            format!("{joined}  {name}")
        };
        let rest = names.len() - index - 1;
        let reserve = if rest == 0 {
            0
        } else {
            format!("  +{rest} more").len()
        };
        if candidate.chars().count() + reserve > room && !joined.is_empty() {
            return format!("{joined}  +{} more", names.len() - index);
        }
        joined = candidate;
    }
    joined
}

fn branch(prefix: &str, last: bool) -> Line {
    Line::new().push(
        Tone::Muted,
        format!("{prefix}{}", if last { "└── " } else { "├── " }),
    )
}

fn files_line(prefix: &str, files: &[String], width: usize) -> Line {
    let room = width.saturating_sub(prefix.chars().count() + 4);
    branch(prefix, true).push(Tone::Value, join_fitting(files, room))
}

/// A compact tree of `files` (relative `/` paths) under `root/`: top-level
/// directories one per line, a directory with subdirectories opened one
/// level, and the files of a level sharing one line.
pub(crate) fn tree_lines(root: &str, files: &[String], width: usize) -> Vec<Line> {
    let mut tree = Dir::default();
    for file in files {
        tree.insert(file);
    }
    let mut lines = vec![
        Line::new()
            .push(Tone::Directory, format!("{root}/"))
            .push(Tone::Muted, format!("  {}", plural(tree.count()))),
    ];
    let (dirs, root_files) = tree.split();
    let entries = dirs.len() + usize::from(!root_files.is_empty());
    for (index, (name, dir)) in dirs.iter().enumerate() {
        let last = index + 1 == entries;
        let (subdirs, own_files) = dir.split();
        if subdirs.is_empty() {
            lines.push(
                branch("", last)
                    .push(Tone::Directory, format!("{name}/"))
                    .push(Tone::Muted, format!("  {}", plural(dir.count()))),
            );
            continue;
        }
        lines.push(branch("", last).push(Tone::Directory, format!("{name}/")));
        let prefix = if last { "    " } else { "│   " };
        let children = subdirs.len() + usize::from(!own_files.is_empty());
        for (position, (child, contents)) in subdirs.iter().enumerate() {
            lines.push(
                branch(prefix, position + 1 == children)
                    .push(Tone::Directory, format!("{child}/"))
                    .push(Tone::Muted, format!("  {}", plural(contents.count()))),
            );
        }
        if !own_files.is_empty() {
            lines.push(files_line(prefix, &own_files, width));
        }
    }
    if !root_files.is_empty() {
        lines.push(files_line("", &root_files, width));
    }
    lines
}

/// `value` as one shell word: unchanged when it is plainly safe, otherwise
/// single-quoted.
pub(crate) fn shell_word(value: &str) -> String {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-./".contains(character));
    if safe {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

fn row(label: &str, value: Line) -> Line {
    let mut line = Line::new().push(Tone::Label, format!("{label:<LABEL_WIDTH$}"));
    line.spans.extend(value.spans);
    line
}

fn indented(line: Line) -> Line {
    row("", line)
}

/// The application kind a plan generates.
pub(crate) fn application_name(plan: &ProjectPlan) -> &'static str {
    if plan.blueprint == BLANK_BLUEPRINT_ID && plan.api {
        "JSON API (headless, no HTML)"
    } else {
        "Full-stack web app (html! + HTMX)"
    }
}

/// The commands `new` runs after writing files, with what each one does.
pub(crate) fn commands_to_run(plan: &ProjectPlan) -> Vec<(&'static str, &'static str)> {
    if plan.db_needed() && !plan.skip_initial_migration {
        vec![(
            "cargo run -q -- db:migrate",
            "first build and initial migration; a cold build can take minutes",
        )]
    } else {
        Vec::new()
    }
}

/// The review content: the answers, the file tree and what happens next.
pub(crate) struct Summary {
    pub(crate) answers: Vec<Line>,
    pub(crate) files: Vec<Line>,
    pub(crate) commands: Vec<Line>,
}

impl Summary {
    /// Everything in reading order.
    pub(crate) fn lines(self) -> Vec<Line> {
        let mut lines = self.answers;
        lines.extend(self.files);
        lines.extend(self.commands);
        lines
    }
}

/// The review of `plan`: choices, the file tree and the commands that will run.
pub(crate) fn summary(plan: &ProjectPlan, files: Option<&[String]>, port: u16) -> Summary {
    let identity = ProjectIdentity::from_destination(&plan.name).ok();
    let package = identity
        .as_ref()
        .map_or(plan.name.as_str(), ProjectIdentity::package_name);
    let destination = identity.as_ref().map_or_else(
        || plan.name.clone(),
        |identity| identity.destination_path().display().to_string(),
    );
    let shown_destination = if destination == package {
        format!("./{package}")
    } else {
        destination.clone()
    };

    let mut answers = vec![row(
        "Project",
        Line::new()
            .push(Tone::Strong, package)
            .push(Tone::Muted, format!("  in {shown_destination}")),
    )];
    if let Some(info) = blueprint(plan.blueprint) {
        answers.push(row(
            "Blueprint",
            Line::new()
                .push(Tone::Value, info.name)
                .push(Tone::Muted, format!(" · {}", info.summary)),
        ));
    }
    answers.push(row(
        "Application",
        Line::new().push(Tone::Value, application_name(plan)),
    ));
    let database = if plan.db_needed() {
        database_name(plan.database)
    } else {
        "none"
    };
    answers.push(row("Database", Line::new().push(Tone::Value, database)));
    let mut features: Vec<&str> = plan.features().into_iter().map(feature_name).collect();
    if plan.buildah {
        features.push("Buildah script");
    }
    let features = if features.is_empty() {
        "none".to_string()
    } else {
        features.join(", ")
    };
    answers.push(row("Features", Line::new().push(Tone::Value, features)));

    let mut tree = Vec::new();
    match files {
        Some(files) => {
            tree.push(row(
                "Files",
                Line::new().push(
                    Tone::Value,
                    format!("{} will be written", plural(files.len())),
                ),
            ));
            for line in tree_lines(package, files, TREE_WIDTH - LABEL_WIDTH) {
                tree.push(indented(line));
            }
        }
        None => tree.push(row(
            "Files",
            Line::new().push(Tone::Warning, "preview unavailable"),
        )),
    }

    let mut commands = Vec::new();
    let planned = commands_to_run(plan);
    if planned.is_empty() {
        commands.push(row(
            "Runs",
            Line::new().push(Tone::Value, "no commands; files only"),
        ));
    }
    for (index, (command, purpose)) in planned.into_iter().enumerate() {
        let label = if index == 0 { "Runs" } else { "" };
        commands.push(row(label, Line::new().push(Tone::Value, command)));
        commands.push(indented(Line::new().push(Tone::Muted, purpose)));
    }
    commands.push(row(
        "Then",
        Line::new().push(
            Tone::Value,
            format!("cd {} && cargo rullst dev", shell_word(&destination)),
        ),
    ));
    let visit = if plan.blueprint == BLANK_BLUEPRINT_ID && plan.api {
        format!("curl http://127.0.0.1:{port}/")
    } else {
        format!("open http://127.0.0.1:{port}")
    };
    commands.push(indented(Line::new().push(Tone::Value, visit)));
    Summary {
        answers,
        files: tree,
        commands,
    }
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
