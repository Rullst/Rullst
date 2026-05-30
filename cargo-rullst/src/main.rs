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
            r#"use rullst_orm::{{Orm, EloquentModel, sqlx::{{self, FromRow}}}};

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
[workspace]
"#,
    );

    fs::write(path.join("Cargo.toml"), cargo_toml)?;

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
        r#"use rullst_orm::{Orm, EloquentModel, sqlx::{self, FromRow}};

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
    let model_template = r##"use rullst_orm::{Orm, EloquentModel, sqlx::{self, FromRow}};

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
    let passkey_model_template = r##"use rullst_orm::{Orm, EloquentModel, sqlx::{self, FromRow}};

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
