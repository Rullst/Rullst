//! The interactive `cargo rullst new` state machine: name → blueprint →
//! application kind (Blank) → database → features → review. Questions that
//! flags already answered are skipped, Esc returns to the previous question
//! and the review screen creates, goes back or cancels.

use super::catalog::{BLUEPRINTS, FEATURES, Feature, database_options};
use super::plan::{Database, ProjectPlan};
use super::summary::{TREE_WIDTH, summary, tree_lines};
use super::{PolyglotIntegration, WizardResult};
use crate::blueprints::BLANK_BLUEPRINT_ID;
use crate::generators::project::ProjectIdentity;
use crate::ui::screen::{Answer, Choice, Line, Screen, Selection, Tone};

pub(crate) const TITLE: &str = "New Rullst app";

/// The wizard's questions, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Name,
    Blueprint,
    Application,
    Database,
    Features,
    Review,
}

const ORDER: [Step; 6] = [
    Step::Name,
    Step::Blueprint,
    Step::Application,
    Step::Database,
    Step::Features,
    Step::Review,
];

/// Questions answered by flags; the wizard never asks them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Locked {
    pub(crate) name: bool,
    pub(crate) blueprint: bool,
    pub(crate) application: bool,
    pub(crate) database: bool,
    pub(crate) ai: bool,
    pub(crate) redis: bool,
    pub(crate) docker: bool,
    pub(crate) nix: bool,
}

/// The starting answers and what the flags fixed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Setup {
    pub(crate) plan: ProjectPlan,
    pub(crate) locked: Locked,
    /// Offer Dockerfile/Nix (only the CLI path can act on them).
    pub(crate) offer_packaging: bool,
    pub(crate) dry_run: bool,
    /// The port shown in the review's "Then" line.
    pub(crate) port: u16,
}

/// How the wizard ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Create(ProjectPlan),
    Cancelled,
}

/// The terminal side of the wizard; tests script it.
pub(crate) trait WizardUi {
    /// Reads a project name; `initial` is the previous answer, if any.
    fn ask_name(&mut self, initial: &str) -> WizardResult<String>;
    /// Explains why a typed name was rejected.
    fn reject_name(&mut self, reason: &str);
    fn choose(&mut self, screen: &Screen) -> WizardResult<Answer>;
}

/// A simple interactive name: letters, digits, `_` and `-`, starting with a
/// letter, not a Rust keyword and not an existing path.
pub(crate) fn validate_name(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("Enter a project name.");
    }
    if name.contains(char::is_whitespace) {
        return Err("Spaces are not allowed in the project name.");
    }
    if name.starts_with(|first: char| first.is_ascii_digit()) {
        return Err("The project name cannot start with a number.");
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err("Only letters, numbers, underscores and dashes are allowed.");
    }
    if !name.starts_with(|first: char| first.is_ascii_alphabetic()) {
        return Err("The project name must start with a letter.");
    }
    if ProjectIdentity::from_destination(name).is_err() {
        return Err("That name is reserved by Rust; choose another one.");
    }
    if std::path::Path::new(name).exists() {
        return Err("A file or directory with that name already exists here.");
    }
    Ok(name.to_string())
}

fn invalid_answer() -> Box<dyn std::error::Error> {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "selection was outside the displayed choices",
    )
    .into()
}

enum Move {
    Forward,
    Back,
    Finish(Outcome),
}

/// Runs the wizard. `preview` lists the files a plan generates.
pub(crate) fn run<U, P>(setup: Setup, ui: &mut U, preview: P) -> WizardResult<Outcome>
where
    U: WizardUi,
    P: FnMut(&ProjectPlan) -> Option<Vec<String>>,
{
    Flow {
        setup,
        previews: Vec::new(),
        preview,
    }
    .run(ui)
}

struct Flow<P> {
    setup: Setup,
    previews: Vec<(ProjectPlan, Option<Vec<String>>)>,
    preview: P,
}

impl<P: FnMut(&ProjectPlan) -> Option<Vec<String>>> Flow<P> {
    fn applies(&self, step: Step) -> bool {
        let locked = self.setup.locked;
        match step {
            Step::Name => !locked.name,
            Step::Blueprint => !locked.blueprint,
            Step::Application => {
                self.setup.plan.blueprint == BLANK_BLUEPRINT_ID && !locked.application
            }
            Step::Database => !locked.database,
            Step::Features => !self.offered_features().is_empty(),
            Step::Review => true,
        }
    }

