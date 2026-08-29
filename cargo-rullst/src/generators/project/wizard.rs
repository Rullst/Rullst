// cargo-rullst/src/generators/project/wizard.rs — Interactive project creation wizard.

use colored::*;

use crate::blueprints::{
    BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};

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
    pub orm_pattern: String,
    pub frontend_engine: String,
}

pub fn run_project_wizard(
    name_arg: Option<&str>,
    api: bool,
    use_defaults: bool,
    turso: bool,
) -> Result<ProjectWizardOptions, Box<dyn std::error::Error>> {
    run_project_wizard_with_blueprint(name_arg, api, use_defaults, turso, None)
}

pub(crate) fn run_project_wizard_with_blueprint(
    name_arg: Option<&str>,
    mut api: bool,
    use_defaults: bool,
    turso: bool,
    blueprint_override: Option<usize>,
) -> Result<ProjectWizardOptions, Box<dyn std::error::Error>> {
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

    if use_defaults {
        let name = name_arg.unwrap_or("app").to_string();
        let blueprint_selection = blueprint_override.unwrap_or(BLANK_BLUEPRINT_ID);
        return Ok(ProjectWizardOptions {
            name,
            api,
            db_provider: "sqlite".to_string(),
            db_needed: true,
            hot_reload: false,
            blueprint_selection,
            wants_ai: false,
            wants_redis: false,
            turso,
            orm_pattern: "Active Record".to_string(),
            frontend_engine: "Zero-Bundle HTMX".to_string(),
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
                "MySQL/MariaDB (Requires localhost:3306 running)",
                "Turso / libSQL (Edge Database)",
            ];
            let db_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("💾 Select a DB Provider (Network DBs will hang on setup if not running locally)")
                .default(0)
                .items(&db_options[..])
                .interact()?;
            db_provider = match db_selection {
                1 => "Postgres".to_string(),
                2 => "MySQL".to_string(),
                3 => "Turso".to_string(),
                _ => "Sqlite".to_string(),
            };
        }

        hot_reload = dialoguer::Confirm::with_theme(&theme)
            .with_prompt("🔥 Enable Hot Reloading by default? (Auto-recompiles on save)")
            .default(true)
            .interact()?;
    }

    let mut orm_pattern = "Active Record".to_string();
    let mut frontend_engine = "Zero-Bundle HTMX".to_string();

    if db_needed {
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
        turso,
        orm_pattern,
        frontend_engine,
    })
}
