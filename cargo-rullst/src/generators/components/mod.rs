//! `cargo rullst ui add <component>...` and `cargo rullst ui list`
//! (13.0.0-alpha.5): copies UI components into the application as Rust
//! source (`src/ui/<name>.rs`, registered in `src/ui/mod.rs`) and, for the
//! components with client behaviour, a same-origin ES module
//! (`static/ui/<name>.js`). Each component is a function returning
//! `rullst::html::Markup` with a small options struct, styled with Tailwind
//! utility classes; the application owns the copies. Running it again
//! changes nothing, and a copy that differs from this CLI's is never
//! replaced without `--force`.
//!
//! Tailwind: the classes need the project's Tailwind build. When the project
//! has none, an interactive run offers `cargo rullst add tailwind` first; any
//! other run copies the components (they work and stay accessible, unstyled)
//! and prints the command.

mod catalog;
mod plan;
#[cfg(test)]
mod render_tests;
mod report;
#[cfg(test)]
mod tests;

use crate::ui::error_report::{self, AlreadyReported, Friendly, ProjectRequired};
use crate::ui::style::Style;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::error::Error;
use std::path::Path;

const DOCS: &str = "https://rullst.github.io/Rullst/book/frontend.html#components";

/// The `ui` subcommand, attached in `crate::command`.
pub(crate) fn command() -> Command {
    let list = catalog::COMPONENTS
        .iter()
        .map(|component| format!("  {:<11}{}", component.name, component.summary))
        .collect::<Vec<_>>()
        .join("\n");
    Command::new("ui")
        .about("Copies UI components (html! + Tailwind, accessible, CSP-safe) into this project")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("list").about("Lists the components `ui add` can copy"))
        .subcommand(
            Command::new("add")
                .about("Copies components into src/ui/ (and their modules into static/ui/)")
                .long_about(format!(
                    "Copies components into the application as Rust source: src/ui/<name>.rs, \
                     declared in src/ui/mod.rs, and `mod ui;` in the crate root. Dialog, menu, \
                     tabs and toast also get a same-origin module in static/ui/, loaded by their \
                     markup with rullst::assets::url. Components are styled with Tailwind utility \
                     classes; without Tailwind an interactive run offers `cargo rullst add \
                     tailwind` first. Running it again changes nothing; a copy you changed is \
                     kept unless --force is given.\n\nComponents:\n{list}"
                ))
                .arg(
                    Arg::new("components")
                        .value_name("COMPONENT")
                        .num_args(1..)
                        .required_unless_present("all")
                        .value_parser(clap::builder::PossibleValuesParser::new(catalog::NAMES))
                        .help("The components to copy (the ones they use are added)"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("components")
                        .help("Copies every component"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(ArgAction::SetTrue)
                        .help("Replaces copies that differ from this CLI's"),
                )
                .arg(
                    Arg::new("dry_run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help("Prints the planned diff without writing anything"),
                ),
        )
}

/// What one `ui add` invocation asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub names: Vec<String>,
    pub force: bool,
    pub dry_run: bool,
}

impl Request {
    pub(crate) fn from_matches(matches: &ArgMatches) -> Self {
        let names = if matches.get_flag("all") {
            catalog::NAMES
                .iter()
                .map(|name| (*name).to_owned())
                .collect()
        } else {
            matches
                .get_many::<String>("components")
                .map(|names| names.cloned().collect())
                .unwrap_or_default()
        };
        Self {
            names,
            force: matches.get_flag("force"),
            dry_run: matches.get_flag("dry_run"),
        }
    }
}

/// Why `execute` stopped.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ComponentsError {
    #[error("unknown component `{0}`")]
    Unknown(String),
    #[error("{} differ from the copies this CLI ships (changed, or copied by another version)", .0.join(", "))]
    Modified(Vec<String>),
    #[error(transparent)]
    Files(#[from] plan::PlanError),
}

#[derive(Debug)]
pub(crate) enum Failure {
    Project,
    Components(ComponentsError),
}

/// What the report needs to know about the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Facts {
    /// The project builds Tailwind CSS (managed or its own setup).
    pub tailwind: bool,
    /// Tailwind through `cargo rullst add tailwind`, which scans `src/**/*.rs`.
    pub managed_tailwind: bool,
    /// The locked `rullst` has `rullst::assets::url` (13.0.0-alpha.5).
    pub supports_assets: bool,
}

/// The facts about the project at `root`, `None` outside a Rullst project.
pub(crate) fn facts(root: &Path) -> Option<Facts> {
    let project = crate::generators::add::frontend::project::Project::read(root)?;
    Some(Facts {
        tailwind: crate::generators::frontend::uses_tailwind(root),
        managed_tailwind: crate::generators::frontend::has_managed_tailwind(root),
        supports_assets: project.supports_assets,
    })
}