    fn next_after(&self, step: Option<Step>) -> Step {
        let start = step
            .and_then(|step| ORDER.iter().position(|candidate| *candidate == step))
            .map_or(0, |index| index + 1);
        ORDER
            .iter()
            .skip(start)
            .copied()
            .find(|step| self.applies(*step))
            .unwrap_or(Step::Review)
    }

    /// Features the user may still toggle: not fixed by a flag and not the
    /// Turso primary itself.
    fn offered_features(&self) -> Vec<(Feature, &'static str, &'static str)> {
        let plan = &self.setup.plan;
        let locked = self.setup.locked;
        FEATURES
            .into_iter()
            .filter(|(feature, ..)| match feature {
                Feature::Ai => !locked.ai,
                Feature::Redis => !locked.redis,
                Feature::Docker => self.setup.offer_packaging && !locked.docker,
                Feature::Nix => self.setup.offer_packaging && !locked.nix,
                Feature::Storage(integration) => {
                    !plan.requested.contains(integration)
                        && !(*integration == PolyglotIntegration::Turso
                            && plan.database == Database::Provider("Turso"))
                }
            })
            .collect()
    }

    fn files(&mut self, plan: &ProjectPlan) -> Option<Vec<String>> {
        if let Some((_, files)) = self.previews.iter().find(|(known, _)| known == plan) {
            return files.clone();
        }
        let files = (self.preview)(plan);
        self.previews.push((plan.clone(), files.clone()));
        files
    }

    fn crumb(&self, step: Step, label: &str) -> String {
        let steps: Vec<Step> = ORDER
            .into_iter()
            .filter(|candidate| self.applies(*candidate))
            .collect();
        let position = steps.iter().position(|candidate| *candidate == step);
        match position {
            Some(index) => format!("Step {} of {} · {label}", index + 1, steps.len()),
            None => label.to_string(),
        }
    }

    fn screen(&self, step: Step, label: &str, question: &str, choices: Vec<Choice>) -> Screen {
        Screen {
            title: TITLE.to_string(),
            crumb: self.crumb(step, label),
            body: Vec::new(),
            question: question.to_string(),
            choices,
            selection: Selection::One { initial: 0 },
            can_go_back: true,
            echo: Some(label.to_string()),
        }
    }

    fn run<U: WizardUi>(mut self, ui: &mut U) -> WizardResult<Outcome> {
        let mut history: Vec<Step> = Vec::new();
        let mut step = self.next_after(None);
        loop {
            let movement = match step {
                Step::Name => self.name(ui)?,
                Step::Blueprint => self.blueprint(ui)?,
                Step::Application => self.application(ui)?,
                Step::Database => self.database(ui)?,
                Step::Features => self.features(ui)?,
                Step::Review => self.review(ui)?,
            };
            match movement {
                Move::Forward => {
                    history.push(step);
                    step = self.next_after(Some(step));
                }
                // Esc on the first question has nowhere to go: ask it again.
                Move::Back => {
                    if let Some(previous) = history.pop() {
                        step = previous;
                    }
                }
                Move::Finish(outcome) => return Ok(outcome),
            }
        }
    }

