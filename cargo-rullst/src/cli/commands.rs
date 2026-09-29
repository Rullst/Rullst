// src/cli/commands.rs — The clap subcommand enum parsed by `Cli`.
#![cfg_attr(mutants, mutants::skip)]
// `Commands` is a published v12 enum that downstream code may match
// exhaustively; runtime-only subcommands are attached in `crate::run`.

use clap::Subcommand;
use std::path::PathBuf;

use super::{BlueprintChoice, DatabaseChoice, OmniPlatformChoice};

#[derive(Subcommand)]
pub enum Commands {
    /// Creates a new Rullst application
    New {
        /// Project name
        name: Option<String>,
        /// Optional: creates a headless REST API from the Blank starter (no HTML)
        #[arg(long)]
        api: bool,
        /// Optional: generates the current Dockerfile packaging scaffold
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
        /// Selects the primary relational backend in deterministic/CI mode
        #[arg(long, value_enum, requires = "default")]
        database: Option<DatabaseChoice>,
        /// Generates a blank project without a primary relational database
        #[arg(long, requires = "default", conflicts_with = "database")]
        no_database: bool,
        /// Legacy DLL profile; rejected in v12 because runtime globals cross an unstable ABI
        #[arg(long, requires = "default", hide = true)]
        hot_reload: bool,
        /// Enables the Rullst AI facade in deterministic/CI mode
        #[arg(long, requires = "default")]
        ai: bool,
        /// Enables Redis-backed adapters in deterministic/CI mode
        #[arg(long, requires = "default")]
        redis: bool,
        /// Skips the best-effort initial database migration after scaffolding
        #[arg(long)]
        skip_initial_migration: bool,
        /// Enables the Turso/libSQL edge SQL adapter and offline development fallback
        #[arg(long)]
        turso: bool,
        /// Enables the MongoDB document adapter
        #[arg(long)]
        mongodb: bool,
        /// Enables the DuckDB analytics adapter
        #[arg(long)]
        duckdb: bool,
        /// Enables the SurrealDB document and graph adapter
        #[arg(long)]
        surrealdb: bool,
        /// Enables the bounded Qdrant dense-vector adapter
        #[arg(long)]
        qdrant: bool,
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
    MakeOmni {
        /// Target platform; repeat the flag or use comma-separated values
        #[arg(long, value_enum, value_delimiter = ',')]
        platform: Vec<OmniPlatformChoice>,
        /// Backend URL embedded in the shell; required when a mobile platform is selected
        #[arg(long)]
        backend_url: Option<String>,
        /// Human-readable product name; defaults to the Cargo package name
        #[arg(long)]
        product_name: Option<String>,
        /// Application-owned reverse-DNS bundle/package identifier; required for mobile
        #[arg(long)]
        identifier: Option<String>,
        /// Application SemVer; defaults to the Cargo package version
        #[arg(long)]
        app_version: Option<String>,
    },
    /// Scaffolds a local IoT telemetry module
    #[command(name = "make:iot")]
    MakeIot {
        /// Name of the telemetry device type (e.g. TemperatureSensor)
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
    /// Scaffolds the bounded NFS-e/international receipt mailable
    #[command(name = "make:mail-invoice")]
    MakeMailInvoice {
        /// Name of the generated mailable struct
        #[arg(default_value = "FiscalInvoiceEmail")]
        name: String,
    },
    /// Scaffolds the explicit D+1/D+3/D+7 payment-recovery mailable
    #[command(name = "make:mail-dunning")]
    MakeMailDunning {
        /// Name of the generated mailable struct
        #[arg(default_value = "PaymentDunningEmail")]
        name: String,
    },
    /// Initializes a Foundry.toml manifest for a reviewed SSH deployment
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
    /// Generate a bounded project inventory while preserving project instructions
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
    /// Plans or applies a transactional Rullst project upgrade
    Upgrade {
        /// Exact target version; defaults to the installed cargo-rullst version
        #[arg(long, value_name = "VERSION")]
        to: Option<String>,
        /// Prints dependency changes and source findings without writing files
        #[arg(long)]
        dry_run: bool,
        /// Emits the dry-run plan as versioned JSON for automation
        #[arg(long, requires = "dry_run", conflicts_with = "restore")]
        json: bool,
        /// Leaves edits in place when a Cargo gate fails instead of restoring the backup
        #[arg(long, conflicts_with = "restore")]
        keep_on_failure: bool,
        /// Restores a backup previously created under target/rullst-upgrades
        #[arg(
            long,
            value_name = "BACKUP_DIR",
            conflicts_with_all = ["to", "dry_run", "json", "keep_on_failure"]
        )]
        restore: Option<PathBuf>,
    },
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
    /// Runs bounded security checks for secrets, CVEs, IDOR/BOLA routes, unsafe syntax, SBOM and network posture
    Audit {
        /// Optional: Print deterministic remediation suggestions (legacy --ai name)
        #[arg(long)]
        ai: bool,
        /// Optional: Export an evidence report without claiming compliance certification
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
        /// Explicit RustSec advisory exception already governed by the caller (repeatable)
        #[arg(long = "audit-ignore", value_name = "RUSTSEC-ID")]
        audit_ignore: Vec<String>,
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
    /// Evaluates the Academy production-boundary contract without claiming certification
    #[command(name = "academy:doctor")]
    AcademyDoctor {
        /// JSON evidence declarations; omitted requirements remain NOT_EVALUATED
        #[arg(long)]
        evidence: Option<PathBuf>,
        /// Emits the normalized diagnostic as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Writes an inspectable Axum/Tokio-oriented migration entry point
    Eject {
        /// Optional: Overwrite src/main.rs directly instead of creating src/ejected_main.rs
        #[arg(long)]
        force: bool,
        /// Optional: Custom output path for ejected file
        #[arg(long)]
        output: Option<String>,
    },
}
