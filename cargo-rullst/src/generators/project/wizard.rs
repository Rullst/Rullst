// cargo-rullst/src/generators/project/wizard.rs — The choices behind `cargo rullst new`:
// the deterministic flag profile and the interactive wizard.

pub(crate) mod catalog;
pub(crate) mod flow;
pub(crate) mod plan;
pub(crate) mod preview;
pub(crate) mod summary;
mod terminal;

use crate::blueprints::{
    BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use crate::generators::project::ProjectScaffoldOptions;
use crate::ui::screen::{Line, Terminal, Tone};
use plan::{Database, ProjectPlan};

const V12_ORM_PATTERN: &str = "Active Record";
const V12_FRONTEND_ENGINE: &str = "Zero-Bundle HTMX";

type WizardResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Optional persistence capabilities that complement the primary SQL ORM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyglotIntegration {
    /// Turso/libSQL edge SQL.
    Turso,
    /// MongoDB document storage.
    MongoDb,
    /// DuckDB OLAP and analytics.
    DuckDb,
    /// SurrealDB document and graph storage.
    SurrealDb,
    /// Qdrant bounded dense-vector storage.
    Qdrant,
}

impl PolyglotIntegration {
    /// Returns the `rullst-orm` feature selected by this integration.
    pub const fn orm_feature(self) -> &'static str {
        match self {
            Self::Turso => "turso",
            Self::MongoDb => "mongodb",
            Self::DuckDb => "duckdb",
            Self::SurrealDb => "surrealdb",
            Self::Qdrant => "qdrant",
        }
    }

    /// Returns the umbrella `rullst` feature selected by this integration.
    pub const fn rullst_feature(self) -> &'static str {
        match self {
            Self::Turso => "orm-turso",
            Self::MongoDb => "orm-mongodb",
            Self::DuckDb => "orm-duckdb",
            Self::SurrealDb => "orm-surrealdb",
            Self::Qdrant => "orm-qdrant",
        }
    }
}

pub struct ProjectWizardOptions {
    pub name: String,
    pub api: bool,
    pub db_provider: String,
    pub db_needed: bool,
    pub hot_reload: bool,
    pub blueprint_selection: usize,
    pub wants_ai: bool,
    pub wants_redis: bool,
    pub turso: bool,
    pub polyglot_integrations: Vec<PolyglotIntegration>,
    pub orm_pattern: String,
    pub frontend_engine: String,
}

/// What `cargo rullst new` was asked on the command line.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NewProjectRequest<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) options: ProjectScaffoldOptions,
    pub(crate) blueprint: Option<usize>,
    pub(crate) skip_initial_migration: bool,
    pub(crate) dry_run: bool,
}

/// The decided plan, or a cancelled wizard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Planned {
    /// `reviewed` is true when the user already saw the review screen.
    Ready {
        plan: ProjectPlan,
        reviewed: bool,
    },
    Cancelled,
}

fn invalid_input(message: &str) -> Box<dyn std::error::Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.to_string()).into()
}

/// Storage adapters selected by flags, in their stable manifest order.
pub(crate) fn requested_integrations(options: &ProjectScaffoldOptions) -> Vec<PolyglotIntegration> {
    [
        (
            options.turso || options.database == Some("Turso"),
            PolyglotIntegration::Turso,
        ),
        (options.mongodb, PolyglotIntegration::MongoDb),
        (options.duckdb, PolyglotIntegration::DuckDb),
        (options.surrealdb, PolyglotIntegration::SurrealDb),
        (options.qdrant, PolyglotIntegration::Qdrant),
    ]
    .into_iter()
    .filter_map(|(selected, integration)| selected.then_some(integration))
    .collect()
}

fn validate_request(request: &NewProjectRequest<'_>) -> WizardResult<()> {
    let options = &request.options;
    if options.hot_reload {
        return Err(invalid_input(
            "DLL hot reload is unavailable in v12: generate without --hot-reload and use `cargo rullst dev` for supervised process reload",
        ));
    }
    if options.database.is_some_and(|provider| {
        !matches!(
            provider,
            "Sqlite" | "Postgres" | "MySQL" | "MariaDB" | "Turso"
        )
    }) {
        return Err(invalid_input("unknown relational database provider"));
    }
    if request.blueprint.is_some_and(|id| {
        !matches!(
            id,
            BLANK_BLUEPRINT_ID
                | LMS_BLUEPRINT_ID
                | SAAS_BLUEPRINT_ID
                | BLOG_BLUEPRINT_ID
                | PORTFOLIO_BLUEPRINT_ID
                | ERP_BLUEPRINT_ID
        )
    }) {
        return Err(invalid_input("unknown public blueprint ID"));
    }
    Ok(())
}

fn check_blank_only_flags(
    plan: &ProjectPlan,
    options: &ProjectScaffoldOptions,
) -> WizardResult<()> {
    if plan.api && plan.blueprint != BLANK_BLUEPRINT_ID {
        return Err(invalid_input(
            "--api is available only for the blank blueprint",
        ));
    }
    if options.no_database && plan.blueprint != BLANK_BLUEPRINT_ID {
        return Err(invalid_input(
            "--no-database is available only for the blank blueprint",
        ));
    }
    Ok(())
}

