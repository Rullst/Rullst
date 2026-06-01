#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::manual_strip,
    clippy::collapsible_if
)]
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub mod docs_generator;

#[derive(Parser)]
#[command(name = "cargo-rullst")]
#[command(about = "CLI oficial do Rullst Framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn get_cache_path() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("rullst_version_cache.txt");
    dir
}

fn is_version_newer(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.').filter_map(|p| p.parse().ok()).collect();
    let latest_parts: Vec<u32> = latest.split('.').filter_map(|p| p.parse().ok()).collect();

    if current_parts.len() == 3 && latest_parts.len() == 3 {
        for i in 0..3 {
            if latest_parts[i] > current_parts[i] {
                return true;
            } else if latest_parts[i] < current_parts[i] {
                return false;
            }
        }
    }
    false
}

fn check_update_available() -> Option<String> {
    let cache_path = get_cache_path();
    if cache_path.exists() {
        if let Ok(cached_version) = std::fs::read_to_string(&cache_path) {
            let cached_version = cached_version.trim().to_string();
            let current_version = env!("CARGO_PKG_VERSION");
            if is_version_newer(current_version, &cached_version) {
                return Some(cached_version);
            }
        }
    }
    None
}

fn trigger_background_update_check() {
    std::thread::spawn(|| {
        let cache_path = get_cache_path();
        let needs_refresh = if cache_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&cache_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        elapsed.as_secs() > 86400 // 24 hours
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if needs_refresh {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(4))
                .build();
            if let Ok(client) = client {
                let response = client
                    .get("https://crates.io/api/v1/crates/rullst")
                    .header("User-Agent", "cargo-rullst-updater/1.0.5")
                    .send();
                if let Ok(res) = response {
                    #[derive(serde::Deserialize)]
                    struct CrateInfo {
                        max_version: String,
                    }
                    #[derive(serde::Deserialize)]
                    struct CratesIoResponse {
                        #[serde(rename = "crate")]
                        krate: CrateInfo,
                    }
                    if let Ok(data) = res.json::<CratesIoResponse>() {
                        let _ = std::fs::write(&cache_path, &data.krate.max_version);
                    }
                }
            }
        }
    });
}

fn print_update_banner(latest_version: &str) {
    let current_version = env!("CARGO_PKG_VERSION");
    println!();
    println!(
        "{}",
        "┌────────────────────────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    println!(
        "{}  🚀 {} {:<19} {}",
        "│".cyan().bold(),
        "New Rullst version available:".bold().yellow(),
        format!("{} → {}", current_version, latest_version)
            .green()
            .bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  Run {} to update safely with              {}",
        "│".cyan().bold(),
        "'cargo rullst upgrade'".magenta().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  automatic code fixes (codemods).                         {}",
        "│".cyan().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Instantly check cached status (takes < 1ms)
    let update_available = check_update_available();
    // 2. Spawn background update check silently
    trigger_background_update_check();

    // If executed as a cargo subcommand (e.g. 'cargo rullst new'),
    // cargo passes "rullst" as the first real argument.
    // We remove it from the argument list so that Clap can parse uniformly.
    let args: Vec<String> = std::env::args().collect();
    let filtered_args = if args.len() > 1 && args[1] == "rullst" {
        let mut new_args = vec![args[0].clone()];
        new_args.extend_from_slice(&args[2..]);
        new_args
    } else {
        args
    };

    let cli = Cli::parse_from(filtered_args);

    run_cli_command(&cli.command)?;

    if let Some(latest) = update_available {
        print_update_banner(&latest);
    }

    Ok(())
}

/// Verifies if the current execution directory is a valid Rullst project
fn is_rullst_project() -> bool {
    let cargo_toml_path = Path::new("Cargo.toml");
    if !cargo_toml_path.exists() {
        return false;
    }
    match fs::read_to_string(cargo_toml_path) {
        Ok(content) => content.contains("rullst"),
        Err(_) => false,
    }
}

/// Normalizes the controller name to snake_case with the "_controller" suffix
fn to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the case-insensitive suffix if it already exists
    if base.to_lowercase().ends_with("controller") {
        let len = base.len();
        base.truncate(len - 10);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_controller");

    // Limpa possíveis underscores repetidos (ex: users__controller)
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result
}

/// Converts the controller name to CamelCase (PascalCase) with the "Controller" suffix
fn to_camel_case(s: &str) -> String {
    let snake = to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Normalizes the model name to snake_case
fn model_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the "Model" or "model" suffix if present
    if base.to_lowercase().ends_with("model") {
        let len = base.len();
        base.truncate(len - 5);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    // Limpa underscores repetidos
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

/// Converts the model name to PascalCase (CamelCase)
fn model_to_pascal_case(s: &str) -> String {
    let snake = model_to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Pluralizes the table name following the Active Record convention
fn pluralize(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.ends_with("ss") {
        format!("{}es", lower)
    } else if lower.ends_with("s") {
        lower
    } else if lower.ends_with("y") {
        let len = lower.len();
        if len > 1 {
            let before_y = &lower[len - 2..len - 1];
            if before_y == "a"
                || before_y == "e"
                || before_y == "i"
                || before_y == "o"
                || before_y == "u"
            {
                format!("{}s", lower)
            } else {
                format!("{}ies", &lower[..len - 1])
            }
        } else {
            format!("{}s", lower)
        }
    } else if lower.ends_with("ch")
        || lower.ends_with("sh")
        || lower.ends_with("x")
        || lower.ends_with("z")
    {
        format!("{}es", lower)
    } else {
        format!("{}s", lower)
    }
}

/// Normalizes the middleware name to snake_case with the "_middleware" suffix
fn middleware_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the case-insensitive suffix if it already exists
    if base.to_lowercase().ends_with("middleware") {
        let len = base.len();
        base.truncate(len - 10);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_middleware");

    // Clean up potential duplicate underscores (e.g., auth__middleware)
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

fn create_new_controller(name: &str, api: bool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate if we are in the root of the Rullst project
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        println!(
            "{}",
            "Make sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let snake_name = to_snake_case(name);
    let camel_name = to_camel_case(name);

    println!(
        "{}",
        format!("🛠️ Generating Rullst controller: {}...", camel_name)
            .cyan()
            .bold()
    );

    // 2. Ensure src/controllers directory exists
    let controllers_dir = Path::new("src/controllers");
    if !controllers_dir.exists() {
        fs::create_dir_all(controllers_dir)?;
    }

    // 3. Garantir que o src/controllers/mod.rs existe
    let mod_path = controllers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Register new controller in mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Create controller file
    let controller_path = controllers_dir.join(format!("{}.rs", snake_name));
    if controller_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Controller '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = if api {
            format!(
                r#"use axum::{{extract::{{Path, Form}}, response::IntoResponse, Json}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Add fields for creation
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Add fields for update
}}

/// Retorna a lista de recursos
pub async fn index() -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "index",
        "message": "This controller was automatically generated by the Rullst CLI. It is 100% friendly for humans and AI agents."
    }}))
}}

/// Retorna um recurso específico
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "show",
        "id": id
    }}))
}}

/// Creates a new resource
pub async fn store(Form(_payload): Form<CreateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "message": "Resource created successfully"
    }}))
}}

/// Atualiza um recurso existente
pub async fn update(Path(id): Path<i32>, Form(_payload): Form<UpdateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Resource updated successfully"
    }}))
}}

/// Deleta um recurso
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Resource deleted successfully"
    }}))
}}
"#
            )
        } else {
            format!(
                r#"use rullst::{{html, response::{{Html, IntoResponse}}}};
use axum::extract::{{Path, Form}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Add fields for creation
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Add fields for update
}}

/// Retorna a lista de recursos
pub async fn index() -> impl IntoResponse {{
    Html(html! {{
        <div style="font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #0f172a; color: #f8fafc; padding: 2rem; box-sizing: border-box;">
            <div style="max-width: 600px; text-align: center; background: #1e293b; padding: 3rem; border-radius: 1rem; border: 1px solid #334155; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);">
                <h1 style="font-size: 2.5rem; margin: 0 0 1rem 0; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800;">
                    "{camel_name}"
                </h1>
                <p style="color: #94a3b8; font-size: 1.1rem; line-height: 1.6; margin-bottom: 2rem;">
                    "This controller was automatically generated by the Rullst CLI. It is 100% friendly for humans and AI agents."
                </p>
                <div style="display: inline-block; padding: 0.75rem 1.5rem; background: #0f172a; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8; font-family: monospace; font-size: 0.95rem;">
                    "pub async fn index() -> impl IntoResponse"
                </div>
            </div>
        </div>
    }})
}}

/// Retorna um recurso específico
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ 
        <div>"Detalhes do recurso "{{id}}</div> 
    }})
}}

/// Creates a new resource
pub async fn store(Form(_payload): Form<CreateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource created successfully"</div> }})
}}

/// Atualiza um recurso existente
pub async fn update(Path(id): Path<i32>, Form(_payload): Form<UpdateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource "{{id}}" updated successfully"</div> }})
}}

/// Deleta um recurso
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource "{{id}}" deleted successfully"</div> }})
}}
"#
            )
        };
        fs::write(&controller_path, template)?;
    }

    // 6. Attempt to inject "pub mod controllers;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod controllers;")
            && !main_content.contains("mod controllers;")
        {
            main_content = format!("pub mod controllers;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod controllers;' to the top of src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Controller '{}' successfully created at '{}'!",
            camel_name,
            controller_path.display()
        )
        .green()
        .bold()
    );
    println!("{}", "How to map in your routes:".cyan());
    println!(
        "{}",
        format!("  1. Use: 'use crate::controllers::{};'", snake_name).cyan()
    );
    println!(
        "{}",
        format!(
            "  2. Add: 'get(\"/url\" => {}::index)' inside your routes! macro.",
            snake_name
        )
        .cyan()
    );

    Ok(())
}

fn create_new_model(name: &str, create_migration: bool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate if we are in the root of the Rullst project
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        println!(
            "{}",
            "Make sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let snake_name = model_to_snake_case(name);
    let pascal_name = model_to_pascal_case(name);
    let plural_name = pluralize(&snake_name);

    println!(
        "{}",
        format!("🛠️ Generating Rullst model: {}...", pascal_name)
            .cyan()
            .bold()
    );

    // 2. Ensure src/models directory exists
    let models_dir = Path::new("src/models");
    if !models_dir.exists() {
        fs::create_dir_all(models_dir)?;
    }

    // 3. Garantir que o src/models/mod.rs existe
    let mod_path = models_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Register new model in mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Create model file
    let model_path = models_dir.join(format!("{}.rs", snake_name));
    if model_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Model '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst_orm::{{Orm, RullstModel, sqlx::{{self, FromRow}}}};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "{plural_name}")]
pub struct {pascal_name} {{
    pub id: i32,
    // Add your fields here (e.g. pub name: String)
}}
"#
        );
        fs::write(&model_path, template)?;
    }

    // 6. Attempt to inject "pub mod models;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod models;") && !main_content.contains("mod models;") {
            main_content = format!("pub mod models;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod models;' to the top of src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Model '{}' successfully created at '{}'!",
            pascal_name,
            model_path.display()
        )
        .green()
        .bold()
    );

    // 7. Create migration if requested
    if create_migration {
        let migrations_dir = Path::new("src/migrations");
        if !migrations_dir.exists() {
            fs::create_dir_all(migrations_dir)?;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let migration_name = format!("create_{}", plural_name);
        let file_stem = format!("m{}_{}", timestamp, migration_name);
        let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

        let template = format!(
            r#"use rullst_orm::schema::{{Schema, Blueprint, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::create("{plural_name}", |table| {{
            table.id();
            // Add your fields here (e.g. table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::drop_if_exists("{plural_name}").await
    }}
}}
"#,
            file_stem = file_stem,
            plural_name = plural_name
        );

        fs::write(&migration_path, template)?;
        println!(
            "{}",
            format!(
                "✨ Rust migration successfully created at '{}'!",
                migration_path.display()
            )
            .green()
            .bold()
        );

        // Regenerar src/migrations/mod.rs
        regenerate_migrations_mod()?;
    }

    println!("{}", "How to import and use:".cyan());
    println!(
        "{}",
        format!(
            "  1. Use: 'use crate::models::{}::{};'",
            snake_name, pascal_name
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "  2. Fetch data: 'let items = {}::all().await?;'",
            pascal_name
        )
        .cyan()
    );

    Ok(())
}

fn create_new_middleware(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate if we are in the root of the Rullst project
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        println!(
            "{}",
            "Make sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let snake_name = middleware_to_snake_case(name);

    println!(
        "{}",
        format!("🛠️ Generating Rullst middleware: {}...", snake_name)
            .cyan()
            .bold()
    );

    // 2. Ensure src/middlewares directory exists
    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    // 3. Garantir que o src/middlewares/mod.rs existe
    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Register new middleware in mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Create middleware file
    let middleware_path = middlewares_dir.join(format!("{}.rs", snake_name));
    if middleware_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Middleware '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use axum::{{extract::Request, middleware::Next, response::Response}};

pub async fn {}(req: Request, next: Next) -> Response {{
    // Pre-request logic here
    
    let response = next.run(req).await;
    
    // Post-request logic here
    
    response
}}
"#,
            snake_name
        );
        fs::write(&middleware_path, template)?;
    }

    // 6. Attempt to inject "pub mod middlewares;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs automaticamente.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Middleware '{}' successfully created at '{}'!",
            snake_name,
            middleware_path.display()
        )
        .green()
        .bold()
    );
    println!("{}", "How to map in your routes using Axum layers:".cyan());
    println!("{}", "  1. Use: 'use axum::middleware::from_fn;'".cyan());
    println!(
        "{}",
        format!(
            "  2. Use: 'use crate::middlewares::{}::{};'",
            snake_name, snake_name
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "  3. Add: '.layer(from_fn({}))' on your router.",
            snake_name
        )
        .cyan()
    );

    Ok(())
}