    fn name<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        loop {
            let raw = ui.ask_name(&self.setup.plan.name)?;
            match validate_name(&raw) {
                Ok(name) => {
                    self.setup.plan.name = name;
                    return Ok(Move::Forward);
                }
                Err(reason) => ui.reject_name(reason),
            }
        }
    }

    fn blueprint<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        let mut choices = Vec::new();
        for info in BLUEPRINTS {
            let mut candidate = self.setup.plan.clone();
            candidate.blueprint = info.id;
            candidate.fit_blueprint();
            let detail = match self.files(&candidate) {
                Some(files) => {
                    let package = ProjectIdentity::from_destination(&candidate.name).map_or_else(
                        |_| candidate.name.clone(),
                        |id| id.package_name().to_string(),
                    );
                    tree_lines(&package, &files, TREE_WIDTH)
                }
                None => vec![Line::new().push(Tone::Warning, "Preview unavailable")],
            };
            choices.push(Choice {
                detail,
                ..Choice::new(info.name, info.summary)
            });
        }
        let mut screen = self.screen(
            Step::Blueprint,
            "Blueprint",
            "Which starter should Rullst generate?",
            choices,
        );
        screen.selection = Selection::One {
            initial: BLUEPRINTS
                .iter()
                .position(|info| info.id == self.setup.plan.blueprint)
                .unwrap_or(0),
        };
        match ui.choose(&screen)? {
            Answer::One(index) => {
                let info = BLUEPRINTS.get(index).ok_or_else(invalid_answer)?;
                self.setup.plan.blueprint = info.id;
                self.setup.plan.fit_blueprint();
                Ok(Move::Forward)
            }
            Answer::Back => Ok(Move::Back),
            Answer::Many(_) => Err(invalid_answer()),
        }
    }

    fn application<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        let choices = vec![
            Choice::new(
                "Full-stack web app",
                "server-rendered html! pages with HTMX",
            ),
            Choice::new("JSON API", "headless REST endpoints, no HTML"),
        ];
        let mut screen = self.screen(
            Step::Application,
            "Application",
            "What are you building?",
            choices,
        );
        screen.selection = Selection::One {
            initial: usize::from(self.setup.plan.api),
        };
        match ui.choose(&screen)? {
            Answer::One(index @ (0 | 1)) => {
                self.setup.plan.api = index == 1;
                Ok(Move::Forward)
            }
            Answer::Back => Ok(Move::Back),
            _ => Err(invalid_answer()),
        }
    }

    fn database<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        let options = database_options(self.setup.plan.blueprint);
        let choices = options
            .iter()
            .map(|option| Choice::new(option.name, option.hint))
            .collect();
        let mut screen = self.screen(
            Step::Database,
            "Database",
            "Which primary database?",
            choices,
        );
        screen.selection = Selection::One {
            initial: options
                .iter()
                .position(|option| option.database == self.setup.plan.database)
                .unwrap_or(0),
        };
        match ui.choose(&screen)? {
            Answer::One(index) => {
                let option = options.get(index).ok_or_else(invalid_answer)?;
                self.setup.plan.database = option.database;
                Ok(Move::Forward)
            }
            Answer::Back => Ok(Move::Back),
            Answer::Many(_) => Err(invalid_answer()),
        }
    }

    fn features<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        let offered = self.offered_features();
        let choices = offered
            .iter()
            .map(|(_, name, hint)| Choice::new(*name, *hint))
            .collect();
        let mut screen = self.screen(
            Step::Features,
            "Features",
            "Optional features (zero or more)",
            choices,
        );
        screen.selection = Selection::Many {
            checked: offered
                .iter()
                .map(|(feature, ..)| self.setup.plan.has(*feature))
                .collect(),
        };
        match ui.choose(&screen)? {
            Answer::Many(picked) => {
                for (index, (feature, ..)) in offered.iter().enumerate() {
                    self.setup.plan.set(*feature, picked.contains(&index));
                }
                Ok(Move::Forward)
            }
            Answer::Back => Ok(Move::Back),
            Answer::One(_) => Err(invalid_answer()),
        }
    }

    fn review<U: WizardUi>(&mut self, ui: &mut U) -> WizardResult<Move> {
        let plan = self.setup.plan.clone();
        let files = self.files(&plan);
        let review = summary(&plan, files.as_deref(), self.setup.port);
        let mut create = if self.setup.dry_run {
            Choice::new("Finish the dry run", "show this plan only; create nothing")
        } else {
            Choice::new("Create the project", "")
        };
        // The tree is the detail panel, so a short terminal shrinks it first.
        create.detail = review.files;
        let mut screen = self.screen(
            Step::Review,
            "Review",
            "Ready?",
            vec![
                create,
                Choice::new("Back", "change an answer"),
                Choice::new("Cancel", "exit without creating anything"),
            ],
        );
        screen.body = review.answers;
        screen.body.extend(review.commands);
        screen.echo = None;
        match ui.choose(&screen)? {
            Answer::One(0) => Ok(Move::Finish(Outcome::Create(plan))),
            Answer::One(1) | Answer::Back => Ok(Move::Back),
            Answer::One(2) => Ok(Move::Finish(Outcome::Cancelled)),
            _ => Err(invalid_answer()),
        }
    }
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
