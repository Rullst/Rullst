// src/cli.rs — Clap command definitions and the central dispatch function.
// This is the nerve center of the CLI: defines every subcommand and routes
// each one to its corresponding generator function.

use clap::{Parser, Subcommand};

use crate::generators::{
    auth::scaffold_auth_system,
    billing::scaffold_billing_system,
    build::{run_build_client, run_production_build, run_upgrade, run_dev_server},
    controller::create_new_controller,
    cors_jwt::{create_cors_middleware, create_jwt_middleware},
    db::run_project_db_command,
    desktop::{scaffold_desktop_system, scaffold_omni_system},
    foundry::{run_foundry_deploy, scaffold_foundry_config},
    middleware::create_new_middleware,
    migration::create_new_migration,
    model::create_new_model,
    openapi::generate_openapi_spec,
    project::create_new_project,
    worker::create_new_worker,
};

// ─── Clap Structs ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cargo-rullst")]
#[command(about = "CLI oficial do Rullst Framework", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Creates a new Rullst application
    New {
        /// Project name
        name: Option<String>,
        /// Optional: creates a headless REST API (no HTML)
        #[arg(long)]
        api: bool,
        /// Optional: generates Dockerfile, docker-compose.yml, and .dockerignore for production
        #[arg(long)]
        docker: bool,
    },
    /// Creates a new Controller in the src/controllers/ folder
    #[command(name = "make:controller")]
    MakeController {
        /// Name of the Controller (e.g. UsersController or users)
        name: String,
        /// Optional: generates JSON routes and responses (headless REST API) instead of HTML
        #[arg(long)]
        api: bool,
    },
    /// Creates a new Model in the src/models/ folder
    #[command(name = "make:model")]
    MakeModel {
        /// Name of the Model (e.g. BlogPost or blog_post)
        name: String,
        /// Optional: creates a corresponding database migration for the table
        #[arg(short, long)]
        migration: bool,
    },
    /// Creates a new Middleware in the src/middlewares/ folder
    #[command(name = "make:middleware")]
    MakeMiddleware {
        /// Name of the Middleware (e.g. Auth or auth_middleware)
        name: String,
    },
    /// Runs pending database migrations
    #[command(name = "db:migrate")]
    DbMigrate,
    /// Rolls back the last batch of applied migrations
    #[command(name = "db:rollback")]
    DbRollback,
    /// Displays the current status of project migrations
    #[command(name = "db:status")]
    DbStatus,
    /// Seeds the database using pre-configured seeders
    #[command(name = "db:seed")]
    DbSeed,
    /// Creates a new empty migration in the src/migrations/ folder
    #[command(name = "make:migration")]
    MakeMigration {
        /// Name of the migration (e.g. create_users_table)
        name: String,
    },
    /// Scaffolds authentication (login, registration, User model, migrations, middlewares, and HTML views)
    Auth,
    /// Scaffolds SaaS Billing (Stripe / LemonSqueezy database migrations, webhooks, checkout views)
    #[command(name = "make:billing")]
    MakeBilling,
    /// Adds Tauri desktop packaging to compile Hyper (HTMX + SSR) apps into native desktop applications
    #[command(name = "make:desktop")]
    MakeDesktop,
    /// Adds Dioxus multi-platform template integration pre-wired to Rullst backend API/WebSockets
    #[command(name = "make:omni")]
    MakeOmni,
    /// Initializes a Foundry.toml deployment manifest for 1-click cloud provisioning
    #[command(name = "foundry:init")]
    FoundryInit,
    /// Deploys the Rullst application to the cloud provider configured in Foundry.toml
    #[command(name = "foundry:deploy")]
    FoundryDeploy,
    /// Scaffolds and configures CORS middleware
    #[command(name = "make:cors")]
    MakeCors,
    /// Scaffolds and configures JWT authentication middleware
    #[command(name = "make:jwt")]
    MakeJwt,
    /// Scans controllers and generates an openapi.json/swagger specification
    #[command(name = "generate:openapi")]
    GenerateOpenapi,
    /// Creates a new background worker in the src/workers/ folder
    #[command(name = "make:worker")]
    MakeWorker {
        /// Name of the worker (e.g. Email or email_worker)
        name: String,
    },
    /// Executes a safe upgrade of the Rullst dependency using cargo fix codemods
    Upgrade,
    /// Starts the Rullst development server with neon spinners
    Dev,
    /// Opens the Rullst Studio dashboard to inspect the database
    #[command(name = "studio")]
    Studio,
    /// Compiles client-side components (Wasm Islands) to WebAssembly
    #[command(name = "build:client")]
    BuildClient {
        /// Optional: compile in debug mode (default is release)
        #[arg(long)]
        debug: bool,
    },
    /// Compiles the production binary and pre-compresses static assets (Brotli + Zstandard)
    Build {
        /// Optional: compile in debug mode instead of release
        #[arg(long)]
        debug: bool,
    },
    /// RullstPress: Native Static Site Generator (SSG) for websites and documentation
    Docs {
        #[command(subcommand)]
        action: DocsCommands,
    },
}

#[derive(Subcommand)]
pub enum DocsCommands {
    /// Starts the local live-preview server for RullstPress
    Dev,
    /// Compiles Markdown files into static HTML pages
    Build,
}

// ─── Dispatch ─────────────────────────────────────────────────────────────────

/// Central command dispatcher. Routes each CLI command to its generator function.
pub fn run_cli_command(command: &Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::New { name, api, docker } => {
            create_new_project(name.as_deref(), *api, *docker)?;
        }
        Commands::MakeController { name, api } => {
            create_new_controller(name, *api)?;
        }
        Commands::MakeModel { name, migration } => {
            create_new_model(name, *migration)?;
        }
        Commands::MakeMiddleware { name } => {
            create_new_middleware(name)?;
        }
        Commands::DbMigrate => {
            run_project_db_command("db:migrate")?;
        }
        Commands::DbRollback => {
            run_project_db_command("db:rollback")?;
        }
        Commands::DbStatus => {
            run_project_db_command("db:status")?;
        }
        Commands::DbSeed => {
            run_project_db_command("db:seed")?;
        }
        Commands::MakeMigration { name } => {
            create_new_migration(name)?;
        }
        Commands::Auth => {
            scaffold_auth_system()?;
        }
        Commands::MakeBilling => {
            scaffold_billing_system()?;
        }
        Commands::MakeDesktop => {
            scaffold_desktop_system()?;
        }
        Commands::MakeOmni => {
            scaffold_omni_system()?;
        }
        Commands::FoundryInit => {
            scaffold_foundry_config()?;
        }
        Commands::FoundryDeploy => {
            run_foundry_deploy()?;
        }
        Commands::MakeCors => {
            create_cors_middleware()?;
        }
        Commands::MakeJwt => {
            create_jwt_middleware()?;
        }
        Commands::GenerateOpenapi => {
            generate_openapi_spec()?;
        }
        Commands::MakeWorker { name } => {
            create_new_worker(name)?;
        }
        Commands::Upgrade => {
            run_upgrade()?;
        }
        Commands::Dev => {
            run_dev_server()?;
        }
        Commands::Studio => {
            run_project_db_command("studio")?;
        }
        Commands::BuildClient { debug } => {
            run_build_client(*debug)?;
        }
        Commands::Build { debug } => {
            run_production_build(!*debug)?;
        }
        Commands::Docs { action } => match action {
            DocsCommands::Dev => crate::docs_generator::run_dev_server()?,
            DocsCommands::Build => crate::docs_generator::run_build()?,
        },
    }
    Ok(())
}