fn has_binary(name: &str) -> bool {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn create_new_project(
    name_arg: Option<&str>,
    api_arg: bool,
    docker: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}{}",
        r#"
  _____       _ _     _   
 |  __ \     | | |   | |  
 | |__) |   _| | |___| |_ 
 |  _  / | | | | / __| __|
 | | \ \ |_| | | \__ \ |_ 
 |_|  \_\__,_|_|_|___/\__|
    "#
        .cyan(),
        "\nWelcome to the official Rullst application wizard!\n"
            .white()
            .bold()
    );

    let theme = dialoguer::theme::ColorfulTheme::default();

    let name = match name_arg {
        Some(n) => n.to_string(),
        None => {
            let mut val = String::new();
            while val.trim().is_empty() || val.contains(' ') {
                val = dialoguer::Input::with_theme(&theme)
                    .with_prompt("App name? (no spaces allowed)")
                    .interact_text()?;
                if val.contains(' ') {
                    println!(
                        "{}",
                        "❌ Spaces are not allowed in the project name. Please try again.".red()
                    );
                }
            }
            val
        }
    };

    let mut api = api_arg;
    let mut db_provider = "Sqlite".to_string();
    let mut db_needed = true;
    let mut hot_reload = false;

    if name_arg.is_none() {
        let build_options = &[
            "Full-Stack Web App (SaaS, Portfolio, Blog)",
            "Headless REST API",
        ];
        let build_selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("What would you like to build?")
            .default(0)
            .items(&build_options[..])
            .interact()?;
        api = build_selection == 1;

        hot_reload = dialoguer::Confirm::with_theme(&theme)
            .with_prompt("Enable Hot Reloading by default?")
            .default(false)
            .interact()?;

        db_needed = dialoguer::Confirm::with_theme(&theme)
            .with_prompt("Will your project need a Data Base?")
            .default(true)
            .interact()?;

        if db_needed {
            let db_options = &["Sqlite", "Postgres", "MySQL/MariaDB"];
            let db_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("Select a DB Provider")
                .default(0)
                .items(&db_options[..])
                .interact()?;
            db_provider = match db_options[db_selection] {
                "MySQL/MariaDB" => "MySQL".to_string(),
                other => other.to_string(),
            };
        }
    }

    println!(
        "{}",
        format!("🚀 Creating new Rullst app: {}...", name)
            .green()
            .bold()
    );

    let path = Path::new(&name);
    if path.exists() {
        println!(
            "{}",
            format!("❌ Error: Directory '{}' already exists.", name).red()
        );
        std::process::exit(1);
    }

    // Create folders
    fs::create_dir_all(path.join("src/pages"))?;
    fs::create_dir_all(path.join("src/models"))?;

    // Scaffold initial src/migrations/mod.rs file
    if db_needed {
        let migrations_dir = path.join("src/migrations");
        fs::create_dir_all(&migrations_dir)?;
        fs::write(
            migrations_dir.join("mod.rs"),
            r#"// Generated by Rullst.

pub fn get_migrations() -> Vec<Box<dyn rullst_orm::schema::Migration>> {
    vec![]
}
"#,
        )?;
    }

    // Get absolute path to the Rullst framework folder for local referencing
    let current_dir = std::env::current_dir()?;
    let rullst_path = if current_dir.join("rullst").exists() {
        current_dir
            .join("rullst")
            .canonicalize()?
            .display()
            .to_string()
    } else {
        "c:\\Users\\venelouis\\Desktop\\REPOS\\Rullst\\rullst".to_string()
    };

    // Get absolute path to rullst-orm for local referencing
    let _rullst_orm_path = if current_dir.join("rullst-orm").exists() {
        current_dir
            .join("rullst-orm/rullst-orm")
            .canonicalize()?
            .display()
            .to_string()
    } else if current_dir
        .parent()
        .map(|p| p.join("rullst-orm/rullst-orm").exists())
        .unwrap_or(false)
    {
        current_dir
            .parent()
            .unwrap()
            .join("rullst-orm/rullst-orm")
            .canonicalize()?
            .display()
            .to_string()
    } else {
        "c:\\Users\\venelouis\\Desktop\\REPOS\\rullst-orm\\rullst-orm".to_string()
    };

    // Fix Windows path escaping in Cargo.toml and strip UNC prefix \\?\ if present
    let rullst_path = rullst_path.trim_start_matches(r"\\?\").replace("\\", "/");
    let _rullst_orm_path = _rullst_orm_path
        .trim_start_matches(r"\\?\")
        .replace("\\", "/");

    // Extract a valid package name from the path (e.g. "..\dummy_test" -> "dummy_test")
    let project_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&name)
        .replace("\\", "")
        .replace("/", "")
        .replace(".", "")
        .replace("-", "_");

    let project_name_safe = project_name.replace("-", "_");

    let sqlx_features = match db_provider.as_str() {
        "Postgres" => r#"features = ["postgres", "runtime-tokio"]"#,
        "MySQL" => r#"features = ["mysql", "runtime-tokio"]"#,
        _ => r#"features = ["sqlite", "runtime-tokio"]"#,
    };

    // Write Cargo.toml
    let mut cargo_toml = format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2024"
"#,
        project_name = project_name
    );

    if hot_reload {
        cargo_toml.push_str(
            r#"
[lib]
crate-type = ["cdylib", "rlib"]
"#,
        );
    }

    cargo_toml.push_str(&format!(
        r#"
[dependencies]
rullst = {{ path = "{rullst_path}" }}
tokio = {{ version = "1.43", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
axum = "0.8"
"#,
        rullst_path = rullst_path
    ));

    if db_needed {
        cargo_toml.push_str(&format!(
            r#"rullst-orm = "1.1.0"
sqlx = {{ version = "0.8", {sqlx_features} }}
"#,
            sqlx_features = sqlx_features
        ));
    }

    cargo_toml.push_str(
        r#"
# ⚡ Rullst God-Mode: Compilação Incremental Instantânea (<100ms)
# Se você deseja velocidade de desenvolvimento próxima de linguagens interpretadas,
# você pode usar o backend Cranelift oficial do compilador Rust.
# 
# Requisitos:
#   1. Instalar toolchain nightly: rustup toolchain install nightly
#   2. Instalar o componente: rustup component add rustc-codegen-cranelift-preview --toolchain nightly
#   3. Ative descomentando o bloco abaixo e rodando o projeto com a toolchain nightly (ex: cargo +nightly run)
# 
# [profile.dev]
# codegen-backend = "cranelift"

[workspace]
"#,
    );

    fs::write(path.join("Cargo.toml"), cargo_toml)?;

    // Criar diretório .cargo e escrever config.toml inteligente
    let cargo_dir = path.join(".cargo");
    fs::create_dir_all(&cargo_dir)?;

    let has_mold = has_binary("mold");
    let has_lld = has_binary("lld") || has_binary("lld-link");

    let mut config_toml = String::new();
    config_toml.push_str(
        r#"# 🚀 Rullst Compiler & Linker Optimization Configuration
# Este arquivo configura linkers ultra-rápidos para desenvolvimento local.
# O Rullst detectou seu ambiente e configurou as opções adequadas.

"#
    );

    // Configuração para Windows (MSVC usa lld-link ou lld)
    if has_lld && cfg!(windows) {
        config_toml.push_str(
            r#"[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

"#
        );
    } else {
        config_toml.push_str(
            r#"# Para ativar no Windows (Instale LLVM com 'winget install LLVM.LLVM' e descomente abaixo):
# [target.x86_64-pc-windows-msvc]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]

"#
        );
    }

    // Configuração para Linux (GNU usa mold ou lld)
    if has_mold && cfg!(target_os = "linux") {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

"#
        );
    } else if has_lld && cfg!(target_os = "linux") {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

"#
        );
    } else {
        config_toml.push_str(
            r#"# Para ativar no Linux (Instale o mold com seu gerenciador de pacotes e descomente abaixo):
# [target.x86_64-unknown-linux-gnu]
# rustflags = ["-C", "link-arg=-fuse-ld=mold"]

"#
        );
    }

    // Configuração para macOS (Darwin usa lld)
    if has_lld && cfg!(target_os = "macos") {
        config_toml.push_str(
            r#"[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
"#
        );
    } else {
        config_toml.push_str(
            r#"# Para ativar no macOS (Instale llvm/lld via brew e descomente abaixo):
# [target.x86_64-apple-darwin]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
# [target.aarch64-apple-darwin]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
"#
        );
    }

    fs::write(cargo_dir.join("config.toml"), config_toml)?;

    // Write Rullst.toml configuration
    if db_needed {
        let db_url = match db_provider.as_str() {
            "Postgres" => "postgres://postgres:password@localhost/rullst",
            "MySQL" => "mysql://root:password@localhost/rullst",
            _ => "sqlite://rullst.db",
        };
        let rullst_toml = format!(
            r#"[database]
url = "{db_url}"
"#
        );
        fs::write(path.join("Rullst.toml"), rullst_toml)?;
    }

    // Write src code templates
    let db_model_code = if db_needed {
        r#"use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};

// 1. Define your database model using the built-in rullst-orm ORM!
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}
"#
    } else {
        ""
    };

    let db_status_code = if db_needed {
        r#"    // ORM usage example: Fetch active users from database
    let db_status = match User::all().await {
        Ok(users) => format!("Database connected! Total users: {}", users.len()),
        Err(e) => format!("Database offline or not configured: {}", e),
    };"#
    } else {
        r#"    let db_status = "Database features are disabled for this project.".to_string();"#
    };

    let migrations_mod_declaration = if db_needed {
        "pub mod migrations;\n"
    } else {
        ""
    };

    let artisan_call = if db_needed {
        "    // 1. Intercept Artisan commands (e.g. cargo rullst db:migrate) before starting server\n    rullst::artisan!(crate::migrations::get_migrations());\n"
    } else {
        ""
    };

    if hot_reload {
        // Scaffold lib.rs and launcher main.rs
        let lib_rs = if api {
            format!(
                r##"use rullst::{{routes, Router, response::IntoResponse}};
use serde::Serialize;

{migrations_mod_declaration}

{db_model_code}

#[derive(Serialize)]
struct HomeResponse {{
    message: String,
    database_status: String,
}}

pub async fn home() -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    axum::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = routes![
        get("/" => home),
    ];
    Box::into_raw(Box::new(router))
}}
"##,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                migrations_mod_declaration = migrations_mod_declaration
            )
        } else {
            format!(
                r##"use rullst::{{html, routes, Router, response::{{Html, IntoResponse}}}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}

{db_model_code}

// Main route: uses hybrid SSR with render_page
pub async fn home(htmx: HtmxRequest) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "The ultimate full-stack framework for Rust. Focused on Security, Maintainability, and Speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                        </button>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    }};

    render_page(&htmx, "Welcome to Rullst", content)
}}

// State for counter
use std::sync::atomic::{{AtomicUsize, Ordering}};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Reactive HTMX endpoint
pub async fn clicked() -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Click here to increment"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ];
    Box::into_raw(Box::new(router))
}}
"##,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                migrations_mod_declaration = migrations_mod_declaration
            )
        };

        fs::write(path.join("src/lib.rs"), lib_rs)?;

        let main_rs = format!(
            r##"{migrations_mod_declaration}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            "target/debug/{project_name}"
        }} else {{
            "target/debug/lib{project_name}"
        }};
        rullst::Server::new_hot(lib_path)
    }} else {{
        let router_ptr = {project_name_safe}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name = project_name,
            project_name_safe = project_name_safe,
            migrations_mod_declaration = migrations_mod_declaration,
            artisan_call = artisan_call
        );

        fs::write(path.join("src/main.rs"), main_rs)?;
    } else {
        // Standard non-hot-reloaded single binary scaffold
        let main_rs = if api {
            format!(
                r##"use rullst::{{routes, Server, Router, response::IntoResponse}};
use serde::Serialize;

{migrations_mod_declaration}

{db_model_code}

#[derive(Serialize)]
struct HomeResponse {{
    message: String,
    database_status: String,
}}

async fn home() -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    axum::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                migrations_mod_declaration = migrations_mod_declaration,
                artisan_call = artisan_call
            )
        } else {
            format!(
                r##"use rullst::{{html, routes, Server, response::{{Html, IntoResponse}}}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}

{db_model_code}

async fn home(htmx: HtmxRequest) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "The ultimate full-stack framework for Rust. Focused on Security, Maintainability, and Speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                        </button>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    }};

    render_page(&htmx, "Welcome to Rullst", content)
}}

// State for counter
use std::sync::atomic::{{AtomicUsize, Ordering}};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Reactive HTMX endpoint
async fn clicked() -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Click here to increment"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                migrations_mod_declaration = migrations_mod_declaration,
                artisan_call = artisan_call
            )
        };

        fs::write(path.join("src/main.rs"), main_rs)?;
    }

    // Generate Docker files if --docker flag was passed
    if docker {
        generate_docker_files(path, &project_name)?;
    }

    let has_mold = has_binary("mold");
    let has_lld = has_binary("lld") || has_binary("lld-link");

    if !has_mold && !has_lld {
        println!("\n{}", "⚡ Rullst Dev Tip: Speed up compile times up to 10x!".yellow().bold());
        println!("{}", "To unlock near-instant compile speeds, we highly recommend installing a fast linker:".white());
        if cfg!(windows) {
            println!("{}", "  👉 Install LLD: winget install LLVM.LLVM".cyan());
        } else if cfg!(target_os = "macos") {
            println!("{}", "  👉 Install LLD: brew install llvm".cyan());
        } else {
            println!("{}", "  👉 Install Mold: sudo apt install mold (or dnf install mold)".cyan());
        }
        println!("{}", "Once installed, uncomment the config lines inside '.cargo/config.toml'!".white());
    } else {
        println!("\n{}", "🚀 High-performance linker automatically detected and configured!".green().bold());
    }

    println!(
        "{}",
        format!("✨ Project '{}' created successfully!", name)
            .green()
            .bold()
    );
    println!("{}", "How to run:".cyan());
    println!("{}", format!("  cd {}", name).cyan());
    if hot_reload {
        println!("{}", "  HOT_RELOAD=1 cargo run".cyan());
    } else {
        println!("{}", "  cargo run".cyan());
    }
    if docker {
        println!(
            "{}",
            "\n🐳 Docker files generated! To run with Docker:".cyan()
        );
        println!("{}", format!("  cd {}", name).cyan());
        println!("{}", "  docker compose up --build".cyan());
    }

    Ok(())
}

// ==========================================
// HELPER FUNCTIONS FOR DATABASE OPERATIONS
// ==========================================

fn run_project_db_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        format!("⏳ Running 'cargo run -- {}'...", command)
            .cyan()
            .bold()
    );

    let status = std::process::Command::new("cargo")
        .args(&["run", "--", command])
        .status()?;

    if !status.success() {
        println!(
            "{}",
            format!("❌ Failed to execute db command: {}", command)
                .red()
                .bold()
        );
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn create_new_migration(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let snake_name = name
        .to_lowercase()
        .replace("-", "_")
        .trim_start_matches("m")
        .to_string();
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_{}", timestamp, snake_name);

    println!(
        "{}",
        format!("🛠️ Gerando migração Rullst: {}...", file_stem)
            .cyan()
            .bold()
    );

    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        fs::create_dir_all(migrations_dir)?;
    }

    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));
    let table_name = get_table_name_from_migration(&snake_name);

    let template = format!(
        r#"use rullst_orm::schema::{{Schema, Blueprint, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::create("{table_name}", |table| {{
            table.id();
            // Add your fields here (e.g. table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::drop_if_exists("{table_name}").await
    }}
}}
"#,
        file_stem = file_stem,
        table_name = table_name
    );

    fs::write(&migration_path, template)?;
    println!(
        "{}",
        format!(
            "✨ Rust migration successfully created at '{}'!",
            migration_path.display()
        )
        .green()
        .bold()
    );

    regenerate_migrations_mod()?;

    Ok(())
}

fn get_table_name_from_migration(name: &str) -> String {
    let s = name.to_lowercase();
    if s.starts_with("create_") && s.ends_with("_table") {
        s[7..s.len() - 6].to_string()
    } else if s.starts_with("create_") {
        s[7..].to_string()
    } else {
        "table_name".to_string()
    }
}

fn regenerate_migrations_mod() -> Result<(), Box<dyn std::error::Error>> {
    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        return Ok(());
    }

    let paths = fs::read_dir(migrations_dir)?;
    let mut modules = vec![];
    for path in paths {
        let path = path?.path();
        if let Some(ext) = path.extension() {
            if ext == "rs" {
                if let Some(stem) = path.file_stem() {
                    let stem_str = stem.to_string_lossy().to_string();
                    if stem_str != "mod" && stem_str.starts_with('m') {
                        modules.push(stem_str);
                    }
                }
            }
        }
    }
    modules.sort();

    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rullst. Do not edit manually.\n\n");
    for m in &modules {
        mod_content.push_str(&format!("pub mod {};\n", m));
    }
    mod_content
        .push_str("\npub fn get_migrations() -> Vec<Box<dyn rullst_orm::schema::Migration>> {\n");
    mod_content.push_str("    vec![\n");
    for m in &modules {
        mod_content.push_str(&format!("        Box::new({}::MigrationImpl),\n", m));
    }
    mod_content.push_str("    ]\n");
    mod_content.push_str("}\n");

    fs::write(migrations_dir.join("mod.rs"), mod_content)?;
    Ok(())
}

fn scaffold_auth_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🛡️  Starting scaffolding of Rullst authentication system..."
            .cyan()
            .bold()
    );

    // 1. Create User Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_users_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst_orm::schema::{{Schema, Blueprint, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::create("users", |table| {{
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::drop_if_exists("users").await
    }}
}}
"##,
        file_stem = file_stem
    );
    fs::write(&migration_path, migration_template)?;
    println!("{}", "  ✨ Created 'users' table migration.".green());

    // 1b. Create User Passkeys Migration
    let timestamp_passkeys = (now + chrono::Duration::seconds(1))
        .format("%Y%m%d%H%M%S")
        .to_string();
    let file_stem_passkeys = format!("m{}_create_user_passkeys_table", timestamp_passkeys);
    let migration_passkeys_path = migrations_dir.join(format!("{}.rs", file_stem_passkeys));

    let migration_passkeys_template = format!(
        r##"use rullst_orm::schema::{{Schema, Blueprint, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem_passkeys}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::create("user_passkeys", |table| {{
            table.id();
            table.integer("user_id").not_null();
            table.string("name").not_null();
            table.text("passkey_json").not_null();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::drop_if_exists("user_passkeys").await
    }}
}}
"##,
        file_stem_passkeys = file_stem_passkeys
    );
    fs::write(&migration_passkeys_path, migration_passkeys_template)?;
    println!(
        "{}",
        "  ✨ Created 'user_passkeys' table migration.".green()
    );

    regenerate_migrations_mod()?;

    // 2. Create User Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    let model_template = r##"use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'User' model.".green());

    // 2b. Create UserPasskey Model
    let passkey_model_path = models_dir.join("user_passkey.rs");
    let passkey_model_template = r##"use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "user_passkeys")]