/// The flag answers, with deterministic defaults for everything else.
fn initial_plan(request: &NewProjectRequest<'_>, requested: &[PolyglotIntegration]) -> ProjectPlan {
    let options = &request.options;
    ProjectPlan {
        name: request.name.unwrap_or("app").to_string(),
        blueprint: request.blueprint.unwrap_or(BLANK_BLUEPRINT_ID),
        api: options.api,
        database: if options.no_database {
            Database::None
        } else {
            Database::Provider(options.database.unwrap_or("Sqlite"))
        },
        ai: options.wants_ai,
        redis: options.wants_redis,
        docker: options.docker,
        nix: options.nix,
        buildah: options.buildah,
        requested: requested.to_vec(),
        add_ons: Vec::new(),
        skip_initial_migration: request.skip_initial_migration,
    }
}

/// Questions the flags already answered.
fn locked_questions(request: &NewProjectRequest<'_>) -> flow::Locked {
    let options = &request.options;
    flow::Locked {
        name: request.name.is_some(),
        // These flags exist only for the Blank starter, so they choose it.
        blueprint: request.blueprint.is_some()
            || options.api
            || options.no_database
            || options.database == Some("Turso"),
        application: options.api,
        database: options.database.is_some() || options.no_database,
        ai: options.wants_ai,
        redis: options.wants_redis,
        docker: options.docker || options.buildah,
        nix: options.nix,
    }
}

/// The wizard's starting point: flag answers locked, the name asked unless given.
fn interactive_setup(
    request: &NewProjectRequest<'_>,
    mut plan: ProjectPlan,
    offer_packaging: bool,
    port: u16,
) -> flow::Setup {
    let locked = locked_questions(request);
    if !locked.name {
        plan.name.clear();
    }
    flow::Setup {
        plan,
        locked,
        offer_packaging,
        dry_run: request.dry_run,
        port,
    }
}

fn needs_terminal() -> Box<dyn std::error::Error> {
    invalid_input(
        "`cargo rullst new` asks its questions only in an interactive terminal; add --default (with --blueprint, --database and other flags as needed) to create a project without prompts, and --dry-run to preview it first",
    )
}

/// Decides what to create: from the flags with `--default`, otherwise with
/// the interactive wizard. Without an interactive terminal it never prompts.
pub(crate) fn plan_project(
    request: &NewProjectRequest<'_>,
    requested: &[PolyglotIntegration],
    terminal: &Terminal,
    offer_packaging: bool,
) -> WizardResult<Planned> {
    validate_request(request)?;
    let plan = initial_plan(request, requested);
    if request.options.use_defaults {
        check_blank_only_flags(&plan, &request.options)?;
        return Ok(Planned::Ready {
            plan,
            reviewed: false,
        });
    }
    if !terminal.interactive() {
        return Err(needs_terminal());
    }
    check_blank_only_flags(&plan, &request.options)?;
    if plan.database == Database::Provider("Turso") && plan.blueprint != BLANK_BLUEPRINT_ID {
        return Err(invalid_input(
            "Turso-primary currently requires the blank starter while the SQLx-specific blueprints are being ported",
        ));
    }
    if let Some(name) = request.name
        && std::path::Path::new(name).exists()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("directory '{name}' already exists"),
        )
        .into());
    }

    terminal.print(&[Line::new()
        .push(Tone::Accent, "◆ ")
        .push(Tone::Brand, flow::TITLE)
        .push(
            Tone::Muted,
            "  A few questions, then a review before anything is written.",
        )])?;
    let port = super::next_steps::resolve_port(std::env::var("PORT").ok().as_deref(), None, None);
    let setup = interactive_setup(request, plan, offer_packaging, port);
    let mut ui = terminal::TerminalUi::new(*terminal);
    Ok(match flow::run(setup, &mut ui, preview::rendered_files)? {
        flow::Outcome::Create(plan) => Planned::Ready {
            plan,
            reviewed: true,
        },
        flow::Outcome::Cancelled => Planned::Cancelled,
    })
}

pub fn run_project_wizard(
    name_arg: Option<&str>,
    api: bool,
    use_defaults: bool,
    turso: bool,
) -> Result<ProjectWizardOptions, Box<dyn std::error::Error>> {
    let integrations = if turso {
        vec![PolyglotIntegration::Turso]
    } else {
        Vec::new()
    };
    run_project_wizard_with_blueprint(
        name_arg,
        ProjectScaffoldOptions {
            api,
            use_defaults,
            turso,
            ..ProjectScaffoldOptions::default()
        },
        &integrations,
        None,
    )
}

/// The library entry point: the wizard's answers without creating anything.
/// Packaging questions are left to the caller here.
pub(crate) fn run_project_wizard_with_blueprint(
    name_arg: Option<&str>,
    options: ProjectScaffoldOptions,
    requested_integrations: &[PolyglotIntegration],
    blueprint_override: Option<usize>,
) -> WizardResult<ProjectWizardOptions> {
    let request = NewProjectRequest {
        name: name_arg,
        options,
        blueprint: blueprint_override,
        ..NewProjectRequest::default()
    };
    match plan_project(&request, requested_integrations, &Terminal::detect(), false)? {
        Planned::Ready { plan, .. } => Ok(plan.wizard_options()),
        Planned::Cancelled => Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "project creation was cancelled",
        )
        .into()),
    }
}

#[cfg(test)]
#[path = "wizard_tests.rs"]
mod tests;
