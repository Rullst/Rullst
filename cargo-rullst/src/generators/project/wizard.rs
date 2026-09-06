// cargo-rullst/src/generators/project/wizard.rs — Interactive project creation wizard.

use colored::*;

use crate::blueprints::{
    BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use crate::generators::project::ProjectScaffoldOptions;

const SQLX_DATABASE_OPTIONS: [(&str, &str); 4] = [
    ("SQLite (zero setup; recommended for a first run)", "Sqlite"),
    ("Postgres (requires localhost:5432)", "Postgres"),
    ("MySQL (requires localhost:3306)", "MySQL"),
    (
        "MariaDB (MySQL protocol; separately contract-tested)",
        "MariaDB",
    ),
];

const BLANK_DATABASE_OPTIONS: [(&str, &str); 5] = [
    SQLX_DATABASE_OPTIONS[0],
    SQLX_DATABASE_OPTIONS[1],
    SQLX_DATABASE_OPTIONS[2],
    SQLX_DATABASE_OPTIONS[3],
    ("Turso / libSQL (primary edge SQL)", "Turso"),
];

const V12_ORM_PATTERN: &str = "Active Record";
const V12_FRONTEND_ENGINE: &str = "Zero-Bundle HTMX";

fn primary_database_options(blueprint_selection: usize) -> &'static [(&'static str, &'static str)] {
    if blueprint_selection == BLANK_BLUEPRINT_ID {
        &BLANK_DATABASE_OPTIONS
    } else {
        &SQLX_DATABASE_OPTIONS
    }
}

const fn should_prompt_project_profile(_has_positional_name: bool, use_defaults: bool) -> bool {
    !use_defaults
}

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

const OPTIONAL_STORAGE_OPTIONS: [(&str, PolyglotIntegration); 5] = [
    (
        "Turso / libSQL adapter (add-on; application integration remains explicit in v12)",
        PolyglotIntegration::Turso,
    ),
    ("MongoDB (document CRUD)", PolyglotIntegration::MongoDb),
    ("DuckDB (in-process analytics)", PolyglotIntegration::DuckDb),
    (
        "SurrealDB (documents + read-only graph queries)",
        PolyglotIntegration::SurrealDb,
    ),
    ("Qdrant (vector search)", PolyglotIntegration::Qdrant),
];