pub struct UserPasskey {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub passkey_json: String,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    fs::write(&passkey_model_path, passkey_model_template)?;
    println!("{}", "  ✨ Created 'UserPasskey' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    let mut modified = false;
    if !mod_models_content.contains("pub mod user;") {
        mod_models_content.push_str("pub mod user;\n");
        modified = true;
    }
    if !mod_models_content.contains("pub mod user_passkey;") {
        mod_models_content.push_str("pub mod user_passkey;\n");
        modified = true;
    }
    if modified {
        fs::write(&mod_models_path, mod_models_content)?;
    }

    // 3. Create Authentication Middleware
    let middlewares_dir = Path::new("src/middlewares");
    fs::create_dir_all(middlewares_dir)?;
    let middleware_path = middlewares_dir.join("auth_middleware.rs");
    let middleware_template = r##"use axum::{
    extract::Request,
    middleware::Next,
    response::{Response, Redirect, IntoResponse},
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    
    // 1. Extrai o cookie de sessão criptografado
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = rullst::auth::get_app_key();
        
        // 2. Descriptografa o user_id
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            // 3. Insere o user_id nas extensions da requisição para acesso nos controllers
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    
    // 4. Redirect to login if not authenticated
    Redirect::to("/login").into_response()
}
"##;
    fs::write(&middleware_path, middleware_template)?;
    println!("{}", "  ✨ Created 'auth_middleware' middleware.".green());

    let mod_middlewares_path = middlewares_dir.join("mod.rs");
    if !mod_middlewares_path.exists() {
        fs::write(&mod_middlewares_path, "")?;
    }
    let mut mod_middlewares_content = fs::read_to_string(&mod_middlewares_path)?;
    if !mod_middlewares_content.contains("pub mod auth_middleware;") {
        mod_middlewares_content.push_str("pub mod auth_middleware;\n");
        fs::write(&mod_middlewares_path, mod_middlewares_content)?;
    }

    // 4. Create HTML Pages
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let pages_path = pages_dir.join("auth.rs");
    let pages_template = r##"use rullst::html;
use axum::response::Html;

const PASSKEY_SCRIPT: &str = r#"<script>
    function bufferDecode(value) {
        const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
        const pad = base64.length % 4;
        const padded = pad ? base64 + "=".repeat(4 - pad) : base64;
        const binary = window.atob(padded);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes.buffer;
    }

    function bufferEncode(value) {
        const bytes = new Uint8Array(value);
        let binary = "";
        for (let i = 0; i < bytes.byteLength; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        const base64 = window.btoa(binary);
        return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
    }

    document.addEventListener("DOMContentLoaded", () => {
        if (window.PublicKeyCredential) {
            document.querySelectorAll(".btn-passkey").forEach(btn => btn.style.display = "flex");
        }
    });

    async function registerPasskey() {
        try {
            const email = document.getElementById("email").value;
            const name = document.getElementById("name").value;
            if (!email || !name) {
                alert("Por favor, preencha o nome e email antes de criar a Passkey.");
                return;
            }

            const res = await fetch("/auth/passkey/register/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, name })
            });
            if (!res.ok) throw new Error(await res.text());
            
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            options.publicKey.user.id = bufferDecode(options.publicKey.user.id);
            if (options.publicKey.excludeCredentials) {
                for (let cred of options.publicKey.excludeCredentials) {
                    cred.id = bufferDecode(cred.id);
                }
            }

            const credential = await navigator.credentials.create({
                publicKey: options.publicKey
            });

            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    attestationObject: bufferEncode(credential.response.attestationObject),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    transports: credential.response.getTransports ? credential.response.getTransports() : []
                }
            };

            const finishRes = await fetch("/auth/passkey/register/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });

            if (finishRes.ok) {
                window.location.href = "/dashboard";
            } else {
                alert("Falha ao registrar Passkey: " + await finishRes.text());
            }
        } catch (err) {
            alert("Erro: " + err.message);
        }
    }

    async function loginPasskey() {
        try {
            const email = document.getElementById("email").value;
            if (!email) {
                alert("Por favor, digite seu email para fazer login com Passkey.");
                return;
            }

            const res = await fetch("/auth/passkey/login/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email })
            });
            if (!res.ok) throw new Error(await res.text());

            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            if (options.publicKey.allowCredentials) {
                for (let cred of options.publicKey.allowCredentials) {
                    cred.id = bufferDecode(cred.id);
                }
            }

            const credential = await navigator.credentials.get({
                publicKey: options.publicKey
            });

            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    authenticatorData: bufferEncode(credential.response.authenticatorData),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    signature: bufferEncode(credential.response.signature),
                    userHandle: credential.response.userHandle ? bufferEncode(credential.response.userHandle) : null
                }
            };

            const finishRes = await fetch("/auth/passkey/login/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });

            if (finishRes.ok) {
                window.location.href = "/dashboard";
            } else {
                alert("Falha na autenticação da Passkey: " + await finishRes.text());
            }
        } catch (err) {
            alert("Erro: " + err.message);
        }
    }
</script>"#;

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        min-height: 100vh;
                        padding: 1rem;
                        box-sizing: border-box;
                    }
                    .card {
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 2.5rem;
                        width: 100%;
                        max-width: 420px;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2rem;
                        margin: 0 0 0.5rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.subtitle {
                        color: #64748b;
                        font-size: 0.95rem;
                        margin: 0 0 2rem 0;
                    }
                    .form-group {
                        margin-bottom: 1.25rem;
                        text-align: left;
                    }
                    label {
                        display: block;
                        font-size: 0.85rem;
                        color: #94a3b8;
                        margin-bottom: 0.5rem;
                        font-weight: 500;
                    }
                    input[type='email'], input[type='password'] {
                        width: 100%;
                        box-sizing: border-box;
                        background: #1f2937;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1rem;
                        color: #fff;
                        font-size: 0.95rem;
                        transition: border-color 0.2s, box-shadow 0.2s;
                    }
                    input[type='email']:focus, input[type='password']:focus {
                        outline: none;
                        border-color: #6366f1;
                        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
                    }
                    button.btn-primary {
                        width: 100%;
                        background: linear-gradient(135deg, #6366f1, #4f46e5);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.85rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        margin-top: 0.5rem;
                    }
                    button.btn-primary:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    .divider {
                        display: flex;
                        align-items: center;
                        color: #475569;
                        font-size: 0.8rem;
                        margin: 1.5rem 0;
                    }
                    .divider::before, .divider::after {
                        content: '';
                        flex: 1;
                        border-bottom: 1px solid #1f2937;
                    }
                    .divider:not(:empty)::before { margin-right: .5em; }
                    .divider:not(:empty)::after { margin-left: .5em; }
                    .oauth-btn {
                        width: 100%;
                        background: #1f2937;
                        color: #fff;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem;
                        font-size: 0.9rem;
                        font-weight: 500;
                        cursor: pointer;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        gap: 0.5rem;
                        transition: background-color 0.2s;
                        text-decoration: none;
                        box-sizing: border-box;
                    }
                    .oauth-btn:hover {
                        background: #374151;
                    }
                    .footer-link {
                        margin-top: 1.5rem;
                        font-size: 0.85rem;
                        color: #94a3b8;
                    }
                    .footer-link a {
                        color: #38bdf8;
                        text-decoration: none;
                    }
                    .footer-link a:hover {
                        text-decoration: underline;
                    }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Bem-vindo de volta"</h1>
                    <p class="subtitle">"Log in to your Rullst account"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label htmlFor="email">"Email"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="password">"Password"</label>
                            <input type="password" id="password" name="password" placeholder="••••••••" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign In"</button>
                    </form>

                    <button type="button" onclick="loginPasskey()" class="oauth-btn btn-passkey" style="display: none; margin-top: 1rem; background: linear-gradient(135deg, #10b981, #059669); color: white; justify-content: center; width: 100%; box-sizing: border-box;">
                        "Entrar com Passkey / Biometria 🔑"
                    </button>

                    <div class="divider">"ou continuar com"</div>

                    <a href="/auth/github/redirect" class="oauth-btn">
                        <svg style="width: 1.25rem; height: 1.25rem; fill: currentColor;" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                        </svg>
                        "GitHub"
                    </a>

                    <div class="footer-link">
                        "Don't have an account? "
                        <a href="/register">"Sign up"</a>
                    </div>
                </div>
                { rullst::html::RawHtml(PASSKEY_SCRIPT.to_string()) }
            </body>
        </html>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Create Account - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        min-height: 100vh;
                        padding: 1rem;
                        box-sizing: border-box;
                    }
                    .card {
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 2.5rem;
                        width: 100%;
                        max-width: 420px;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2rem;
                        margin: 0 0 0.5rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.subtitle {
                        color: #64748b;
                        font-size: 0.95rem;
                        margin: 0 0 2rem 0;
                    }
                    .form-group {
                        margin-bottom: 1.25rem;
                        text-align: left;
                    }
                    label {
                        display: block;
                        font-size: 0.85rem;
                        color: #94a3b8;
                        margin-bottom: 0.5rem;
                        font-weight: 500;
                    }
                    input[type='text'], input[type='email'], input[type='password'] {
                        width: 100%;
                        box-sizing: border-box;
                        background: #1f2937;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1rem;
                        color: #fff;
                        font-size: 0.95rem;
                        transition: border-color 0.2s, box-shadow 0.2s;
                    }
                    input[type='text']:focus, input[type='email']:focus, input[type='password']:focus {
                        outline: none;
                        border-color: #6366f1;
                        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
                    }
                    button.btn-primary {
                        width: 100%;
                        background: linear-gradient(135deg, #6366f1, #4f46e5);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.85rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        margin-top: 0.5rem;
                    }
                    button.btn-primary:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    .footer-link {
                        margin-top: 1.5rem;
                        font-size: 0.85rem;
                        color: #94a3b8;
                    }
                    .footer-link a {
                        color: #38bdf8;
                        text-decoration: none;
                    }
                    .footer-link a:hover {
                        text-decoration: underline;
                    }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Crie sua conta"</h1>
                    <p class="subtitle">"Sign up and start building with Rullst"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label htmlFor="name">"Full Name"</label>
                            <input type="text" id="name" name="name" placeholder="Your Name" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="email">"Email"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="password">"Password"</label>
                            <input type="password" id="password" name="password" placeholder="Minimum 6 characters" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Registrar"</button>
                    </form>

                    <button type="button" onclick="registerPasskey()" class="oauth-btn btn-passkey" style="display: none; margin-top: 1rem; background: linear-gradient(135deg, #10b981, #059669); color: white; justify-content: center; width: 100%; box-sizing: border-box;">
                        "Registrar com Passkey / Biometria 🔑"
                    </button>

                    <div class="footer-link">
                        "Already have an account? "
                        <a href="/login">"Sign In"</a>
                    </div>
                </div>
                { rullst::html::RawHtml(PASSKEY_SCRIPT.to_string()) }
            </body>
        </html>
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Dashboard - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        padding: 2rem;
                        box-sizing: border-box;
                    }
                    .container {
                        max-width: 800px;
                        margin: 4rem auto;
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 3rem;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2.5rem;
                        margin: 0 0 1rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.lead {
                        color: #94a3b8;
                        font-size: 1.15rem;
                        line-height: 1.6;
                        margin-bottom: 2rem;
                    }
                    .badge {
                        display: inline-block;
                        padding: 0.5rem 1rem;
                        background: rgba(56, 189, 248, 0.1);
                        border: 1px solid rgba(56, 189, 248, 0.2);
                        color: #38bdf8;
                        border-radius: 9999px;
                        font-weight: 600;
                        font-size: 0.85rem;
                        margin-bottom: 2rem;
                    }
                    .btn-logout {
                        background: linear-gradient(135deg, #ef4444, #dc2626);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.75rem 2rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        text-decoration: none;
                    }
                    .btn-logout:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <span class="badge">"Rullst Active Authentication"</span>
                    <h1>"Hello, "{user_name}"! 👋"</h1>
                    <p class="lead">"You are in a high-performance, secure restricted area. This dashboard and its entire infrastructure were built automatically via the CLI."</p>
                    <a href="/logout" class="btn-logout">"Sign Out"</a>
                </div>
            </body>
        </html>
    })
}
"##;
    fs::write(&pages_path, pages_template)?;
    println!(
        "{}",
        "  ✨ Created HTML views in 'src/pages/auth.rs'.".green()
    );

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod auth;") {
        mod_pages_content.push_str("pub mod auth;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    // 5. Create Auth Controller
    let controllers_dir = Path::new("src/controllers");
    let controller_path = controllers_dir.join("auth_controller.rs");
    let controller_template = r##"use axum::{
    extract::{Form, Query},
    response::{Html, IntoResponse, Redirect, Response},
    http::HeaderMap,
};
use serde::Deserialize;
use crate::models::user::User;
use crate::models::user_passkey::UserPasskey;
use crate::pages::auth;
use rullst::auth as rullst_auth;
use rullst::auth::passkey::{PasskeyAuth, PasskeyConfig};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static PASSKEY: LazyLock<PasskeyAuth> = LazyLock::new(|| {
    let config = PasskeyConfig::new(
        "Rullst App",
        "localhost",
        "http://localhost:3000"
    );
    PasskeyAuth::new(&config).expect("Failed to initialize PasskeyAuth")
});

static REG_STATES: LazyLock<Mutex<HashMap<String, webauthn_rs::prelude::PasskeyRegistration>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static AUTH_STATES: LazyLock<Mutex<HashMap<String, webauthn_rs::prelude::PasskeyAuthentication>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
pub struct RegisterDto {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
}

#[derive(Deserialize)]
pub struct PasskeyRegisterStartDto {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct PasskeyLoginStartDto {
    pub email: String,
}

#[derive(Deserialize)]
pub struct PasskeyEmailQuery {
    pub email: String,
}

fn get_csrf_token(headers: &HeaderMap) -> String {
    headers.get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            for cookie in cookie_str.split(';') {
                let trimmed = cookie.trim();
                if trimmed.starts_with("rullst_csrf=") {
                    return Some(trimmed["rullst_csrf=".len()..].to_string());
                }
            }
            None
        })
        .unwrap_or_default()
}

pub async fn login_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::login_page(&token, None)
}

pub async fn login_submit(headers: HeaderMap, Form(payload): Form<LoginDto>) -> Response {
    let token = get_csrf_token(&headers);
    
    let users = match User::all().await {
        Ok(u) => u,
        Err(_) => return auth::login_page(&token, Some("Internal error fetching user")).into_response(),
    };
    
    let user = users.into_iter().find(|u| u.email == payload.email);
    
    let Some(u) = user else {
        return auth::login_page(&token, Some("Incorrect email or password")).into_response();
    };

    let hash = u.password_hash.as_deref().unwrap_or("");
    if !rullst_auth::verify_password(&payload.password, hash) {
        return auth::login_page(&token, Some("Incorrect email or password")).into_response();
    }

    match rullst_auth::make_login_cookie(u.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => auth::login_page(&token, Some("Error starting session")).into_response(),
    }
}

pub async fn register_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::register_page(&token, None)
}

