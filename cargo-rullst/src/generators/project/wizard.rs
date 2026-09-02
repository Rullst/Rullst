// cargo-rullst/src/generators/project/wizard.rs — Interactive project creation wizard.

use colored::*;

use crate::blueprints::{
    BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use crate::generators::project::ProjectScaffoldOptions;

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

    if options.use_defaults {
        let name = name_arg.unwrap_or("app").to_string();
        let blueprint_selection = blueprint_override.unwrap_or(BLANK_BLUEPRINT_ID);
        let db_provider = db_provider_override.unwrap_or("Sqlite").to_string();
        if options.no_database && blueprint_selection != BLANK_BLUEPRINT_ID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--no-database is available only for the blank blueprint",
            )
            .into());
        }
        if options.no_database && options.orm_pattern.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "an ORM architecture cannot be selected without a primary database",
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
        if db_provider == "Turso" && options.orm_pattern.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Turso-primary selects its typed Active Record profile automatically",
            )
            .into());
        }
        return Ok(ProjectWizardOptions {
            name,
            api,
            orm_pattern: if db_provider == "Turso" {
                "Turso Active Record"
            } else {
                options.orm_pattern.unwrap_or("Active Record")
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
            frontend_engine: options
                .frontend_engine
                .unwrap_or("Zero-Bundle HTMX")
                .to_string(),
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
    let mut hot_reload = false;
    let mut blueprint_selection = blueprint_override.unwrap_or(BLANK_BLUEPRINT_ID);
    let mut polyglot_integrations = requested_integrations.to_vec();

    if name_arg.is_none() {
        let portfolio_title = format!(
            "Portfolio 🔥 (showcase for Rullst/AI developers) - {}",
            "HOT".bright_red().bold()
        );
        let blueprint_choices = vec![
            "Blank Starter (Minimal template with HTMX reactive counter)".to_string(),
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
            let db_options = &[
                "Sqlite (Zero setup)",
                "Postgres (Requires localhost:5432 running)",
                "MySQL (Requires localhost:3306 running)",
                "MariaDB (MySQL protocol; separately contract-tested)",
                "Turso / libSQL (primary edge SQL; explicit typed model API)",
            ];
            let db_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("💾 Select a DB Provider (Network DBs will hang on setup if not running locally)")
                .default(0)
                .items(&db_options[..])
                .interact()?;
            db_provider = match db_selection {
                1 => "Postgres".to_string(),
                2 => "MySQL".to_string(),
                3 => "MariaDB".to_string(),
                4 => "Turso".to_string(),
                _ => "Sqlite".to_string(),
            };
            if db_provider == "Turso"
                && !polyglot_integrations.contains(&PolyglotIntegration::Turso)
            {
                polyglot_integrations.push(PolyglotIntegration::Turso);
            }
        }

        let persistence_options = &[
            "Turso / libSQL (edge SQL; Hrana v3 + offline fallback)",
            "MongoDB (document CRUD)",
            "DuckDB (in-process OLAP / analytics)",
            "SurrealDB (document CRUD + bounded read-only graph queries)",
            "Qdrant (bounded dense-vector search)",
        ];
        let persistence_selection = dialoguer::MultiSelect::with_theme(&theme)
            .with_prompt("🧩 Select optional persistence capabilities (space toggles)")
            .items(&persistence_options[..])
            .interact()?;
        for selected in persistence_selection {
            let integration = match selected {
                0 => PolyglotIntegration::Turso,
                1 => PolyglotIntegration::MongoDb,
                2 => PolyglotIntegration::DuckDb,
                3 => PolyglotIntegration::SurrealDb,
                4 => PolyglotIntegration::Qdrant,
                _ => continue,
            };
            if !polyglot_integrations.contains(&integration) {
                polyglot_integrations.push(integration);
            }
        }

        if db_provider == "Turso" {
            hot_reload = false;
        } else {
            hot_reload = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("🔥 Enable Hot Reloading by default? (Auto-recompiles on save)")
                .default(true)
                .interact()?;
        }
    }

    let mut orm_pattern = "Active Record".to_string();
    let mut frontend_engine = "Zero-Bundle HTMX".to_string();

    if db_needed && db_provider != "Turso" {
        let orm_options = &[
            "Active Record Mode (Recommended — User::find(id), concise model-oriented CRUD)",
            "Data Mapper / Repository (For Enterprise DDD — UserRepository::find(), decoupled domain structs)",
            "Hybrid Architecture (Active Record for simple models + Repository Pattern for complex domain entities)",
        ];
        let orm_selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("🏗️ Select ORM Pattern / Architecture")
            .default(0)
            .items(&orm_options[..])
            .interact()?;
        orm_pattern = match orm_selection {
            1 => "Repository".to_string(),
            2 => "Hybrid".to_string(),
            _ => "Active Record".to_string(),
        };
    } else if db_provider == "Turso" {
        orm_pattern = "Turso Active Record".to_string();
    }

    if !api && blueprint_selection != BLANK_BLUEPRINT_ID {
        let fe_options = &[
            "HTMX + Tailwind SSR (Recommended — html! views; HTMX remains a browser dependency)",
            "LiveView compatibility profile (rullst::live primitives; wire routes and client transport)",
            "Wasm Island compatibility profile (rullst::island; build and load generated artifacts)",
            "Pico.css compatibility profile (semantic HTML; add and serve the stylesheet)",
            "Tera compatibility profile (adds Tera; application templates remain explicit)",
        ];
        let fe_selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("🎨 Select Frontend Engine")
            .default(0)
            .items(&fe_options[..])
            .interact()?;
        frontend_engine = match fe_selection {
            1 => "LiveView".to_string(),
            2 => "Wasm Island".to_string(),
            3 => "Pico CSS".to_string(),
            4 => "Tera Templates".to_string(),
            _ => "Zero-Bundle HTMX".to_string(),
        };
    }

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
mod tests {
    use super::*;

    #[test]
    fn deterministic_wizard_preserves_requested_persistence_features() {
        let selected = [
            PolyglotIntegration::Turso,
            PolyglotIntegration::MongoDb,
            PolyglotIntegration::DuckDb,
            PolyglotIntegration::SurrealDb,
            PolyglotIntegration::Qdrant,
        ];
        let options = run_project_wizard_with_blueprint(
            Some("polyglot-app"),
            ProjectScaffoldOptions {
                use_defaults: true,
                database: Some("MariaDB"),
                ..ProjectScaffoldOptions::default()
            },
            &selected,
            Some(BLANK_BLUEPRINT_ID),
        )
        .expect("deterministic wizard");

        assert_eq!(options.db_provider, "MariaDB");
        assert_eq!(options.polyglot_integrations, selected);
        assert!(options.turso);
    }

    #[test]
    fn deterministic_wizard_preserves_every_build_axis() {
        let options = run_project_wizard_with_blueprint(
            Some("profiled-app"),
            ProjectScaffoldOptions {
                use_defaults: true,
                database: Some("Postgres"),
                orm_pattern: Some("Hybrid"),
                frontend_engine: Some("Tera Templates"),
                hot_reload: true,
                wants_ai: true,
                wants_redis: true,
                ..ProjectScaffoldOptions::default()
            },
            &[],
            Some(ERP_BLUEPRINT_ID),
        )
        .expect("deterministic build axes");

        assert_eq!(options.db_provider, "Postgres");
        assert_eq!(options.orm_pattern, "Hybrid");
        assert_eq!(options.frontend_engine, "Tera Templates");
        assert!(options.hot_reload);
        assert!(options.wants_ai);
        assert!(options.wants_redis);
    }

    #[test]
    fn impossible_deterministic_profiles_fail_instead_of_being_ignored() {
        let no_database_lms = run_project_wizard_with_blueprint(
            Some("invalid-lms"),
            ProjectScaffoldOptions {
                use_defaults: true,
                no_database: true,
                ..ProjectScaffoldOptions::default()
            },
            &[],
            Some(LMS_BLUEPRINT_ID),
        );
        assert!(no_database_lms.is_err());

        let turso_hot_reload = run_project_wizard_with_blueprint(
            Some("invalid-edge"),
            ProjectScaffoldOptions {
                use_defaults: true,
                database: Some("Turso"),
                hot_reload: true,
                ..ProjectScaffoldOptions::default()
            },
            &[PolyglotIntegration::Turso],
            Some(BLANK_BLUEPRINT_ID),
        );
        assert!(turso_hot_reload.is_err());
    }
}