fn available_optional_storage_options(
    selected: &[PolyglotIntegration],
) -> Vec<(&'static str, PolyglotIntegration)> {
    OPTIONAL_STORAGE_OPTIONS
        .into_iter()
        .filter(|(_, integration)| !selected.contains(integration))
        .collect()
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

pub(crate) fn run_project_wizard_with_blueprint(
    name_arg: Option<&str>,
    options: ProjectScaffoldOptions,
    requested_integrations: &[PolyglotIntegration],
    blueprint_override: Option<usize>,
) -> Result<ProjectWizardOptions, Box<dyn std::error::Error>> {
    if options.hot_reload {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "DLL hot reload is unavailable in v12: generate without --hot-reload and use `cargo rullst dev` for supervised process reload",
        )
        .into());
    }
    let prompt_project_profile =
        should_prompt_project_profile(name_arg.is_some(), options.use_defaults);
    let mut api = options.api;
    let db_provider_override = options.database;
    if db_provider_override.is_some_and(|provider| {
        !matches!(
            provider,
            "Sqlite" | "Postgres" | "MySQL" | "MariaDB" | "Turso"
        )
    }) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unknown relational database provider",
        )
        .into());
    }
    if blueprint_override.is_some_and(|id| {
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
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unknown public blueprint ID",
        )
        .into());
    }

    if !prompt_project_profile {
        let name = name_arg.unwrap_or("app").to_string();
        let blueprint_selection = blueprint_override.unwrap_or(BLANK_BLUEPRINT_ID);
        let db_provider = db_provider_override.unwrap_or("Sqlite").to_string();
        if api && blueprint_selection != BLANK_BLUEPRINT_ID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--api is available only for the blank blueprint",
            )
            .into());
        }
        if options.no_database && blueprint_selection != BLANK_BLUEPRINT_ID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--no-database is available only for the blank blueprint",
            )
            .into());
        }
        if db_provider == "Turso" && options.hot_reload {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Turso-primary does not support the generated hot-reload profile",
            )
            .into());
        }
        return Ok(ProjectWizardOptions {
            name,
            api,
            orm_pattern: if db_provider == "Turso" {
                "Turso Active Record"
            } else {
                V12_ORM_PATTERN
            }
            .to_string(),
            db_provider,
            db_needed: !options.no_database,
            hot_reload: options.hot_reload,
            blueprint_selection,
            wants_ai: options.wants_ai,
            wants_redis: options.wants_redis,
            turso: requested_integrations.contains(&PolyglotIntegration::Turso),
            polyglot_integrations: requested_integrations.to_vec(),
            frontend_engine: V12_FRONTEND_ENGINE.to_string(),
        });
    }

    let theme = dialoguer::theme::ColorfulTheme::default();

    let name = match name_arg {
        Some(n) => n.to_string(),
        None => {
            loop {
                let val: String = dialoguer::Input::with_theme(&theme)
                .with_prompt("🚀 What's the New App Name? (lowercase, no spaces, must start with a letter)")
                .interact_text()?;
                let val_trim = val.trim();
                if val_trim.is_empty() {
                    continue;
                }
                if val_trim.contains(' ') {
                    println!(
                        "{}",
                        "❌ Spaces are not allowed in the project name. Please try again.".red()
                    );
                    continue;
                }
                if val_trim
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_digit())
                {
                    println!(
                        "{}",
                        "❌ The project name cannot start with a number. Please try again.".red()
                    );
                    continue;
                }
                if !val_trim
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    println!("{}", "❌ Only letters, numbers, underscores, and dashes are allowed. Please try again.".red());
                    continue;
                }
                break val_trim.to_string();
            }
        }
    };

    let mut db_provider = "Sqlite".to_string();
    let mut db_needed = true;
    let hot_reload = false;
    let mut blueprint_selection = blueprint_override.unwrap_or(BLANK_BLUEPRINT_ID);
    let mut polyglot_integrations = requested_integrations.to_vec();

    // A positional name replaces only the name prompt. `--default` is the
    // explicit contract for skipping the interactive project-profile choices.
    if prompt_project_profile {
        let portfolio_title = format!(
            "Portfolio 🔥 (showcase for Rullst/AI developers) - {}",
            "HOT".bright_red().bold()
        );
        let blueprint_choices = vec![
            "Blank Starter (Minimal HTMX counter; Nexus CMS is not included)".to_string(),
            "LMS Platform (Courses, lessons, video player, HTMX integration)".to_string(),
            "SaaS App Starter (Authentication + Stripe payments billing template)".to_string(),
            "Blog / Press (Static site generator pre-wired with Nexus CMS)".to_string(),
            portfolio_title,
            "ERP Pocket (Inventory, stock management, orders tracker, auto-CMS)".to_string(),
        ];
        if blueprint_override.is_none() {
            blueprint_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("👉 Select a Starter Blueprint")
                .default(0)
                .items(&blueprint_choices)
                .interact()?;
        }

        if api && blueprint_selection != BLANK_BLUEPRINT_ID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--api is available only for the blank blueprint",
            )
            .into());
        }

        if blueprint_selection == BLANK_BLUEPRINT_ID {
            let build_options = &[
                "Full-Stack Web App (SaaS, Portfolio, Blog, Etc)",
                "Headless REST API",
            ];
            let build_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("🏗️ What would you like to build?")
                .default(0)
                .items(&build_options[..])
                .interact()?;
            api = build_selection == 1;

            db_needed = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("🗄️ Will your project need a Database?")
                .default(true)
                .interact()?;
        } else {
            db_needed = true;
        }

        if db_needed {
            let db_options = primary_database_options(blueprint_selection);
            let db_labels = db_options
                .iter()
                .map(|(label, _)| *label)
                .collect::<Vec<_>>();
            let db_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt(
                    "💾 Select the primary DB (network choices need a running local server)",
                )
                .default(0)
                .items(&db_labels)
                .interact()?;
            db_provider = db_options
                .get(db_selection)
                .map(|(_, provider)| (*provider).to_string())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "database selection was outside the displayed choices",
                    )
                })?;
            if db_provider == "Turso"
                && !polyglot_integrations.contains(&PolyglotIntegration::Turso)
            {
                polyglot_integrations.push(PolyglotIntegration::Turso);
            }
        }

        let persistence_options = available_optional_storage_options(&polyglot_integrations);
        if !persistence_options.is_empty() {
            let labels = persistence_options
                .iter()
                .map(|(label, _)| *label)
                .collect::<Vec<_>>();
            let persistence_selection = dialoguer::MultiSelect::with_theme(&theme)
                .with_prompt(
                    "🧩 Optional storage add-ons (select zero or more; Space toggles, Enter confirms)",
                )
                .items(&labels)
                .interact()?;
            for selected in persistence_selection {
                let Some((_, integration)) = persistence_options.get(selected) else {
                    continue;
                };
                polyglot_integrations.push(*integration);
            }
        }
    }

    let orm_pattern = if db_provider == "Turso" {
        "Turso Active Record"
    } else {
        V12_ORM_PATTERN
    }
    .to_string();
    let frontend_engine = V12_FRONTEND_ENGINE.to_string();

    let wants_ai = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("🤖 Will your project need Artificial Intelligence features (rullst-ai)?")
        .default(false)
        .interact()?;

    let wants_redis = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("Enable Redis adapters? (Requires explicit configuration; no automatic production fallback)")
        .default(false)
        .interact()?;

    Ok(ProjectWizardOptions {
        name,
        api,
        db_provider,
        db_needed,
        hot_reload,
        blueprint_selection,
        wants_ai,
        wants_redis,
        turso: polyglot_integrations.contains(&PolyglotIntegration::Turso),
        polyglot_integrations,
        orm_pattern,
        frontend_engine,
    })
}

#[cfg(test)]
#[path = "wizard_tests.rs"]
mod tests;