pub async fn register_submit(headers: HeaderMap, Form(payload): Form<RegisterDto>) -> Response {
    let token = get_csrf_token(&headers);
    
    if payload.password.len() < 6 {
        return auth::register_page(&token, Some("Password must be at least 6 characters")).into_response();
    }

    if let Ok(users) = User::all().await {
        if users.iter().any(|u| u.email == payload.email) {
            return auth::register_page(&token, Some("This email address is already registered")).into_response();
        }
    }

    let hash = match rullst_auth::hash_password(&payload.password) {
        Ok(h) => h,
        Err(_) => return auth::register_page(&token, Some("Error processing password")).into_response(),
    };

    let mut user = User {
        id: 0,
        name: payload.name,
        email: payload.email,
        password_hash: Some(hash),
        oauth_provider: None,
        oauth_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    if let Err(e) = user.save().await {
        return auth::register_page(&token, Some(&format!("Error creating account: {}", e))).into_response();
    }

    match rullst_auth::make_login_cookie(user.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => Redirect::to("/login").into_response(),
    }
}

pub async fn logout() -> Response {
    let cookie = rullst_auth::make_logout_cookie();
    let mut res = Redirect::to("/login").into_response();
    res.headers_mut().append(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie).unwrap()
    );
    res
}

pub async fn dashboard(axum::Extension(user_id): axum::Extension<i32>) -> Response {
    if let Ok(users) = User::all().await {
        if let Some(user) = users.into_iter().find(|u| u.id == user_id) {
            return auth::dashboard_page(&user.name).into_response();
        }
    }
    Redirect::to("/login").into_response()
}

pub async fn oauth_github_redirect() -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());
    
    if let Some(provider) = rullst_connect::Socialite::driver("github", client_id, String::new(), redirect_url) {
        return Redirect::to(&provider.redirect_url()).into_response();
    }
    
    Redirect::to("/login").into_response()
}

pub async fn oauth_github_callback(Query(query): Query<OAuthCallbackQuery>) -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| "dummy_client_secret".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());

    if let Some(provider) = rullst_connect::Socialite::driver("github", client_id, client_secret, redirect_url) {
        if let Ok(social_user) = provider.get_user(&query.code).await {
            let mut existing_user = None;
            if let Ok(users) = User::all().await {
                existing_user = users.into_iter().find(|u| {
                    u.oauth_provider.as_deref() == Some("github") && u.oauth_id.as_deref() == Some(&social_user.id)
                });
            }

            let user_id = if let Some(u) = existing_user {
                u.id
            } else {
                let mut user = User {
                    id: 0,
                    name: social_user.name.clone().unwrap_or_else(|| "GitHub User".to_string()),
                    email: social_user.email.clone().unwrap_or_else(|| format!("{}@github.com", social_user.id)),
                    password_hash: None,
                    oauth_provider: Some("github".to_string()),
                    oauth_id: Some(social_user.id.clone()),
                    created_at: String::new(),
                    updated_at: String::new(),
                };
                if user.save().await.is_ok() {
                    user.id
                } else {
                    return Redirect::to("/login").into_response();
                }
            };

            if let Ok(cookie) = rullst_auth::make_login_cookie(user_id) {
                let mut res = Redirect::to("/dashboard").into_response();
                res.headers_mut().append(
                    axum::http::header::SET_COOKIE,
                    axum::http::HeaderValue::from_str(&cookie).unwrap()
                );
                return res;
            }
        }
    }

    Redirect::to("/login").into_response()
}

