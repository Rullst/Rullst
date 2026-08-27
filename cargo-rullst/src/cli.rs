// src/cli.rs — Clap command definitions and the central dispatch function.
#![cfg_attr(mutants, mutants::skip)]
// This is the nerve center of the CLI: defines every subcommand and routes
// each one to its corresponding generator function.

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use crate::generators::{
    auth::scaffold_auth_system,
    billing::scaffold_billing_system,
    build::{run_build_client, run_production_build, run_upgrade},
    controller::create_new_controller,
    cors_jwt::{create_cors_middleware, create_jwt_middleware},
    db::run_project_db_command,
    desktop::{run_omni_app, scaffold_omni_system},
    foundry::{run_foundry_deploy, scaffold_foundry_config},
    inspect::inspect_project,
    introspect::generate_models_from_db,
    mail::create_new_mailable,
    middleware::create_new_middleware,
    migration::create_new_migration,
    model::create_new_model,
    openapi::generate_openapi_spec,
    project::{ProjectScaffoldOptions, create_new_project_with_cli_options},
    resource::create_new_resource,
    worker::create_new_worker,
};

// ─── Clap Structs ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "rullst")]
#[command(version)]
#[command(about = "Official CLI for the Rullst Framework", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsCommand {
    /// Compiles docs/**/*.md into docs/dist/
    Build,
    /// Builds, watches and serves docs/dist/ on loopback
    Dev {
        /// Local preview port
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
}

/// Stable blueprint names accepted by non-interactive project generation.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlueprintChoice {
    Blank,
    Lms,
    Saas,
    Blog,
    Portfolio,
    Erp,
}