/// Plans and (unless `dry_run`) applies `request` at `root`; returns the
/// report to print.
pub(crate) fn execute(root: &Path, request: &Request, style: Style) -> Result<String, Failure> {
    let facts = facts(root).ok_or(Failure::Project)?;
    let (components, added) = catalog::resolve(request.names.iter().map(String::as_str))
        .map_err(|name| Failure::Components(ComponentsError::Unknown(name)))?;
    // A dry run shows the differences of changed copies as replacements.
    let plan = plan::plan(root, &components, request.force || request.dry_run)
        .map_err(|error| Failure::Components(error.into()))?;
    if !request.force && !request.dry_run && !plan.modified.is_empty() {
        return Err(Failure::Components(ComponentsError::Modified(
            plan.modified.clone(),
        )));
    }
    if !request.dry_run {
        plan::apply(root, &plan).map_err(|error| Failure::Components(error.into()))?;
    }
    Ok(report::render(&report::Inputs {
        components: &components,
        added: &added,
        plan: &plan,
        facts,
        dry_run: request.dry_run,
        force: request.force,
        style,
    }))
}

fn friendly(error: &ComponentsError) -> Friendly {
    let fix = match error {
        ComponentsError::Unknown(_) => {
            "Run `cargo rullst ui list` to see the components.".to_owned()
        }
        ComponentsError::Modified(_) => "Nothing was written. Keep your copy, or see the differences with --dry-run and replace it with this CLI's copy with --force.".to_owned(),
        ComponentsError::Files(plan::PlanError::Read { .. }) => "Check the file's permissions and that it is valid UTF-8, then run the command again.".to_owned(),
        ComponentsError::Files(plan::PlanError::Write { .. }) => "Check the file's permissions and free disk space; running the command again is safe.".to_owned(),
    };
    let title = match error {
        ComponentsError::Unknown(_) => "Unknown component",
        ComponentsError::Modified(_) => "A component file was changed",
        ComponentsError::Files(_) => "A project file could not be read or written",
    };
    Friendly {
        title: title.to_owned(),
        happened: error.to_string(),
        fix: Some(fix),
        docs: Some(DOCS),
    }
}

/// Whether `request` would stop before writing (an unknown name, a file
/// that cannot be read, a changed copy without `--force`).
pub(crate) fn refused(root: &Path, request: &Request) -> bool {
    let Ok((components, _)) = catalog::resolve(request.names.iter().map(String::as_str)) else {
        return true;
    };
    plan::plan(root, &components, false)
        .map_or(true, |plan| !request.force && !plan.modified.is_empty())
}

/// Whether the command may ask a question: interactive input and output,
/// outside CI.
#[cfg_attr(mutants, mutants::skip)]
fn interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::env::var_os("CI").is_none_or(|value| value.is_empty() || value == "false")
}

/// Offers `cargo rullst add tailwind` before copying into a project without
/// Tailwind; returns whether it ran.
#[cfg_attr(mutants, mutants::skip)]
fn offer_tailwind(root: &Path, style: Style) -> Result<bool, Box<dyn Error>> {
    let question = "The components are styled with Tailwind CSS, which this project does not use yet. Set it up now with `cargo rullst add tailwind` (downloads the pinned standalone CLI once, about 100 MB)?";
    if !dialoguer::Confirm::new()
        .with_prompt(question)
        .default(true)
        .interact()?
    {
        return Ok(false);
    }
    let Some(kit) = crate::generators::add::frontend::find("tailwind") else {
        return Ok(false);
    };
    crate::generators::add::frontend::run(root, kit, false, style)?;
    println!();
    Ok(true)
}

/// Runs `cargo rullst ui` in the current directory.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let style = Style::stdout();
    let Some(add) = matches.subcommand_matches("add") else {
        print!("{}", report::list(style));
        return Ok(());
    };
    let request = Request::from_matches(add);
    let root = std::env::current_dir()?;
    let Some(facts) = facts(&root) else {
        return Err(ProjectRequired.into());
    };
    // Checked first, so a copy that would be refused downloads nothing.
    if !facts.tailwind && !request.dry_run && !refused(&root, &request) && interactive() {
        offer_tailwind(&root, style)?;
    }
    match execute(&root, &request, style) {
        Ok(text) => {
            print!("{text}");
            Ok(())
        }
        Err(Failure::Project) => Err(ProjectRequired.into()),
        Err(Failure::Components(error)) => {
            eprint!(
                "{}",
                error_report::render(&friendly(&error), &[], None, false, Style::stderr())
            );
            Err(AlreadyReported::new(1, error.to_string()).into())
        }
    }
}