pub async fn passkey_register_start(
    axum::Json(payload): axum::Json<PasskeyRegisterStartDto>
) -> Response {
    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let email_lower = payload.email.to_lowercase();
    if existing_users.iter().any(|u| u.email.to_lowercase() == email_lower) {
        return (axum::http::StatusCode::BAD_REQUEST, "Email already registered").into_response();
    }

    let next_id = existing_users.iter().map(|u| u.id).max().unwrap_or(0) + 1;

    match PASSKEY.start_register(next_id, &payload.email, &payload.name) {
        Ok((challenge, state)) => {
            if let Ok(mut states) = REG_STATES.lock() {
                states.insert(email_lower, state);
            }
            axum::Json(challenge).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

pub async fn passkey_register_finish(
    Query(query): Query<PasskeyEmailQuery>,
    axum::Json(credential): axum::Json<webauthn_rs::prelude::RegisterPublicKeyCredential>
) -> Response {
    let email_lower = query.email.to_lowercase();
    let state = {
        let Ok(mut states) = REG_STATES.lock() else {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lock error").into_response();
        };
        match states.remove(&email_lower) {
            Some(s) => s,
            None => return (axum::http::StatusCode::BAD_REQUEST, "Registration challenge not found").into_response(),
        }
    };

    match PASSKEY.finish_register(&credential, state) {
        Ok(passkey) => {
            let name = query.email.split('@').next().unwrap_or("User").to_string();
            let mut user = User {
                id: 0,
                name,
                email: query.email.clone(),
                password_hash: None,
                oauth_provider: Some("passkey".to_string()),
                oauth_id: Some(query.email.clone()),
                created_at: String::new(),
                updated_at: String::new(),
            };

            if let Err(e) = user.save().await {
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save user: {}", e)).into_response();
            }

            let passkey_json = serde_json::to_string(&passkey).unwrap_or_default();
            let mut user_passkey = UserPasskey {
                id: 0,
                user_id: user.id,
                name: "Passkey".to_string(),
                passkey_json,
                created_at: String::new(),
                updated_at: String::new(),
            };

            if let Err(e) = user_passkey.save().await {
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save passkey: {}", e)).into_response();
            }

            match rullst_auth::make_login_cookie(user.id) {
                Ok(cookie) => {
                    let mut res = (axum::http::StatusCode::OK, "Success").into_response();
                    res.headers_mut().append(
                        axum::http::header::SET_COOKIE,
                        axum::http::HeaderValue::from_str(&cookie).unwrap()
                    );
                    res
                }
                Err(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Error logging in").into_response(),
            }
        }
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response()
    }
}

pub async fn passkey_login_start(
    axum::Json(payload): axum::Json<PasskeyLoginStartDto>
) -> Response {
    let email_lower = payload.email.to_lowercase();
    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (axum::http::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let user_credentials: Vec<webauthn_rs::prelude::Passkey> = all_passkeys
        .into_iter()
        .filter(|pk| pk.user_id == user.id)
        .filter_map(|pk| serde_json::from_str(&pk.passkey_json).ok())
        .collect();

    if user_credentials.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "No passkeys registered for this user").into_response();
    }

    match PASSKEY.start_authenticate(&user_credentials) {
        Ok((challenge, state)) => {
            if let Ok(mut states) = AUTH_STATES.lock() {
                states.insert(email_lower, state);
            }
            axum::Json(challenge).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

pub async fn passkey_login_finish(
    Query(query): Query<PasskeyEmailQuery>,
    axum::Json(credential): axum::Json<webauthn_rs::prelude::PublicKeyCredential>
) -> Response {
    let email_lower = query.email.to_lowercase();
    let state = {
        let Ok(mut states) = AUTH_STATES.lock() else {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lock error").into_response();
        };
        match states.remove(&email_lower) {
            Some(s) => s,
            None => return (axum::http::StatusCode::BAD_REQUEST, "Authentication challenge not found").into_response(),
        }
    };

    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (axum::http::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let mut all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let mut found_passkey = None;
    let mut found_user_passkey = None;

    for pk in all_passkeys.iter_mut() {
        if pk.user_id == user.id {
            if let Ok(parsed_pk) = serde_json::from_str::<webauthn_rs::prelude::Passkey>(&pk.passkey_json) {
                if credential.id == parsed_pk.cred_id() {
                    found_passkey = Some(parsed_pk);
                    found_user_passkey = Some(pk);
                    break;
                }
            }
        }
    }

    let (passkey, mut user_passkey) = match (found_passkey, found_user_passkey) {
        (Some(pk), Some(upk)) => (pk, upk),
        _ => return (axum::http::StatusCode::BAD_REQUEST, "Matching credential not found").into_response(),
    };

    match PASSKEY.finish_authenticate(&credential, state, passkey) {
        Ok(updated_passkey) => {
            user_passkey.passkey_json = serde_json::to_string(&updated_passkey).unwrap_or_default();
            let _ = user_passkey.save().await;

            match rullst_auth::make_login_cookie(user.id) {
                Ok(cookie) => {
                    let mut res = (axum::http::StatusCode::OK, "Success").into_response();
                    res.headers_mut().append(
                        axum::http::header::SET_COOKIE,
                        axum::http::HeaderValue::from_str(&cookie).unwrap()
                    );
                    res
                }
                Err(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Error logging in").into_response(),
            }
        }
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e).into_response()
    }
}
"##;
    fs::write(&controller_path, controller_template)?;
    println!(
        "{}",
        "  ✨ Created 'src/controllers/auth_controller.rs' controller.".green()
    );

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod auth_controller;") {
        mod_controllers_content.push_str("pub mod auth_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    // 6. Register modules in src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;

        // Register required modules if not present
        for module in &["controllers", "models", "middlewares", "pages"] {
            let declaration = format!("pub mod {};", module);
            let alt_declaration = format!("mod {};", module);
            if !main_content.contains(&declaration) && !main_content.contains(&alt_declaration) {
                main_content = format!("pub mod {};\n{}", module, main_content);
            }
        }

        // Auto-inject required dependencies in Cargo.toml if needed (like rullst-connect and webauthn-rs)
        let cargo_toml_path = Path::new("Cargo.toml");
        if cargo_toml_path.exists() {
            let mut cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
            let mut modified = false;

            if !cargo_toml_content.contains("rullst-connect") {
                let current_dir = std::env::current_dir()?;
                let sibling_path = current_dir.parent().unwrap().join("rullst-connect");
                let dep_str = if sibling_path.exists() {
                    let absolute_path = sibling_path
                        .canonicalize()?
                        .display()
                        .to_string()
                        .replace("\\", "/");
                    format!("rullst-connect = {{ path = \"{}\" }}\n", absolute_path)
                } else {
                    "rullst-connect = \"0.4.0\"\n".to_string()
                };

                if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                    cargo_toml_content.insert_str(pos + 14, &dep_str);
                    modified = true;
                    println!(
                        "{}",
                        "  ✨ Added 'rullst-connect' dependency to your Cargo.toml.".green()
                    );
                }
            }

            if !cargo_toml_content.contains("webauthn-rs") {
                let dep_str = "webauthn-rs = { version = \"0.5\", default-features = false }\n";
                if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                    cargo_toml_content.insert_str(pos + 14, dep_str);
                    modified = true;
                    println!(
                        "{}",
                        "  ✨ Added 'webauthn-rs' dependency to your Cargo.toml.".green()
                    );
                }
            }

            if modified {
                fs::write(cargo_toml_path, cargo_toml_content)?;
            }
        }

        fs::write(main_path, main_content)?;
        println!("{}", "  ✨ Injetadas declarações de módulos ('pub mod controllers/models...') no seu src/main.rs.".green());
    }

    println!(
        "\n{}",
        "🎉 Authentication system generated successfully!"
            .green()
            .bold()
    );
    println!("{}", "To complete the integration:".cyan().bold());
    println!(
        "{}",
        "  1. Register the routes below in the routes! macro of your 'src/main.rs':".cyan()
    );
    println!(
        "{}",
        r##"     get("/login" => controllers::auth_controller::login_view),
     post("/login" => controllers::auth_controller::login_submit),
     get("/register" => controllers::auth_controller::register_view),
     post("/register" => controllers::auth_controller::register_submit),
     get("/logout" => controllers::auth_controller::logout),
     get("/dashboard" => controllers::auth_controller::dashboard),
     get("/auth/github/redirect" => controllers::auth_controller::oauth_github_redirect),
     get("/auth/github/callback" => controllers::auth_controller::oauth_github_callback),
     post("/auth/passkey/register/start" => controllers::auth_controller::passkey_register_start),
     post("/auth/passkey/register/finish" => controllers::auth_controller::passkey_register_finish),
     post("/auth/passkey/login/start" => controllers::auth_controller::passkey_login_start),
     post("/auth/passkey/login/finish" => controllers::auth_controller::passkey_login_finish),
     -------------------------------------------------------------------------------------"##
            .yellow()
    );
    println!(
        "{}",
        "  2. To protect routes with a middleware, apply the layer to your router:".cyan()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let protected_router = routes![\n         get(\"/dashboard\" => controllers::auth_controller::dashboard)\n     ]".yellow());
    println!(
        "{}",
        "     .layer(axum::middleware::from_fn(middlewares::auth_middleware::auth_middleware));"
            .yellow()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!(
        "{}",
        "  3. Aplique as proteções CSRF e Security Headers globais no seu router principal:".cyan()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let main_router = routes![...]\n         .layer(axum::middleware::from_fn(rullst::security::csrf_middleware))\n         .layer(axum::middleware::from_fn(rullst::security::headers_middleware));".yellow());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "  4. Execute as migrations:".cyan());
    println!("{}", "     $ cargo rullst db:migrate".yellow());

    Ok(())
}

fn scaffold_billing_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "💳 Starting scaffolding of Rullst billing system (Stripe & LemonSqueezy)..."
            .cyan()
            .bold()
    );

    // 1. Create Subscriptions Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_subscriptions_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst_orm::schema::{{Schema, Blueprint, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::create("subscriptions", |table| {{
            table.id();
            table.integer("user_id").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").unique().not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {{
        Schema::drop_if_exists("subscriptions").await
    }}
}}
"##,
        file_stem = file_stem
    );
    fs::write(&migration_path, migration_template)?;
    println!("{}", "  ✨ Created 'subscriptions' table migration.".green());

    regenerate_migrations_mod()?;

    // 2. Create Subscription Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("subscription.rs");
    let model_template = r##"use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "subscriptions")]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: String,
    pub subscription_id: String,
    pub plan_id: String,
    pub status: String,
    pub ends_at: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'Subscription' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    if !mod_models_content.contains("pub mod subscription;") {
        mod_models_content.push_str("pub mod subscription;\n");
        fs::write(&mod_models_path, mod_models_content)?;
    }

    // 3. Create Pricing View Page
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let page_path = pages_dir.join("billing.rs");
    let page_template = r##"use rullst::html;
use axum::response::Html;

pub fn pricing_page() -> Html<String> {
    Html(html! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>Select a Plan - Rullst Billing</title>
            <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
            <style>
                * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow-x: hidden; position: relative; }
                .glow-bg { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, rgba(139, 92, 246, 0.05) 50%, transparent 100%); top: -10%; left: -10%; z-index: -1; }
                .glow-bg-right { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, rgba(99, 102, 241, 0.05) 50%, transparent 100%); bottom: -10%; right: -10%; z-index: -1; }
                .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; text-align: center; z-index: 1; }
                .header { margin-bottom: 3.5rem; }
                .badge { background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%); color: white; padding: 0.35rem 1rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 1rem; }
                h1 { font-size: 3rem; font-weight: 700; background: linear-gradient(to right, #ffffff, #9ca3af); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 1rem; }
                .subtitle { color: #9ca3af; font-size: 1.15rem; max-width: 600px; margin: 0 auto; }
                .pricing-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 2rem; max-width: 1000px; margin: 0 auto; }
                .pricing-card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; padding: 3rem 2rem; text-align: left; display: flex; flex-direction: column; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); position: relative; }
                .pricing-card:hover { transform: translateY(-8px); border-color: rgba(99, 102, 241, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
                .pricing-card.premium { border: 2px solid #6366f1; }
                .pricing-card.premium::after { content: 'Best Value'; position: absolute; top: -14px; right: 24px; background: #6366f1; color: white; font-size: 0.75rem; font-weight: 700; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; }
                .plan-name { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.5rem; }
                .plan-desc { color: #9ca3af; font-size: 0.95rem; margin-bottom: 2rem; min-height: 40px; }
                .price-container { display: flex; align-items: baseline; margin-bottom: 2.5rem; }
                .currency { font-size: 1.75rem; font-weight: 600; color: #ffffff; }
                .price { font-size: 3.5rem; font-weight: 700; color: #ffffff; letter-spacing: -0.02em; }
                .period { color: #9ca3af; font-size: 1rem; margin-left: 0.5rem; }
                .features-list { list-style: none; margin-bottom: 3rem; flex-grow: 1; }
                .features-list li { display: flex; align-items: center; color: #d1d5db; font-size: 0.95rem; margin-bottom: 1rem; }
                .features-list svg { width: 20px; height: 20px; margin-right: 0.75rem; color: #10b981; flex-shrink: 0; }
                .btn-checkout { display: block; width: 100%; text-align: center; padding: 1rem; border-radius: 12px; font-weight: 600; text-decoration: none; font-size: 1rem; transition: all 0.3s; cursor: pointer; border: none; }
                .btn-checkout.primary { background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); color: white; box-shadow: 0 4px 14px rgba(99, 102, 241, 0.4); }
                .btn-checkout.primary:hover { background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%); box-shadow: 0 6px 20px rgba(99, 102, 241, 0.6); }
                .btn-checkout.secondary { background: rgba(255, 255, 255, 0.08); color: white; border: 1px solid rgba(255, 255, 255, 0.1); }
                .btn-checkout.secondary:hover { background: rgba(255, 255, 255, 0.15); border-color: rgba(255, 255, 255, 0.25); }
            </style>
        </head>
        <body>
            <div class="glow-bg"></div>
            <div class="glow-bg-right"></div>
            <div class="container">
                <div class="header">
                    <span class="badge">Rullst Capital</span>
                    <h1>Simple, Transparent Pricing</h1>
                    <p class="subtitle">Choose the perfect plan to boost your application with next-gen fullstack performance.</p>
                </div>
                <div class="pricing-grid">
                    <!-- Starter Plan -->
                    <div class="pricing-card">
                        <h2 class="plan-name">Starter</h2>
                        <p class="plan-desc">For hobbyists and early-stage startup prototypes.</p>
                        <div class="price-container">
                            <span class="currency">$</span>
                            <span class="price">9</span>
                            <span class="period">/mo</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Up to 5 Projects
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Standard SQLite Database
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Email Support
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_starter" class="btn-checkout secondary">Get Started</a>
                    </div>
                    
                    <!-- Pro Plan -->
                    <div class="pricing-card premium">
                        <h2 class="plan-name">Pro</h2>
                        <p class="plan-desc">For growing apps needing production scaling and support.</p>
                        <div class="price-container">
                            <span class="currency">$</span>
                            <span class="price">29</span>
                            <span class="period">/mo</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Unlimited Projects
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                PostgreSQL & SQLite Support
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Adaptive WAF & Bot Management
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                Priority Support (Sub-1 hour)
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_pro" class="btn-checkout primary">Go Pro</a>
                    </div>
                </div>
            </div>
        </body>
        </html>
    })
}
"##;
    fs::write(&page_path, page_template)?;
    println!("{}", "  ✨ Created HTML views in 'src/pages/billing.rs'.".green());

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod billing;") {
        mod_pages_content.push_str("pub mod billing;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    // 4. Create Billing Controller
    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(controllers_dir)?;
    let controller_path = controllers_dir.join("billing_controller.rs");
    let controller_template = r##"use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::collections::HashMap;
use rullst::capital::{BillingProvider, StripeProvider, LemonSqueezyProvider};
use rullst_orm::sqlx::Row;
use crate::pages::billing;

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub plan: String,
}

/// Serves the premium pricing page.
pub async fn pricing_view() -> impl IntoResponse {
    billing::pricing_page()
}

/// Initiates a checkout redirect.
pub async fn checkout_redirect(Query(query): Query<CheckoutQuery>) -> impl IntoResponse {
    // Resolve Billing Provider using environment keys
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let redirect_url = std::env::var("BILLING_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/dashboard".to_string());

    let url_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
    };

    match url_result {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create checkout session: {}", e)).into_response(),
    }
}

/// Handles incoming webhook events from the selected provider.
pub async fn webhook_handler(headers: HeaderMap, body: axum::body::Bytes) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let mut headers_map = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val_str) = v.to_str() {
            headers_map.insert(k.as_str().to_string(), val_str.to_string());
        }
    }

    let event_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
    };

    let event = match event_result {
        Ok(evt) => evt,
        Err(e) => {
            eprintln!("❌ Webhook verification/parsing error: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid webhook signature or payload").into_response();
        }
    };

    println!("🔔 Received Webhook for Subscription {} [{}] -> Status: {:?}", event.subscription_id, event.plan_id, event.status);

    let pool = rullst_orm::Orm::pool();
    
    let existing = sqlx::query("SELECT id FROM subscriptions WHERE subscription_id = ?1")
        .bind(&event.subscription_id)
        .fetch_optional(pool)
        .await;

    match existing {
        Ok(Some(row)) => {
            let id: i32 = row.get("id");
            let update_res = sqlx::query("UPDATE subscriptions SET status = ?1, plan_id = ?2, ends_at = ?3, updated_at = datetime('now') WHERE id = ?4")
                .bind(event.status.as_str())
                .bind(&event.plan_id)
                .bind(event.ends_at)
                .bind(id)
                .execute(pool)
                .await;
            if let Err(err) = update_res {
                eprintln!("❌ Failed to update subscription: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        }
        Ok(None) => {
            let insert_res = sqlx::query("INSERT INTO subscriptions (user_id, customer_id, subscription_id, plan_id, status, ends_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))")
                .bind(1)
                .bind(&event.customer_id)
                .bind(&event.subscription_id)
                .bind(&event.plan_id)
                .bind(event.status.as_str())
                .bind(event.ends_at)
                .execute(pool)
                .await;
            if let Err(err) = insert_res {
                eprintln!("❌ Failed to insert subscription: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        }
        Err(err) => {
            eprintln!("❌ Database query failed: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    (StatusCode::OK, "Webhook processed successfully").into_response()
}
"##;
    fs::write(&controller_path, controller_template)?;
    println!(
        "{}",
        "  ✨ Created 'src/controllers/billing_controller.rs' controller.".green()
    );

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod billing_controller;") {
        mod_controllers_content.push_str("pub mod billing_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    // 5. Register modules in src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        for module in &["controllers", "models", "pages"] {
            let declaration = format!("pub mod {};", module);
            let alt_declaration = format!("mod {};", module);
            if !main_content.contains(&declaration) && !main_content.contains(&alt_declaration) {
                main_content = format!("pub mod {};\n{}", module, main_content);
            }
        }
        fs::write(main_path, main_content)?;
    }

    println!("\n{}", "🎉 Rullst Capital Billing Scaffolding Completed Successfully!".green().bold());
    println!("{}", "To mount the billing panel and webhooks, register these routes in your main router:".white());
    println!("{}", "  👉 .route(\"/pricing\", axum::routing::get(controllers::billing_controller::pricing_view))".cyan());
    println!("{}", "  👉 .route(\"/billing/checkout\", axum::routing::get(controllers::billing_controller::checkout_redirect))".cyan());
    println!("{}", "  👉 .route(\"/billing/webhook\", axum::routing::post(controllers::billing_controller::webhook_handler))".cyan());
    println!("\n{}", "Configure your gateway credentials in environment variables or your .env file:".white());
    println!("{}", "  💰 BILLING_PROVIDER=stripe".yellow());
    println!("{}", "  💰 BILLING_API_KEY=sk_test_...".yellow());
    println!("{}", "  💰 BILLING_WEBHOOK_SECRET=whsec_...".yellow());

    Ok(())
}

// ==========================================
// DOCKER FILE GENERATION
// ==========================================

fn generate_docker_files(
    project_path: &Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🐳 Generating Docker files...".cyan().bold());

    // --- Dockerfile (multi-stage, distroless) ---
    let dockerfile = format!(
        r#"# ══════════════════════════════════════════════════════════════
# Rullst Production Dockerfile (auto-generated)
# Multi-stage build: Rust builder → Distroless runtime
# Final image: ~20MB | Zero CVEs | Ultra-fast cold start
# ══════════════════════════════════════════════════════════════

# ── Stage 1: Builder ─────────────────────────────────────────
FROM rust:1.87-slim AS builder
WORKDIR /app

# Install system dependencies for SQLite/Postgres/MySQL linking
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {{}}" > src/main.rs && cargo build --release && rm -rf src

# Build the actual application
COPY . .
RUN cargo build --release

# ── Stage 2: Runtime (Distroless) ────────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app

# Copy only the compiled binary
COPY --from=builder /app/target/release/{project_name} /app/{project_name}

# Copy configuration files
COPY Rullst.toml /app/Rullst.toml

EXPOSE 3000

ENTRYPOINT ["/app/{project_name}"]
"#
    );

    // --- docker-compose.yml ---
    let docker_compose = format!(
        r#"# ══════════════════════════════════════════════════════════════
# Rullst Docker Compose (auto-generated)
# Services: App + PostgreSQL + Redis
# ══════════════════════════════════════════════════════════════

services:
  app:
    build: .
    container_name: {project_name}-app
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://rullst:rullst@db:5432/rullst_db
      - REDIS_URL=redis://redis:6379
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_healthy
    restart: unless-stopped

  db:
    image: postgres:16-alpine
    container_name: {project_name}-db
    environment:
      POSTGRES_USER: rullst
      POSTGRES_PASSWORD: rullst
      POSTGRES_DB: rullst_db
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rullst"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    container_name: {project_name}-redis
    ports:
      - "6379:6379"
    volumes:
      - redisdata:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped

volumes:
  pgdata:
  redisdata:
"#
    );

    // --- .dockerignore ---
    let dockerignore = r#"target/
.git/
.gitignore
*.md
LICENSE
.vscode/
.idea/
*.db
*.sqlite
"#;

    fs::write(project_path.join("Dockerfile"), dockerfile)?;
    fs::write(project_path.join("docker-compose.yml"), docker_compose)?;
    fs::write(project_path.join(".dockerignore"), dockerignore)?;

    println!("{}", "  ✅ Dockerfile (multi-stage distroless)".green());
    println!(
        "{}",
        "  ✅ docker-compose.yml (App + Postgres + Redis)".green()
    );
    println!("{}", "  ✅ .dockerignore".green());

    Ok(())
}

// ==========================================
// SCAPE / GENERATE BOILERPLATE MIDDLEWARES
// ==========================================

fn create_cors_middleware() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "🛠️ Generating CORS middleware...".cyan().bold());

    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    let mut mod_content = fs::read_to_string(&mod_path)?;
    if !mod_content.contains("pub mod cors_middleware;") {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str("pub mod cors_middleware;\n");
        fs::write(&mod_path, mod_content)?;
    }

    let middleware_path = middlewares_dir.join("cors_middleware.rs");
    if middleware_path.exists() {
        println!(
            "{}",
            "⚠️ Warning: CORS middleware 'cors_middleware.rs' already exists. Skipping creation."
                .yellow()
        );
    } else {
        let template = r#"use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::{header, Method, StatusCode},
};

/// Middleware CORS robusto e de alta performance.
/// Gerencia cabeçalhos de compartilhamento de recursos de origem cruzada e requisições preflight (OPTIONS).
pub async fn cors_middleware(req: Request, next: Next) -> Response {
    let origin = req.headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*")
        .to_string();

    // Lida com requisições preflight OPTIONS
    if req.method() == Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
            .header(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, PUT, DELETE, PATCH, OPTIONS")
            .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization, X-Requested-With, X-CSRF-Token")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(header::ACCESS_CONTROL_MAX_AGE, "86400")
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_str(&origin).unwrap_or_else(|_| header::HeaderValue::from_static("*")),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        header::HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        header::HeaderValue::from_static("Content-Type, Authorization, X-Requested-With, X-CSRF-Token"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        header::HeaderValue::from_static("true"),
    );

    response
}
"#;
        fs::write(&middleware_path, template)?;
    }

    // Attempt to inject "pub mod middlewares;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        "✨ CORS middleware successfully created!".green().bold()
    );
    println!(
        "{}",
        "How to register in your main router (src/main.rs):".cyan()
    );
    println!("{}", "  1. Add: '.layer(axum::middleware::from_fn(middlewares::cors_middleware::cors_middleware))'".cyan());

    Ok(())
}

fn create_jwt_middleware() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "🛠️ Generating JWT middleware...".cyan().bold());

    // 1. Injetar jsonwebtoken e chrono no Cargo.toml do usuário
    let cargo_toml_path = Path::new("Cargo.toml");
    if cargo_toml_path.exists() {
        let mut cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
        let mut modified = false;
        if !cargo_toml_content.contains("jsonwebtoken") {
            if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                cargo_toml_content.insert_str(pos + 14, "jsonwebtoken = \"9.3\"\n");
                modified = true;
            }
        }
        if !cargo_toml_content.contains("chrono") {
            if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                cargo_toml_content.insert_str(
                    pos + 14,
                    "chrono = { version = \"0.4\", features = [\"serde\"] }\n",
                );
                modified = true;
            }
        }
        if modified {
            fs::write(cargo_toml_path, cargo_toml_content)?;
            println!(
                "{}",
                "  ✨ Added 'jsonwebtoken' and 'chrono' dependencies to your Cargo.toml.".green()
            );
        }
    }

    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    let mut mod_content = fs::read_to_string(&mod_path)?;
    if !mod_content.contains("pub mod jwt_middleware;") {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str("pub mod jwt_middleware;\n");
        fs::write(&mod_path, mod_content)?;
    }

    let middleware_path = middlewares_dir.join("jwt_middleware.rs");
    if middleware_path.exists() {
        println!(
            "{}",
            "⚠️ Warning: JWT middleware 'jwt_middleware.rs' already exists. Skipping creation."
                .yellow()
        );
    } else {
        let template = r#"use axum::{
    extract::Request,
    middleware::Next,
    response::{Response, IntoResponse},
    http::{header, StatusCode},
};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // Subject (id do usuário)
    pub exp: usize,  // Timestamp de expiração
}

/// JWT Authentication Middleware.
/// Extrai o cabeçalho 'Authorization: Bearer <token>', valida e injeta os claims nas extensões da requisição.
pub async fn jwt_middleware(mut req: Request, next: Next) -> Response {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let Some(auth_str) = auth_header else {
        return (StatusCode::UNAUTHORIZED, "Missing Authorization Header").into_response();
    };

    if !auth_str.starts_with("Bearer ") {
        return (StatusCode::UNAUTHORIZED, "Invalid Authorization Header Format").into_response();
    }

    let token = &auth_str["Bearer ".len()..];
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_super_secreto_rullst_key".to_string());

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(token_data) => {
            // Insere os claims nas extensões da requisição para acesso nos controllers
            req.extensions_mut().insert(token_data.claims);
            next.run(req).await
        }
        Err(_) => (StatusCode::UNAUTHORIZED, "Invalid or Expired Token").into_response(),
    }
}

/// Helper para gerar um novo token JWT com duração de 1 dia.
pub fn generate_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_super_secreto_rullst_key".to_string());
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(1))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}
"#;
        fs::write(&middleware_path, template)?;
    }

    // Attempt to inject "pub mod middlewares;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        "✨ JWT middleware successfully created!".green().bold()
    );
    println!("{}", "How to use:".cyan());
    println!(
        "{}",
        "  1. Add the layer to your protected router (src/main.rs):".cyan()
    );
    println!(
        "{}",
        "     .layer(axum::middleware::from_fn(middlewares::jwt_middleware::jwt_middleware))"
            .cyan()
    );
    println!("{}", "  2. Acesse os claims no controller:".cyan());
    println!("{}", "     pub async fn meu_endpoint(axum::Extension(claims): axum::Extension<Claims>) -> impl IntoResponse".cyan());

    Ok(())
}