impl BlueprintChoice {
    pub const fn id(self) -> usize {
        match self {
            Self::Blank => crate::blueprints::BLANK_BLUEPRINT_ID,
            Self::Lms => crate::blueprints::LMS_BLUEPRINT_ID,
            Self::Saas => crate::blueprints::SAAS_BLUEPRINT_ID,
            Self::Blog => crate::blueprints::BLOG_BLUEPRINT_ID,
            Self::Portfolio => crate::blueprints::PORTFOLIO_BLUEPRINT_ID,
            Self::Erp => crate::blueprints::ERP_BLUEPRINT_ID,
        }
    }
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
        /// Optional: generates rootless OCI build.sh script via Buildah
        #[arg(long)]
        buildah: bool,
        /// Optional: generates Nix flake and direnv setup for reproducible environments
        #[arg(long)]
        nix: bool,
        /// Optional: skips interactive prompts and uses default values (useful for CI)
        #[arg(long)]
        default: bool,
        /// Selects a starter blueprint in deterministic/CI generation mode
        #[arg(long, value_enum, requires = "default")]
        blueprint: Option<BlueprintChoice>,
        /// Skips the best-effort initial database migration after scaffolding
        #[arg(long)]
        skip_initial_migration: bool,
        /// Optional: Scaffolds Turso/libSQL sidecar (sqld) for edge replication
        #[arg(long)]
        turso: bool,
    },
    /// Builds or previews the RullstPress documentation site
    Docs {
        #[command(subcommand)]
        command: DocsCommand,
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
    /// Creates a new Resource (Model, Migration, Controller, Views) in one command
    #[command(name = "make:resource")]
    MakeResource {
        /// Name of the Resource (e.g. Product or product)
        name: String,
        /// Optional: generates JSON API controller instead of HTML views
        #[arg(long)]
        api: bool,
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
    /// Automatically generates a migration by diffing Rust structs against the current database schema
    #[command(name = "make:migration:auto")]
    MakeMigrationAuto,
    /// Scaffolds authentication (login, registration, User model, migrations, middlewares, and HTML views)
    Auth,
    /// Scaffolds SaaS Billing (Stripe / LemonSqueezy database migrations, webhooks, checkout views)
    #[command(name = "make:billing")]
    MakeBilling {
        /// The primary Billable model (e.g. User, Team, Workspace)
        #[arg(long, default_value = "User")]
        model: String,
    },
    /// Scaffolds Tauri desktop & mobile packaging (Omni) for your application
    #[command(name = "make:omni")]
    MakeOmni,
    /// Scaffolds an IoT edge device module (Sensor Node, MQTT Gateway)
    #[command(name = "make:iot")]
    MakeIot {
        /// Name of the IoT edge device (e.g. TemperatureSensor, MqttGateway)
        name: String,
    },
    /// Scaffolds a strongly-typed Mailable email template in src/mail/
    #[command(name = "make:mail")]
    MakeMail {
        /// Name of the Mailable struct (e.g. WelcomeEmail, PasswordReset, InvoiceReceipt)
        name: String,
        /// Optional: generate a Welcome & Onboarding email template
        #[arg(long)]
        welcome: bool,
        /// Optional: generate a Password Reset email template
        #[arg(long)]
        reset: bool,
        /// Optional: generate a 2FA OTP Token email template
        #[arg(long)]
        otp: bool,
        /// Optional: generate a SaaS Invoice Receipt email template
        #[arg(long)]
        invoice: bool,
    },
    /// Initializes a Foundry.toml deployment manifest for 1-click cloud provisioning
    #[command(name = "foundry:init")]
    FoundryInit,
    /// Deploys the Rullst application to the cloud provider configured in Foundry.toml
    #[command(name = "foundry:deploy")]
    FoundryDeploy,
    /// Generates Dockerfile and docker-compose.yml for the project
    Dockerize,
    /// Generates a rootless OCI image build script via Buildah
    #[command(name = "generate:buildah")]
    GenerateBuildah,
    /// Generates Nix environment files (flake.nix, .envrc)
    Nixify,
    /// Scaffolds and configures CORS middleware
    #[command(name = "make:cors")]
    MakeCors,
    /// Scaffolds and configures JWT authentication middleware
    #[command(name = "make:jwt")]
    MakeJwt,
    /// Scans controllers and generates an openapi.json/swagger specification
    #[command(name = "generate:openapi")]
    GenerateOpenapi,
    /// Scans routes and generates a typed TypeScript client SDK
    #[command(name = "generate:ts")]
    GenerateTs,
    /// Auto-generates a Mermaid ER diagram from the Rust models
    #[command(name = "generate:diagram")]
    GenerateDiagram,
    /// Connects to an existing database and generates Rullst ORM models
    #[command(name = "generate:models", alias = "make:models-from-db")]
    GenerateModels {
        /// The database type (sqlite, postgres, mysql)
        #[arg(short, long)]
        driver: String,
        /// The connection string
        #[arg(short, long)]
        url: String,
        /// The output directory
        #[arg(short, long, default_value = "src/models")]
        output: String,
    },
    /// Generate an AI context file (.llms.txt) for Cursor, Claude, Gemini, etc.
    #[command(name = "generate:ai-context")]
    GenerateAiContext,
    /// Creates a new background worker in the src/workers/ folder
    #[command(name = "make:worker")]
    MakeWorker {
        /// Name of the worker (e.g. Email or email_worker)
        name: String,
    },
    /// Creates a new interactive frontend Wasm Island in src/islands/
    #[command(name = "make:island")]
    MakeIsland {
        /// Name of the Island component (e.g. Counter or user_profile)
        name: String,
    },
    /// Scaffolds ChatSession and ChatMessage models for Conversational AI memory
    #[command(name = "make:chat-session")]
    MakeChatSession,
    /// Scaffolds Kubernetes manifest files (Deployment, Service, ConfigMap, HPA, Ingress) in k8s/
    #[command(name = "make:k8s")]
    MakeK8s,
    /// Scaffolds a complete 2FA TOTP authentication system in src/controllers/mfa.rs
    #[command(name = "make:mfa")]
    MakeMfa,
    /// Scaffolds interactive Scalar API documentation router at /docs
    #[command(name = "make:scalar")]
    MakeScalar,
    /// Scaffolds a new LiveView-style reactive server component in src/live/
    #[command(name = "make:live")]
    MakeLive {
        /// Name of the LiveComponent (e.g. Counter or UserFeed)
        name: String,
    },
    /// Scaffolds a new gRPC service and Protobuf schema in proto/ and src/grpc/
    #[command(name = "make:grpc")]
    MakeGrpc {
        /// Name of the gRPC service (e.g. UserService or OrderService)
        name: String,
    },
    /// Deploys application to PaaS cloud providers (Fly.io, Railway, Render, VPS)
    Deploy {
        /// Target deployment platform (fly, railway, render, vps)
        #[arg(short, long)]
        platform: Option<String>,
    },
    /// Executes a safe upgrade of the Rullst dependency using cargo fix codemods
    Upgrade,
    /// Starts the Rullst development server with neon spinners
    Dev {
        /// Optional: Automatically sync TypeScript SDK (sdk.ts) on file changes
        #[arg(long = "ts-sync")]
        ts_sync: bool,
    },
    /// Manages community extensions and RullstPackage dependencies
    #[command(name = "pkg")]
    Pkg {
        /// Action to perform (add, list)
        action: String,
        /// Package name to add
        name: Option<String>,
    },
    /// Starts the interactive Ratatui Development Dashboard
    Dash,
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
    /// Starts the Omni App client (must be generated via make:omni first)
    Omni {
        /// Target platform (desktop, android, ios)
        target: Option<String>,
    },
    /// Expands and inspects macro code or structural definitions for debugging
    Inspect {
        /// Target item to inspect (e.g. routes, models, schema, or file path)
        target: Option<String>,
    },
    /// Runs AI-assisted security audit for secret leaks, CVEs, IDOR/BOLA routes, unsafe code, SBOM and network posture
    Audit {
        /// Optional: Enable AI Sentinel analysis suggestions
        #[arg(long)]
        ai: bool,
        /// Optional: Export SECURITY_COMPLIANCE.md report evaluating OWASP Top 10, SOC2, and ISO 27001
        #[arg(long)]
        compliance: bool,
        /// Optional: Run static IDOR / BOLA vulnerability scanner on parameterized routes
        #[arg(long)]
        idor: bool,
        /// Optional: Run Cargo Geiger dependency tree and AST unsafe memory safety analysis
        #[arg(long)]
        geiger: bool,
        /// Optional: Export CycloneDX 1.5 JSON Software Bill of Materials (sbom-cyclonedx.json)
        #[arg(long)]
        sbom: bool,
        /// Optional: Scan local network surface and interface bindings (inspired by RustScan)
        #[arg(long)]
        network: bool,
    },
    /// Installs automated Git pre-commit quality and security hook in .git/hooks/pre-commit
    #[command(name = "hook:install")]
    HookInstall,
    /// Runs full system diagnostics and toolchain health checks (Rust MSRV, Docker, linters, security tools)
    Doctor {
        /// Automatically attempt to install missing components or fix environment configurations
        #[arg(long)]
        fix: bool,
    },
    /// Ejects the framework abstractions into 100% pure Axum/Tokio Rust code
    Eject {
        /// Optional: Overwrite src/main.rs directly instead of creating src/ejected_main.rs
        #[arg(long)]
        force: bool,
        /// Optional: Custom output path for ejected file
        #[arg(long)]
        output: Option<String>,
    },
}

// ─── Dispatch ─────────────────────────────────────────────────────────────────

/// Central command dispatcher. Routes each CLI command to its generator function.
pub fn run_cli_command(command: &Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::New {
            name,
            api,
            docker,
            buildah,
            nix,
            default,
            blueprint,
            skip_initial_migration,
            turso,
        } => {
            create_new_project_with_cli_options(
                name.as_deref(),
                ProjectScaffoldOptions {
                    api: *api,
                    docker: *docker,
                    buildah: *buildah,
                    nix: *nix,
                    use_defaults: *default,
                    turso: *turso,
                },
                blueprint.as_ref().map(|choice| choice.id()),
                *skip_initial_migration,
            )?;
        }
        Commands::Docs { command } => match command {
            DocsCommand::Build => {
                let report = crate::generators::docs::build_docs_site(std::path::Path::new("."))?;
                println!(
                    "RullstPress built {} Markdown page(s) and {} asset(s) in {}",
                    report.markdown_pages,
                    report.copied_assets,
                    report.output_dir.display()
                );
            }
            DocsCommand::Dev { port } => {
                tokio::runtime::Runtime::new()?.block_on(crate::generators::docs::run_docs_dev(
                    std::path::Path::new("."),
                    *port,
                ))?;
            }
        },
        Commands::MakeController { name, api } => {
            create_new_controller(name, *api)?;
        }
        Commands::MakeModel { name, migration } => {
            create_new_model(name, *migration)?;
        }
        Commands::MakeResource { name, api } => {
            create_new_resource(name, *api)?;
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
        Commands::MakeMigrationAuto => {
            tokio::runtime::Runtime::new()?
                .block_on(crate::generators::migration::create_auto_migration())?;
        }
        Commands::Auth => {
            scaffold_auth_system()?;
        }
        Commands::MakeBilling { model } => {
            scaffold_billing_system(model)?;
        }
        Commands::MakeChatSession => {
            crate::generators::chat::scaffold_chat_session()?;
        }
        Commands::MakeOmni => {
            scaffold_omni_system()?;
        }
        Commands::MakeIot { name } => {
            crate::generators::iot::run_make_iot(name)?;
        }
        Commands::MakeMail {
            name,
            welcome,
            reset,
            otp,
            invoice,
        } => {
            create_new_mailable(name, *welcome, *reset, *otp, *invoice)?;
        }
        Commands::FoundryInit => {
            scaffold_foundry_config()?;
        }
        Commands::FoundryDeploy => {
            run_foundry_deploy()?;
        }
        Commands::Dockerize => {
            let mut proj_name = "app".to_string();
            if let Ok(toml_content) = std::fs::read_to_string("Cargo.toml") {
                for line in toml_content.lines() {
                    if line.starts_with("name = ") {
                        proj_name = line
                            .replace("name = ", "")
                            .replace("\"", "")
                            .trim()
                            .to_string();
                        break;
                    }
                }
            }
            crate::generators::project::generate_docker_files(
                std::path::Path::new("."),
                &proj_name,
                None,
                None,
            )?;
        }
        Commands::GenerateBuildah => {
            let mut proj_name = "app".to_string();
            if let Ok(toml_content) = std::fs::read_to_string("Cargo.toml") {
                for line in toml_content.lines() {
                    if line.starts_with("name = ") {
                        proj_name = line
                            .replace("name = ", "")
                            .replace("\"", "")
                            .trim()
                            .to_string();
                        break;
                    }
                }
            }
            crate::generators::project::generate_buildah_script(
                std::path::Path::new("."),
                &proj_name,
            )?;
        }
        Commands::Nixify => {
            crate::generators::project::generate_nix_files(std::path::Path::new("."))?;
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
        Commands::GenerateTs => {
            crate::generators::ts::generate_ts_sdk()?;
        }
        Commands::GenerateDiagram => {
            println!("Generating Schema Visualizer...");
            crate::generators::diagram::generate_mermaid_diagram(None)?;
            println!("Diagram generated successfully at diagram.md");
        }
        Commands::GenerateModels {
            driver,
            url,
            output,
        } => {
            generate_models_from_db(driver, url, output)?;
        }
        Commands::GenerateAiContext => {
            crate::generators::ai_context::generate_ai_context(None)?;
        }
        Commands::MakeWorker { name } => {
            create_new_worker(name)?;
        }
        Commands::MakeIsland { name } => {
            crate::generators::island::create_new_island(name)?;
        }
        Commands::Upgrade => {
            run_upgrade()?;
        }
        Commands::Dev { ts_sync } => {
            if *ts_sync {
                let _ = crate::generators::ts::generate_ts_sdk();
            }
            crate::generators::dev::run_dev_server(false)?;
        }
        Commands::Pkg { action, name } => match action.as_str() {
            "add" => {
                if let Some(pkg_name) = name {
                    crate::pkg::pkg_add(pkg_name)?;
                } else {
                    println!("{}", "❌ Please specify a package name (e.g. 'cargo rullst pkg add rullst-auth')".red());
                }
            }
            "list" => {
                crate::pkg::pkg_list()?;
            }
            _ => {
                println!(
                    "{}",
                    format!("❌ Unknown pkg action '{}'. Use 'add' or 'list'.", action).red()
                );
            }
        },
        Commands::Dash => {
            crate::generators::dev::run_dev_server(true)?;
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
        Commands::Omni { target } => {
            run_omni_app(target.as_deref())?;
        }
        Commands::Inspect { target } => {
            inspect_project(target.as_deref())?;
        }
        Commands::Audit {
            ai,
            compliance,
            idor,
            geiger,
            sbom,
            network,
        } => {
            crate::generators::audit::run_security_audit(
                *ai,
                *compliance,
                *idor,
                *geiger,
                *sbom,
                *network,
            )?;
        }
        Commands::HookInstall => {
            crate::generators::hook::install_git_pre_commit_hook()?;
        }
        Commands::Doctor { fix } => {
            crate::generators::doctor::run_doctor(*fix)?;
        }
        Commands::Eject { force, output } => {
            crate::generators::eject::run_eject_project(*force, output.as_deref())?;
        }
        Commands::MakeK8s => {
            crate::generators::k8s::generate_k8s_manifests()?;
        }
        Commands::MakeMfa => {
            crate::generators::auth::mfa::scaffold_mfa_system()?;
        }
        Commands::MakeScalar => {
            crate::generators::scalar::generate_scalar_docs()?;
        }
        Commands::MakeLive { name } => {
            crate::generators::live::create_new_live_component(name)?;
        }
        Commands::MakeGrpc { name } => {
            crate::generators::grpc::create_new_grpc_service(name)?;
        }
        Commands::Deploy { platform } => {
            crate::generators::deploy::run_deploy(platform.as_deref())?;
        }
    }

    // Automatically generate AI Context for scaffolding commands so it stays up to date
    match command {
        Commands::MakeController { .. }
        | Commands::MakeModel { .. }
        | Commands::MakeMiddleware { .. }
        | Commands::MakeWorker { .. }
        | Commands::MakeIsland { .. }
        | Commands::Auth
        | Commands::MakeBilling { .. }
        | Commands::MakeCors
        | Commands::MakeJwt => {
            crate::generators::ai_context::generate_ai_context(None).ok();
            crate::generators::diagram::generate_mermaid_diagram(None).ok();
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rullstpress_commands_from_the_sst() {
        let build = Cli::try_parse_from(["rullst", "docs", "build"]).expect("docs build CLI");
        assert!(matches!(
            build.command,
            Commands::Docs {
                command: DocsCommand::Build
            }
        ));

        let dev =
            Cli::try_parse_from(["rullst", "docs", "dev", "--port", "4321"]).expect("docs dev CLI");
        assert!(matches!(
            dev.command,
            Commands::Docs {
                command: DocsCommand::Dev { port: 4321 }
            }
        ));
    }

    #[test]
    fn parses_nix_and_buildah_flags_without_swapping_them() {
        let nix =
            Cli::try_parse_from(["rullst", "new", "demo", "--default", "--nix"]).expect("Nix CLI");
        assert!(matches!(
            nix.command,
            Commands::New {
                nix: true,
                buildah: false,
                ..
            }
        ));

        let buildah = Cli::try_parse_from(["rullst", "new", "demo", "--default", "--buildah"])
            .expect("Buildah CLI");
        assert!(matches!(
            buildah.command,
            Commands::New {
                nix: false,
                buildah: true,
                ..
            }
        ));
    }

    #[test]
    fn parses_deterministic_blueprint_generation_flags() {
        let cli = Cli::try_parse_from([
            "rullst",
            "new",
            "release-consumer",
            "--default",
            "--blueprint",
            "erp",
            "--skip-initial-migration",
        ])
        .expect("deterministic blueprint CLI");

        assert!(matches!(
            cli.command,
            Commands::New {
                blueprint: Some(BlueprintChoice::Erp),
                skip_initial_migration: true,
                ..
            }
        ));
    }

    #[test]
    fn explicit_blueprint_requires_non_interactive_defaults() {
        let error =
            Cli::try_parse_from(["rullst", "new", "release-consumer", "--blueprint", "saas"])
                .err()
                .expect("blueprint without deterministic defaults must be rejected");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }
}