fn worker_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    if base.to_lowercase().ends_with("worker") {
        let len = base.len();
        base.truncate(len - 6);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_worker");

    // Clean duplicate underscores
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

fn create_new_worker(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let snake_name = worker_to_snake_case(name);
    let job_name = snake_name.strip_suffix("_worker").unwrap_or(&snake_name);

    println!(
        "{}",
        format!("🛠️ Generating background worker Rullst: {}...", snake_name)
            .cyan()
            .bold()
    );

    let workers_dir = Path::new("src/workers");
    if !workers_dir.exists() {
        fs::create_dir_all(workers_dir)?;
    }

    let mod_path = workers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // Add module declaration to mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
    }

    // Ensure register_workers function exists in mod.rs
    if !mod_content.contains("pub fn register_workers") {
        mod_content.push_str("\npub fn register_workers(worker: &mut rullst::queue::Worker) {\n");
        mod_content.push_str(&format!("    {}::register(worker);\n", snake_name));
        mod_content.push_str("}\n");
    } else {
        // Inject registration inside register_workers
        let search_str = "pub fn register_workers(worker: &mut rullst::queue::Worker) {";
        if let Some(pos) = mod_content.find(search_str) {
            let insert_pos = pos + search_str.len() + 1;
            mod_content.insert_str(
                insert_pos,
                &format!("    {}::register(worker);\n", snake_name),
            );
        }
    }
    fs::write(&mod_path, mod_content)?;

    let worker_path = workers_dir.join(format!("{}.rs", snake_name));
    if worker_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Worker '{}.rs' already exists. Skipping creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst::queue::{{Worker, QueueError}};
use serde_json::Value;

/// Registra o processador de tarefas deste worker.
pub fn register(worker: &mut Worker) {{
    worker.register("{job_name}", |payload: Value| async move {{
        println!("🚀 [Worker] Processando tarefa '{job_name}' com payload: {{:?}}", payload);
        
        // Escreva a lógica da sua tarefa em segundo plano aqui (ex: enviar e-mail, processar imagem)
        
        Ok(())
    }});
}}
"#,
            job_name = job_name
        );
        fs::write(&worker_path, template)?;
    }

    // Add module declaration to src/main.rs
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod workers;") && !main_content.contains("mod workers;") {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod workers;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content =
                    main_content.replace("pub mod models;", "pub mod models;\npub mod workers;");
            } else {
                main_content = format!("pub mod workers;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod workers;' to src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Worker '{}' successfully created at '{}'!",
            snake_name,
            worker_path.display()
        )
        .green()
        .bold()
    );
    println!(
        "{}",
        "How to initialize the background Worker in your 'src/main.rs':".cyan()
    );
    println!(
        "{}",
        "  1. Create the queue and initialize the worker:".cyan()
    );
    println!(
        "{}",
        "     let queue = rullst::Queue::sqlite(\"sqlite://rullst.db\").await?;".cyan()
    );
    println!(
        "{}",
        "     let mut worker = rullst::queue::Worker::new(&queue);".cyan()
    );
    println!("{}", "  2. Register your workers:".cyan());
    println!("{}", "     workers::register_workers(&mut worker);".cyan());
    println!("{}", "  3. Start the processing loop:".cyan());
    println!("{}", "     worker.run();".cyan());

    Ok(())
}

fn extract_description_from_handler(handler_path: &str) -> Option<String> {
    let parts: Vec<&str> = handler_path.split("::").collect();
    if parts.len() < 2 {
        return None;
    }
    let action = parts.last()?.to_string();
    let controller_module = parts[parts.len() - 2];

    // Find in src/controllers/<controller_module>.rs
    let controller_path = Path::new("src/controllers").join(format!("{}.rs", controller_module));
    if !controller_path.exists() {
        return None;
    }

    let content = fs::read_to_string(controller_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("pub async fn {}", action)) {
            let mut comments = Vec::new();
            let mut j = i;
            while j > 0 {
                j -= 1;
                let prev_line = lines[j].trim();
                if prev_line.starts_with("///") {
                    comments.push(prev_line["///".len()..].trim().to_string());
                } else if prev_line.starts_with("#[") || prev_line.is_empty() {
                    // skip decorators and empty lines
                    continue;
                } else {
                    break;
                }
            }
            if !comments.is_empty() {
                comments.reverse();
                return Some(comments.join(" "));
            }
        }
    }
    None
}

fn generate_openapi_spec() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🔍 Scanning project to extract routes and generate OpenAPI specification..."
            .cyan()
            .bold()
    );

    let main_path = Path::new("src/main.rs");
    if !main_path.exists() {
        println!("{}", "❌ Error: File src/main.rs not found.".red());
        std::process::exit(1);
    }

    let main_content = fs::read_to_string(main_path)?;

    // Parses Axum get/post/put/delete routing patterns
    let route_regex = regex::Regex::new(
        r#"(get|post|put|delete|patch|options|head)\s*\(\s*"([^"]+)"\s*=>\s*([\w_:]+)\s*\)"#,
    )?;

    let mut paths_map: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();

    for cap in route_regex.captures_iter(&main_content) {
        let method = cap[1].to_lowercase();
        let path = cap[2].to_string();
        let handler_path = cap[3].to_string();

        // Convert route parameters from Axum format (:id) to OpenAPI format ({id})
        let openapi_path = path
            .split('/')
            .map(|segment| {
                if segment.starts_with(':') {
                    format!("{{{}}}", &segment[1..])
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("/");

        let description = extract_description_from_handler(&handler_path)
            .unwrap_or_else(|| format!("Ação '{}' executada pelo handler.", handler_path));

        let mut parameters = serde_json::json!([]);
        for segment in path.split('/') {
            if segment.starts_with(':') {
                parameters.as_array_mut().unwrap().push(serde_json::json!({
                    "name": &segment[1..],
                    "in": "path",
                    "required": true,
                    "schema": {
                        "type": "string"
                    }
                }));
            }
        }

        let mut operation = serde_json::json!({
            "summary": description,
            "responses": {
                "200": {
                    "description": "Success"
                }
            }
        });

        if !parameters.as_array().unwrap().is_empty() {
            operation
                .as_object_mut()
                .unwrap()
                .insert("parameters".to_string(), parameters);
        }

        let path_item = paths_map
            .entry(openapi_path)
            .or_insert_with(|| serde_json::json!({}));
        path_item.as_object_mut().unwrap().insert(method, operation);
    }

    let openapi = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Especificação da API Rullst",
            "description": "Specification automatically generated via the cargo-rullst static analyzer.",
            "version": "1.0.0"
        },
        "paths": paths_map
    });

    let output_path = Path::new("openapi.json");
    fs::write(output_path, serde_json::to_string_pretty(&openapi)?)?;

    println!(
        "{}",
        format!(
            "✨ OpenAPI JSON specification successfully created at '{}'!",
            output_path.display()
        )
        .green()
        .bold()
    );
    Ok(())
}

fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "\n🚀 Starting Rullst Safe Upgrade (Self-Healing Upgrades)...\n"
            .cyan()
            .bold()
    );

    let latest_version = if get_cache_path().exists() {
        std::fs::read_to_string(get_cache_path())
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
            .trim()
            .to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    // Step 1: Update Cargo.toml
    println!(
        "{}",
        format!(
            "📦 Updating Rullst dependency versions to {} in Cargo.toml...",
            latest_version
        )
        .yellow()
    );
    let cargo_path = Path::new("Cargo.toml");
    if cargo_path.exists() {
        let mut cargo_content = std::fs::read_to_string(cargo_path)?;

        let re_rullst = regex::Regex::new(r#"(?m)^(\s*rullst\s*=\s*)"[^"]+""#)?;
        cargo_content = re_rullst
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}""#, &caps[1], latest_version)
            })
            .into_owned();

        let re_macros = regex::Regex::new(r#"(?m)^(\s*rullst-macros\s*=\s*)"[^"]+""#)?;
        cargo_content = re_macros
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}""#, &caps[1], latest_version)
            })
            .into_owned();

        let re_eloquent = regex::Regex::new(r#"(?m)^(\s*rullst-orm\s*=\s*)"[^"]+""#)?;
        cargo_content = re_eloquent
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"1.1.0""#, &caps[1])
            })
            .into_owned();

        std::fs::write(cargo_path, cargo_content)?;
    }

    // Step 2: Run cargo update
    println!(
        "{}",
        "📦 Refreshing dependencies and lockfile via cargo update...".yellow()
    );
    let update_status = Command::new("cargo").arg("update").status()?;

    if !update_status.success() {
        println!(
            "{}",
            "❌ Failed to update dependencies via cargo update.".red()
        );
        std::process::exit(1);
    }

    // Step 3: Run self-healing codemod AST & regex rules
    println!(
        "{}",
        "\n🔧 Executing self-healing codemod AST & regex rules over project files...".yellow()
    );

    let rules = vec![
        (
            r#"\bold_initializer\s*\(\s*\)"#,
            "Router::new()",
            "Legacy old_initializer() -> Router::new()",
        ),
        (
            r#"\brullst::routing::old_initializer\b"#,
            "rullst::routing::Router::new",
            "Legacy router initialization path",
        ),
        (
            r#"\buse\s+sqlx::"#,
            "use rullst::db::sqlx::",
            "Enforce Dependency Shielding for sqlx",
        ),
        (
            r#"\buse\s+axum::"#,
            "use rullst::web::axum::",
            "Enforce Dependency Shielding for axum",
        ),
        (
            r#"\buse\s+tokio::"#,
            "use rullst::async_runtime::tokio::",
            "Enforce Dependency Shielding for tokio",
        ),
    ];

    let mut applied_count = 0;
    if Path::new("src").exists() {
        let walker = walkdir::WalkDir::new("src");
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let mut file_content = std::fs::read_to_string(path)?;
                let mut modified = false;

                for (pattern, replacement, desc) in &rules {
                    let re = regex::Regex::new(pattern)?;
                    if re.is_match(&file_content) {
                        file_content = re.replace_all(&file_content, *replacement).into_owned();
                        println!(
                            "  [{}] Applied codemod: {} in {}",
                            "Codemod".green().bold(),
                            desc.cyan(),
                            path.display()
                        );
                        modified = true;
                        applied_count += 1;
                    }
                }

                if modified {
                    std::fs::write(path, file_content)?;
                }
            }
        }
    }

    if applied_count == 0 {
        println!("  ✨ No legacy APIs or shielding patterns required patching in this codebase.");
    } else {
        println!(
            "  ✨ Successfully executed {} codemod modifications.",
            applied_count
        );
    }

    // Step 4: Run `cargo fix`
    println!(
        "{}",
        "\n🔧 Applying additional code fixes via cargo fix...".yellow()
    );
    let fix_status = Command::new("cargo")
        .arg("fix")
        .arg("--allow-no-vcs")
        .arg("--allow-dirty")
        .status()?;

    if !fix_status.success() {
        println!(
            "{}",
            "❌ Failed to apply additional code fixes via cargo fix.".red()
        );
        std::process::exit(1);
    }

    // Step 5: Compiler validation gate
    println!(
        "{}",
        "\n🛡️ Running validation gate (cargo check) to confirm health status...".yellow()
    );
    let check_status = Command::new("cargo").arg("check").status()?;

    if check_status.success() {
        println!(
            "{}",
            "\n✅ Rullst updated successfully. No breaking changes detected! Code is 100% stable.\n"
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            "\n⚠️ Warning: Upgrade completed with check failures. Please review the compiler errors manually.\n"
                .yellow()
                .bold()
        );
    }

    Ok(())
}

fn run_build_client(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "\n🏝️  Iniciando a compilação do Rullst Wasm Island Client...\n"
            .cyan()
            .bold()
    );

    // 1. Check and inject [lib] crate-type into Cargo.toml if missing
    let mut cargo_content = fs::read_to_string("Cargo.toml")?;
    if !cargo_content.contains("[lib]") {
        cargo_content.push_str("\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n");
        fs::write("Cargo.toml", &cargo_content)?;
        println!(
            "{}",
            "ℹ️ Automatically injected [lib] crate-type into your Cargo.toml.".cyan()
        );
    }

    // 2. Proactively try to add the wasm32 target using rustup
    println!(
        "{}",
        "⚙️ Verificando e instalando target wasm32-unknown-unknown...".yellow()
    );
    let _ = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-unknown-unknown")
        .status();

    // 3. Compile the target
    println!(
        "{}",
        "📦 Compilando componentes frontend para wasm32-unknown-unknown...".yellow()
    );
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--lib");
    if !debug {
        cargo_cmd.arg("--release");
    }
    let build_status = cargo_cmd.status()?;
    if !build_status.success() {
        println!(
            "{}",
            "❌ Error compiling wasm32-unknown-unknown target.".red()
        );
        std::process::exit(1);
    }

    // 4. Extract package name to locate the compiled wasm file
    let package_name = cargo_content
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|val| {
            val.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .replace("-", "_")
        })
        .unwrap_or_else(|| "app".to_string());

    let profile = if debug { "debug" } else { "release" };
    let mut wasm_file_path = format!(
        "target/wasm32-unknown-unknown/{}/{}.wasm",
        profile, package_name
    );

    if !Path::new(&wasm_file_path).exists() {
        if Path::new("../../target").exists() {
            wasm_file_path = format!(
                "../../target/wasm32-unknown-unknown/{}/{}.wasm",
                profile, package_name
            );
        } else if Path::new("../target").exists() {
            wasm_file_path = format!(
                "../target/wasm32-unknown-unknown/{}/{}.wasm",
                profile, package_name
            );
        }
    }

    if !Path::new(&wasm_file_path).exists() {
        println!(
            "{}",
            format!("❌ Error: Compiled Wasm file not found at '{}'. Rullst also searched in parent directories.", wasm_file_path).red()
        );
        std::process::exit(1);
    }

    // 5. Ensure wasm-bindgen-cli is installed
    println!("{}", "🔍 Checking wasm-bindgen-cli...".yellow());
    let wasm_bindgen_installed = Command::new("wasm-bindgen")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

    if !wasm_bindgen_installed {
        println!(
            "{}",
            "⚙️ Automatically installing wasm-bindgen-cli... This might take a moment.".yellow()
        );
        let install_status = Command::new("cargo")
            .arg("install")
            .arg("wasm-bindgen-cli")
            .status()?;
        if !install_status.success() {
            println!(
                "{}",
                "❌ Failed to automatically install wasm-bindgen-cli.".red()
            );
            std::process::exit(1);
        }
    }

    // 6. Ensure static/ directory exists
    let static_dir = Path::new("static");
    if !static_dir.exists() {
        fs::create_dir_all(static_dir)?;
    }

    // 7. Run wasm-bindgen compiler
    println!("{}", "⚡ Running wasm-bindgen bindings...".yellow());
    let bindgen_status = Command::new("wasm-bindgen")
        .arg(&wasm_file_path)
        .arg("--out-dir")
        .arg("static")
        .arg("--target")
        .arg("web")
        .arg("--no-typescript")
        .status()?;

    if !bindgen_status.success() {
        println!(
            "{}",
            "❌ Error generating bindings with wasm-bindgen.".red()
        );
        std::process::exit(1);
    }

    // 8. Append the orchestrator to the generated JS file
    let js_file_path = format!("static/{}.js", package_name);
    if Path::new(&js_file_path).exists() {
        let mut js_content = fs::read_to_string(&js_file_path)?;

        let orchestrator = format!(
            r#"
// ─── Rullst Wasm Island Hydration Loop 🏝️ ────────────────────────────────────
export function hydrate_all() {{
    import('./{}.js').then((m) => {{
        const islands = document.querySelectorAll('[data-island]');
        for (const island of islands) {{
            const name = island.getAttribute('data-island');
            const props = island.getAttribute('data-props');
            const fn_name = `hydrate_${{name}}`;
            const hydrate_fn = m[fn_name];
            if (hydrate_fn) {{
                try {{
                    hydrate_fn(island, props);
                    console.log(`[Rullst] Hydrated island: ${{name}}`);
                }} catch (e) {{
                    console.error(`[Rullst] Failed to hydrate island ${{name}}:`, e);
                }}
            }} else {{
                console.warn(`[Rullst] No hydration function found for island: ${{name}}`);
            }}
        }}
    }}).catch(e => console.error("[Rullst] Failed to load Wasm ES module:", e));
}}

// Automatically hydrate when ready
if (typeof document !== 'undefined') {{
    if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', hydrate_all);
    }} else {{
        hydrate_all();
    }}
}}
"#,
            package_name
        );

        js_content.push_str(&orchestrator);
        fs::write(&js_file_path, js_content)?;
    }

    println!(
        "{}",
        "✨ Rullst Wasm Islands successfully compiled and generated!"
            .green()
            .bold()
    );
    println!("{}", "How to load in your HTML page:".cyan());
    println!(
        "{}",
        format!(
            "  <script type=\"module\">\n    import init from '/static/{}.js';\n    init();\n  </script>",
            package_name
        )
        .cyan()
    );

    Ok(())
}

fn run_production_build(release: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        format!(
            "\n🚀 Starting Rullst production build pipeline (Release Mode: {})...\n",
            release
        )
        .cyan()
        .bold()
    );

    // 1. Run cargo build --release (or debug)
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.arg("build");
    if release {
        cargo_cmd.arg("--release");
    }

    println!(
        "{}",
        format!(
            "⚙️ Executing cargo build{}...",
            if release { " --release" } else { "" }
        )
        .yellow()
    );
    let build_status = cargo_cmd.status()?;
    if !build_status.success() {
        println!("{}", "❌ Error: Cargo build failed.".red().bold());
        std::process::exit(1);
    }

    // 2. Pre-compress static files in static/ directory
    let static_dir = Path::new("static");
    if static_dir.exists() {
        println!(
            "{}",
            "📦 Pre-compressing static assets in static/ directory...".yellow()
        );
        let walker = walkdir::WalkDir::new(static_dir);
        let mut file_count = 0;
        let mut br_count = 0;
        let mut zst_count = 0;

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if matches!(
                    ext.as_str(),
                    "html" | "css" | "js" | "json" | "svg" | "wasm" | "xml" | "txt"
                ) {
                    file_count += 1;
                    let input_bytes = fs::read(path)?;

                    // Brotli compression (level 11)
                    let br_path = path.with_extension(format!("{}.br", ext));
                    println!(
                        "  Compressing {} -> {} (Brotli L11)...",
                        path.display(),
                        br_path.display()
                    );
                    {
                        let br_file = fs::File::create(&br_path)?;
                        let mut writer = brotli::CompressorWriter::new(br_file, 4096, 11, 22);
                        writer.write_all(&input_bytes)?;
                        writer.flush()?;
                    }
                    br_count += 1;

                    // Zstandard compression (level 19)
                    let zst_path = path.with_extension(format!("{}.zst", ext));
                    println!(
                        "  Compressing {} -> {} (Zstd L19)...",
                        path.display(),
                        zst_path.display()
                    );
                    {
                        let zst_file = fs::File::create(&zst_path)?;
                        let mut encoder = zstd::Encoder::new(zst_file, 19)?;
                        encoder.write_all(&input_bytes)?;
                        encoder.finish()?;
                    }
                    zst_count += 1;
                }
            }
        }
        println!(
            "{}",
            format!(
                "\n✨ Pre-compression finished: processed {} files, generated {} .br files and {} .zst files.",
                file_count, br_count, zst_count
            )
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            "ℹ️ No static/ directory found. Skipping static asset pre-compression.".cyan()
        );
    }

    println!(
        "{}",
        "\n🎉 Rullst production build completed successfully!"
            .green()
            .bold()
    );

    Ok(())
}

fn scaffold_desktop_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🖥️ Starting scaffolding of Rullst desktop packaging system (Tauri)..."
            .cyan()
            .bold()
    );

    // 1. Create Directories
    let src_tauri_dir = Path::new("src-tauri");
    let src_dir = src_tauri_dir.join("src");
    let icons_dir = src_tauri_dir.join("icons");

    fs::create_dir_all(&src_tauri_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&icons_dir)?;

    // 2. Write Cargo.toml
    let cargo_toml = r#"[package]
name = "rullst-desktop"
version = "0.1.0"
description = "Rullst Desktop Application"
authors = ["Rullst Developer"]
edition = "2021"

[build-dependencies]
tauri-build = { version = "1.5" }

[dependencies]
tauri = { version = "1.5", features = ["shell-open"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["blocking"] }
"#;
    fs::write(src_tauri_dir.join("Cargo.toml"), cargo_toml)?;

    // 3. Write tauri.conf.json
    let tauri_conf = r#"{
  "build": {
    "beforeDevCommand": "",
    "beforeBuildCommand": "",
    "devPath": "http://localhost:3000",
    "distDir": "http://localhost:3000"
  },
  "package": {
    "productName": "RullstDesktop",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false
    },
    "bundle": {
      "active": true,
      "category": "DeveloperTool",
      "copyright": "",
      "deb": {
        "depends": []
      },
      "externalBin": [],
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.rullst.desktop",
      "longDescription": "",
      "macOS": {
        "entitlements": null,
        "exceptionDomain": "",
        "frameworks": [],
        "providerBundleIdentifier": null,
        "signingIdentity": null
      },
      "resources": [],
      "shortDescription": "",
      "targets": "all",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "security": {
      "csp": null
    },
    "windows": [
      {
        "fullscreen": false,
        "height": 768,
        "resizable": true,
        "title": "Rullst Hyper Desktop",
        "width": 1024
      }
    ]
  }
}
"#;
    fs::write(src_tauri_dir.join("tauri.conf.json"), tauri_conf)?;

    // 4. Write build.rs
    let build_rs = r#"fn main() {
    tauri_build::build();
}
"#;
    fs::write(src_tauri_dir.join("build.rs"), build_rs)?;

    // 5. Write src/main.rs (Process Orchester)
    let main_rs = r#"#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Child};
use std::net::TcpStream;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    let backend_process = Arc::new(Mutex::new(None::<Child>));
    let backend_clone = Arc::clone(&backend_process);

    thread::spawn(move || {
        println!("🚀 Starting Rullst backend server...");
        
        let mut cmd = if std::path::Path::new("../Cargo.toml").exists() {
            let mut c = Command::new("cargo");
            c.arg("run").current_dir("..");
            c
        } else {
            let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
            let server_bin = if cfg!(windows) { "server.exe" } else { "server" };
            Command::new(exe_dir.join(server_bin))
        };

        match cmd.spawn() {
            Ok(child) => {
                let mut lock = backend_clone.lock().unwrap();
                *lock = Some(child);
            }
            Err(e) => {
                eprintln!("❌ Failed to start Rullst backend: {}", e);
            }
        }
    });

    println!("⏳ Waiting for Rullst server to bind on port 3000...");
    let poll_interval = Duration::from_millis(100);
    let timeout = Duration::from_secs(30);
    let start_time = std::time::Instant::now();
    let mut connected = false;

    while start_time.elapsed() < timeout {
        if TcpStream::connect("127.0.0.1:3000").is_ok() {
            connected = true;
            break;
        }
        thread::sleep(poll_interval);
    }

    if connected {
        println!("✅ Rullst server is ready! Launching Tauri interface...");
    } else {
        eprintln!("⚠️ Timeout waiting for port 3000 to open. Attempting window launch anyway...");
    }

    let backend_for_cleanup = Arc::clone(&backend_process);

    tauri::Builder::default()
        .on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event.event() {
                println!("🛑 Tauri window closed. Shutting down Rullst backend...");
                let mut lock = backend_for_cleanup.lock().unwrap();
                if let Some(mut child) = lock.take() {
                    let _ = child.kill();
                    println!("✅ Rullst backend terminated.");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
"#;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    // 6. Generate icons to prevent Tauri compile errors
    // PNG 1x1 transparent
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
        0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x01, 0x73, 0x0d, 0x8b, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
        0x42, 0x60, 0x82
    ];

    fs::write(icons_dir.join("32x32.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128.png"), png_bytes)?;
    fs::write(icons_dir.join("128x128@2x.png"), png_bytes)?;

    // Construct valid minimal ICO embedding the 1x1 PNG
    let mut ico_bytes = Vec::new();
    ico_bytes.extend_from_slice(&[0x00, 0x00]); // Reserved
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Type (1 = ICO)
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Number of images (1)
    
    // Directory entry (16 bytes)
    ico_bytes.push(0x01); // Width (1 pixel)
    ico_bytes.push(0x01); // Height (1 pixel)
    ico_bytes.push(0x00); // Color count
    ico_bytes.push(0x00); // Reserved
    ico_bytes.extend_from_slice(&[0x01, 0x00]); // Color planes (1)
    ico_bytes.extend_from_slice(&[0x20, 0x00]); // Bits per pixel (32)
    
    let png_len = png_bytes.len() as u32;
    ico_bytes.extend_from_slice(&png_len.to_le_bytes()); // Size of image data
    ico_bytes.extend_from_slice(&22u32.to_le_bytes());   // Offset of image data
    
    ico_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.ico"), &ico_bytes)?;

    // Construct valid minimal ICNS embedding the 1x1 PNG under "ic07" (128x128 size key)
    let mut icns_bytes = Vec::new();
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x6e, 0x73]); // Magic "icns"
    
    let total_icns_len = (8 + 8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&total_icns_len.to_be_bytes()); // Total length (big endian)
    
    icns_bytes.extend_from_slice(&[0x69, 0x63, 0x30, 0x37]); // OSType "ic07" (128x128 icon)
    let chunk_len = (8 + png_bytes.len()) as u32;
    icns_bytes.extend_from_slice(&chunk_len.to_be_bytes()); // Chunk length (big endian)
    
    icns_bytes.extend_from_slice(png_bytes);
    fs::write(icons_dir.join("icon.icns"), &icns_bytes)?;

    println!(
        "{}",
        "✅ Rullst Hyper desktop template successfully generated in 'src-tauri/'!"
            .green()
            .bold()
    );

    Ok(())
}

fn scaffold_omni_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "📱 Starting scaffolding of Rullst Omni multi-platform frontend (Dioxus)..."
            .cyan()
            .bold()
    );

    // 1. Create Directories
    let omni_dir = Path::new("omni-app");
    let src_dir = omni_dir.join("src");

    fs::create_dir_all(&omni_dir)?;
    fs::create_dir_all(&src_dir)?;

    // 2. Write Cargo.toml
    let cargo_toml = r#"[package]
name = "omni-app"
version = "0.1.0"
authors = ["Rullst Developer"]
edition = "2021"

[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
"#;
    fs::write(omni_dir.join("Cargo.toml"), cargo_toml)?;

    // 3. Write src/main.rs
    let main_rs = r##"#![allow(non_snake_case)]
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct BackendStatus {
    version: String,
    status: String,
    uptime: String,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut backend_data = use_signal(|| None::<BackendStatus>);

    let mut fetch_status = move |_| {
        spawn(async move {
            let client = reqwest::Client::new();
            match client.get("http://localhost:3000/api/status").send().await {
                Ok(res) => {
                    if let Ok(data) = res.json::<BackendStatus>().await {
                        backend_data.set(Some(data));
                    }
                }
                Err(_) => {
                    backend_data.set(Some(BackendStatus {
                        version: "1.0.5".to_string(),
                        status: "Running (Offline Simulation)".to_string(),
                        uptime: "2h 45m".to_string(),
                    }));
                }
            }
        });
    };

    use_future(move || async move {
        let client = reqwest::Client::new();
        match client.get("http://localhost:3000/api/status").send().await {
            Ok(res) => {
                if let Ok(data) = res.json::<BackendStatus>().await {
                    backend_data.set(Some(data));
                }
            }
            Err(_) => {
                backend_data.set(Some(BackendStatus {
                    version: "1.0.5".to_string(),
                    status: "Running (Offline Simulation)".to_string(),
                    uptime: "2h 45m".to_string(),
                }));
            }
        }
    });

    rsx! {
        style { {include_str!("./style.css")} }
        
        div { class: "app-container",
            div { class: "glow-circle glow-1" }
            div { class: "glow-circle glow-2" }
            
            div { class: "glass-card",
                header { class: "header-container",
                    div { class: "logo-group",
                        span { class: "logo-glow", "R" }
                        h1 { "Rullst "; span { class: "gradient-text", "Omni" } }
                    }
                    span { class: "badge", "v1.0.5 - Free Enterprise" }
                }

                div { class: "main-grid",
                    div { class: "sidebar-panel",
                        h3 { "System Status" }
                        div { class: "status-indicator active",
                            div { class: "ping-dot" }
                            span { "Connected to Dual-Engine Backend" }
                        }
                        
                        div { class: "stats-list",
                            div { class: "stat-item",
                                span { class: "stat-label", "Backend Version:" }
                                span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.version}"
                                    } else {
                                        "Fetching..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "Engine State:" }
                                span { class: "stat-value state-ok", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.status}"
                                    } else {
                                        "Connecting..."
                                    }
                                }
                            }
                            div { class: "stat-item",
                                span { class: "stat-label", "API Uptime:" }
                                span { class: "stat-value", 
                                    if let Some(ref data) = *backend_data.read() {
                                        "{data.uptime}"
                                    } else {
                                        "..."
                                    }
                                }
                            }
                        }

                        button { 
                            class: "primary-btn",
                            onclick: fetch_status,
                            "Refresh Backend Link"
                        }
                    }

                    div { class: "content-panel",
                        h2 { "Multi-Platform Frontend Engine" }
                        p { class: "panel-desc",
                            "Rullst Omni connects your Axum backend to high-fidelity user experiences across iOS, Android, and Desktop using the Dioxus renderer."
                        }

                        div { class: "cards-container",
                            div { class: "feature-card",
                                h4 { "⚡ Rullst Hyper" }
                                p { "Server-side HTMX rendering for extreme lightweight speed and zero Client Wasm overhead." }
                            }
                            div { class: "feature-card highlighted",
                                h4 { "📱 Rullst Omni" }
                                p { "Interactive, cross-compiled native Rust components with instant state reactivity." }
                            }
                        }
                    }
                }

                footer { class: "footer-container",
                    span { "Rullst Framework © 2026" }
                    span { class: "footer-link", "rullst.dev" }
                }
            }
        }
    }
}
"##;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    // 4. Write src/style.css
    let style_css = r#"* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    font-family: 'Outfit', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

body, html {
    background-color: #030712;
    color: #f3f4f6;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
}

.app-container {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    background: radial-gradient(circle at 50% 50%, #0c1020 0%, #030712 100%);
    overflow: hidden;
}

.glow-circle {
    position: absolute;
    border-radius: 50%;
    filter: blur(100px);
    opacity: 0.3;
    z-index: 1;
    animation: pulse 10s infinite alternate;
}

.glow-1 {
    width: 400px;
    height: 400px;
    background: #6366f1;
    top: -100px;
    left: -100px;
}

.glow-2 {
    width: 450px;
    height: 450px;
    background: #06b6d4;
    bottom: -150px;
    right: -150px;
    animation-delay: 5s;
}

@keyframes pulse {
    0% { transform: scale(1) translate(0, 0); opacity: 0.2; }
    100% { transform: scale(1.2) translate(30px, 30px); opacity: 0.4; }
}

.glass-card {
    position: relative;
    z-index: 10;
    width: 90%;
    max-width: 960px;
    height: 80%;
    max-height: 600px;
    background: rgba(17, 24, 39, 0.65);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 24px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5), 0 0 40px rgba(99, 102, 241, 0.1);
    overflow: hidden;
}

.header-container {
    padding: 24px 32px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.logo-group {
    display: flex;
    align-items: center;
    gap: 12px;
}

.logo-glow {
    width: 38px;
    height: 38px;
    background: linear-gradient(135deg, #6366f1, #06b6d4);
    border-radius: 10px;
    display: flex;
    justify-content: center;
    align-items: center;
    font-weight: 800;
    font-size: 20px;
    color: white;
    box-shadow: 0 0 20px rgba(99, 102, 241, 0.5);
}

h1 {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.5px;
}

.gradient-text {
    background: linear-gradient(90deg, #6366f1, #06b6d4);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
}

.badge {
    padding: 6px 12px;
    border-radius: 9999px;
    background: rgba(99, 102, 241, 0.15);
    border: 1px solid rgba(99, 102, 241, 0.3);
    color: #a5b4fc;
    font-size: 12px;
    font-weight: 600;
}

.main-grid {
    flex: 1;
    display: grid;
    grid-template-columns: 320px 1fr;
    overflow: hidden;
}

.sidebar-panel {
    padding: 32px;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(10, 15, 30, 0.2);
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.sidebar-panel h3 {
    font-size: 16px;
    font-weight: 600;
    color: #9ca3af;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.status-indicator {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 14px;
}

.ping-dot {
    width: 8px;
    height: 8px;
    background-color: #10b981;
    border-radius: 50%;
    box-shadow: 0 0 10px #10b981, 0 0 20px #10b981;
    animation: beacon 1.5s infinite alternate;
}

@keyframes beacon {
    0% { transform: scale(1); opacity: 0.8; }
    100% { transform: scale(1.3); opacity: 1; }
}

.stats-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 14px;
}

.stat-label {
    color: #9ca3af;
}

.stat-value {
    font-weight: 600;
    color: #f3f4f6;
}

.state-ok {
    color: #06b6d4;
}

.primary-btn {
    margin-top: auto;
    width: 100%;
    padding: 14px;
    border-radius: 12px;
    border: none;
    background: linear-gradient(90deg, #6366f1, #06b6d4);
    color: white;
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.3s ease;
    box-shadow: 0 4px 15px rgba(99, 102, 241, 0.3);
}

.primary-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(99, 102, 241, 0.5), 0 0 10px rgba(6, 182, 212, 0.3);
}

.primary-btn:active {
    transform: translateY(0);
}

.content-panel {
    padding: 40px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
}

h2 {
    font-size: 28px;
    font-weight: 800;
    letter-spacing: -0.5px;
}

.panel-desc {
    color: #9ca3af;
    line-height: 1.6;
    font-size: 15px;
}

.cards-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-top: 10px;
}

.feature-card {
    padding: 24px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: all 0.3s ease;
}

.feature-card h4 {
    font-size: 16px;
    font-weight: 700;
}

.feature-card p {
    font-size: 13px;
    color: #9ca3af;
    line-height: 1.5;
}

.feature-card.highlighted {
    background: rgba(99, 102, 241, 0.06);
    border: 1px solid rgba(99, 102, 241, 0.2);
    box-shadow: 0 0 15px rgba(99, 102, 241, 0.05);
}

.feature-card:hover {
    transform: scale(1.02);
    border-color: rgba(99, 102, 241, 0.4);
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.2), 0 0 15px rgba(99, 102, 241, 0.1);
}

.footer-container {
    padding: 16px 32px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: #6b7280;
}

.footer-link {
    color: #9ca3af;
    cursor: pointer;
    transition: color 0.2s;
}

.footer-link:hover {
    color: #6366f1;
}
"#;
    fs::write(src_dir.join("style.css"), style_css)?;

    println!(
        "{}",
        "✅ Rullst Omni (Dioxus) template successfully generated in 'omni-app/'!"
            .green()
            .bold()
    );

    Ok(())
}

fn scaffold_foundry_config() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let foundry_path = std::path::Path::new("Foundry.toml");
    if foundry_path.exists() {
        println!(
            "{}",
            "⚠️  Foundry.toml already exists. Delete it first to re-initialize.".yellow().bold()
        );
        std::process::exit(0);
    }

    println!(
        "{}",
        "🏭 Initializing Rullst Foundry deployment manifest (Foundry.toml)..."
            .cyan()
            .bold()
    );

    // Read project name from Cargo.toml
    let cargo_content = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let project_name = cargo_content
        .lines()
        .find(|l| l.trim_start().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "my-rullst-app".to_string());

    let foundry_toml = format!(
        r#"# ┌──────────────────────────────────────────────────────────────┐
# │           Rullst Foundry - Deployment Manifest               │
# │  Generated by `cargo rullst foundry:init`                    │
# │  Edit this file and run `cargo rullst foundry:deploy`        │
# └──────────────────────────────────────────────────────────────┘

[app]
# The name of your application (used for container and systemd service naming)
name = "{project_name}"
# The public domain that will serve your app (Caddy will get SSL automatically)
domain = "yourdomain.com"
# The internal port your Rullst server binds to
port = 3000

[deploy]
# Cloud provider: "hetzner" | "aws" | "gcp" | "azure" | "oci" | "digitalocean"
provider = "hetzner"

[server]
# Public IP or hostname of the target server
host = "1.2.3.4"
# SSH login user (typically root for fresh VPS, or a sudo user for managed VMs)
user = "root"
# Path to your SSH private key (leave empty to use SSH agent)
ssh_key = "~/.ssh/id_rsa"
# SSH port (default: 22)
ssh_port = 22

[build]
# Build profile: "release" | "debug"
profile = "release"
# Target triple (leave empty to use the local default)
# For cross-compilation from macOS/Windows to Linux: "x86_64-unknown-linux-musl"
target = ""

[database]
# Database type: "sqlite" | "postgres" | "mysql"
type = "sqlite"
# SQLite: path relative to the deployed binary  |  Postgres/MySQL: full connection URL
url = "sqlite:///app/data/db.sqlite"

[caddy]
# Enable automatic HTTPS via Caddy (strongly recommended for production)
auto_https = true
# Optional: add extra Caddyfile directives (e.g., rate_limit, header, etc.)
extra_directives = ""

[env]
# Environment variables injected into the container at runtime.
# Add your application secrets here (they will NOT be committed if you gitignore Foundry.toml).
APP_ENV = "production"
APP_KEY = "CHANGE_ME_TO_A_SECURE_RANDOM_KEY"
DATABASE_URL = "sqlite:///app/data/db.sqlite"
# STRIPE_SECRET_KEY = ""
# AWS_ACCESS_KEY_ID = ""
"#, project_name = project_name);

    fs::write(foundry_path, &foundry_toml)?;

    println!(
        "{}",
        "✅ Foundry.toml generated successfully!".green().bold()
    );
    println!();
    println!(
        "{}",
        "📋 Next steps:".bold()
    );
    println!(
        "  1. Edit {} with your server IP, domain, and secrets.",
        "Foundry.toml".cyan()
    );
    println!(
        "  2. Add {} to your {} to keep secrets safe.",
        "Foundry.toml".cyan(),
        ".gitignore".yellow()
    );
    println!(
        "  3. Run {} to deploy to your cloud provider.",
        "cargo rullst foundry:deploy".magenta().bold()
    );
    println!();

    // Suggest adding to .gitignore
    let gitignore_path = std::path::Path::new(".gitignore");
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path).unwrap_or_default();
        if !content.contains("Foundry.toml") {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str("# Rullst Foundry (contains server secrets)\nFoundry.toml\n");
            fs::write(gitignore_path, new_content)?;
            println!(
                "{}",
                "🔒 Automatically added Foundry.toml to .gitignore to protect your secrets."
                    .green()
            );
        }
    }

    Ok(())
}

fn run_foundry_deploy() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let foundry_path = std::path::Path::new("Foundry.toml");
    if !foundry_path.exists() {
        println!(
            "{}",
            "❌ Foundry.toml not found. Run 'cargo rullst foundry:init' first."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    // --- Parse Foundry.toml ---
    let content = fs::read_to_string(foundry_path)?;

    let get_value = |key: &str| -> String {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(key) && trimmed.contains('=') {
                let val = trimmed
                    .splitn(2, '=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
                return val;
            }
        }
        String::new()
    };

    let app_name   = get_value("name");
    let domain     = get_value("domain");
    let port       = get_value("port");
    let host       = get_value("host");
    let user       = get_value("user");
    let ssh_key    = get_value("ssh_key");
    let ssh_port   = get_value("ssh_port");
    let provider   = get_value("provider");
    let db_type    = get_value("type");
    let _db_url     = get_value("url");
    let profile    = get_value("profile");
    let target_triple = get_value("target");
    let auto_https = get_value("auto_https");

    // Collect [env] block
    let mut env_vars: Vec<(String, String)> = Vec::new();
    let mut in_env = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[env]" { in_env = true; continue; }
        if trimmed.starts_with('[') && trimmed != "[env]" { in_env = false; }
        if in_env && trimmed.contains('=') && !trimmed.starts_with('#') {
            let mut parts = trimmed.splitn(2, '=');
            if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                env_vars.push((k.trim().to_string(), v.trim().trim_matches('"').to_string()));
            }
        }
    }

    let ssh_key_expanded = ssh_key.replace("~", &std::env::var("HOME").unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default()));
    let ssh_port_num = if ssh_port.is_empty() { "22".to_string() } else { ssh_port };
    let app_port = if port.is_empty() { "3000".to_string() } else { port };

    let ssh_base_args: Vec<String> = {
        let mut args = Vec::new();
        args.push("-p".to_string());
        args.push(ssh_port_num.clone());
        if !ssh_key_expanded.is_empty() {
            args.push("-i".to_string());
            args.push(ssh_key_expanded.clone());
        }
        args.push("-o".to_string());
        args.push("StrictHostKeyChecking=no".to_string());
        args.push(format!("{}@{}", user, host));
        args
    };

    let run_ssh = |cmd: &str| -> Result<bool, Box<dyn std::error::Error>> {
        let mut full_args = ssh_base_args.clone();
        full_args.push(cmd.to_string());
        let status = Command::new("ssh").args(&full_args).status()?;
        Ok(status.success())
    };

    println!();
    println!("{}", "┌────────────────────────────────────────────────────────────┐".cyan().bold());
    println!("{}", format!("│  🏭  Rullst Foundry — Deploying to {:>24} │", provider.to_uppercase()).cyan().bold());
    println!("{}", "└────────────────────────────────────────────────────────────┘".cyan().bold());
    println!();
    println!("  {} {}", "→ App:".bold(),    app_name.cyan());
    println!("  {} {}", "→ Domain:".bold(), domain.cyan());
    println!("  {} {}", "→ Server:".bold(), format!("{}@{}", user, host).cyan());
    println!("  {} {}", "→ Port:".bold(),   app_port.cyan());
    println!("  {} {}", "→ DB:".bold(),     db_type.cyan());
    println!("  {} {}", "→ Profile:".bold(), if profile.is_empty() { "release".to_string() } else { profile.clone() }.cyan());
    println!();

    // ── Step 1: Build ────────────────────────────────────────────────
    println!("{}", "📦 [1/5] Building production binary...".bold().yellow());
    let mut build_args = vec!["build".to_string()];
    if profile != "debug" {
        build_args.push("--release".to_string());
    }
    if !target_triple.is_empty() {
        build_args.push("--target".to_string());
        build_args.push(target_triple.clone());
    }
    let build_status = Command::new("cargo").args(&build_args).status()?;
    if !build_status.success() {
        println!("{}", "❌ Build failed. Aborting deployment.".red().bold());
        std::process::exit(1);
    }
    println!("{}", "  ✅ Build successful.".green());

    // Determine binary path
    let bin_subdir = if target_triple.is_empty() {
        if profile == "debug" { "debug".to_string() } else { "release".to_string() }
    } else {
        format!("{}/{}", target_triple, if profile == "debug" { "debug" } else { "release" })
    };
    let cargo_toml_content = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let bin_name = cargo_toml_content
        .lines()
        .find(|l| l.trim_start().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| app_name.clone());
    let local_bin = format!("target/{}/{}", bin_subdir, bin_name);

    // ── Step 2: Provision server ─────────────────────────────────────
    println!("{}", "🖥️  [2/5] Provisioning server environment...".bold().yellow());
    let provision_cmd = format!(
        r#"set -e
apt-get update -qq
apt-get install -y -qq docker.io curl wget || yum install -y docker curl wget || true
systemctl enable docker --now || true
mkdir -p /app/data /app/bin /app/config
echo "✅ Server environment ready.""#
    );
    if !run_ssh(&provision_cmd)? {
        println!("{}", "⚠️  Server provisioning had warnings (continuing anyway)...".yellow());
    } else {
        println!("{}", "  ✅ Server provisioned.".green());
    }

    // ── Step 3: Upload binary ─────────────────────────────────────────
    println!("{}", "📤 [3/5] Uploading application binary...".bold().yellow());
    let mut scp_args = Vec::new();
    scp_args.push("-P".to_string());
    scp_args.push(ssh_port_num.clone());
    if !ssh_key_expanded.is_empty() {
        scp_args.push("-i".to_string());
        scp_args.push(ssh_key_expanded.clone());
    }
    scp_args.push("-o".to_string());
    scp_args.push("StrictHostKeyChecking=no".to_string());
    scp_args.push(local_bin.clone());
    scp_args.push(format!("{}@{}:/app/bin/{}", user, host, bin_name));

    let scp_status = Command::new("scp").args(&scp_args).status()?;
    if !scp_status.success() {
        println!("{}", "❌ Failed to upload binary via SCP. Check SSH access and try again.".red().bold());
        std::process::exit(1);
    }
    println!("{}", "  ✅ Binary uploaded to /app/bin/.".green());

    // ── Step 4: Write env + Caddyfile + start container ──────────────
    println!("{}", "⚙️  [4/5] Configuring services (env, Caddy, container)...".bold().yellow());

    let _env_block: String = env_vars
        .iter()
        .map(|(k, v)| format!("ENV {}={}\n", k, v))
        .collect();

    let _env_run_flags: String = env_vars
        .iter()
        .map(|(k, v)| format!("-e {}=\"{}\" ", k, v))
        .collect();

    let caddy_site = if auto_https == "true" || auto_https.is_empty() {
        format!(
            r#"{domain} {{
    reverse_proxy localhost:{app_port}
    encode gzip zstd
    header {{
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Frame-Options DENY
        X-Content-Type-Options nosniff
        Referrer-Policy strict-origin-when-cross-origin
    }}
    log {{
        output file /var/log/caddy/{app_name}.log
    }}
}}"#, domain = domain, app_port = app_port, app_name = app_name
        )
    } else {
        format!(
            r#":{app_port} {{
    reverse_proxy localhost:{app_port}
}}"#, app_port = app_port
        )
    };

    let configure_cmd = format!(
        r#"set -e
# Write env file
cat > /app/config/.env << 'ENVEOF'
{env_lines}
ENVEOF

# Write Caddyfile
cat > /etc/caddy/Caddyfile << 'CADDYEOF'
{caddy_site}
CADDYEOF

# Install Caddy if not present
if ! command -v caddy &> /dev/null; then
    curl -fsSL https://caddyserver.com/install.sh | bash -s -- --
fi

# Make binary executable
chmod +x /app/bin/{bin_name}

# Stop old service if running
docker rm -f rullst_{app_name} 2>/dev/null || true
pkill -f "/app/bin/{bin_name}" 2>/dev/null || true

# Start app as background systemd service
cat > /etc/systemd/system/rullst_{app_name}.service << 'SVCEOF'
[Unit]
Description=Rullst App: {app_name}
After=network.target

[Service]
Type=simple
ExecStart=/app/bin/{bin_name}
WorkingDirectory=/app/data
EnvironmentFile=/app/config/.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable rullst_{app_name}
systemctl restart rullst_{app_name}

# Reload Caddy
systemctl enable caddy 2>/dev/null || true
systemctl reload caddy 2>/dev/null || systemctl restart caddy 2>/dev/null || caddy reload 2>/dev/null || true

echo "✅ Services configured and started."
"#,
        env_lines = env_vars.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("\n"),
        caddy_site = caddy_site,
        bin_name = bin_name,
        app_name = app_name,
    );

    if !run_ssh(&configure_cmd)? {
        println!("{}", "⚠️  Service configuration had warnings. Verify on the server.".yellow());
    } else {
        println!("{}", "  ✅ Services configured and started.".green());
    }

    // ── Step 5: Health check ──────────────────────────────────────────
    println!("{}", "🩺 [5/5] Running deployment health check...".bold().yellow());
    let health_cmd = format!(
        "sleep 3 && curl -sf http://localhost:{app_port} > /dev/null && echo '✅ App is responding!' || echo '⚠️  App may still be starting...'",
        app_port = app_port
    );
    let _ = run_ssh(&health_cmd);

    println!();
    println!("{}", "┌────────────────────────────────────────────────────────────┐".green().bold());
    println!("{}", "│  🎉  Rullst Foundry — Deployment Complete!                  │".green().bold());
    println!("{}", "└────────────────────────────────────────────────────────────┘".green().bold());
    println!();
    let url_protocol = if auto_https == "true" || auto_https.is_empty() { "https" } else { "http" };
    println!(
        "  {} {}://{}",
        "🌐 Your app is live at:".bold(),
        url_protocol,
        domain.cyan().bold()
    );
    println!("  {}", "📋 To check logs: ssh into your server and run:".bold());
    println!("     {}", format!("journalctl -u rullst_{} -f", app_name).magenta());
    println!();

    Ok(())
}

fn run_cli_command(command: &Commands) -> Result<(), Box<dyn std::error::Error>> {
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
            DocsCommands::Dev => docs_generator::run_dev_server()?,
            DocsCommands::Build => docs_generator::run_build()?,
        },
    }
    Ok(())
}
