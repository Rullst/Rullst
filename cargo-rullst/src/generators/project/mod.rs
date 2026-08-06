// cargo-rullst/src/generators/project/mod.rs — Root of project generator module (< 200 lines).

pub mod cargo_toml;
pub mod env_config;
pub mod wizard;

use colored::*;
use std::fs;
use std::path::Path;

pub use env_config::{generate_buildah_script, generate_nix_files};
pub use wizard::{run_project_wizard, ProjectWizardOptions};

pub fn has_binary(name: &str) -> bool {
    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return false;
    }
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn generate_secure_app_key() -> String {
    use rand::RngExt;
    let mut key = String::new();
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    for _ in 0..32 {
        let idx = rng.random_range(0..chars.len());
        key.push(chars[idx] as char);
    }
    key
}

pub fn create_new_project(
    name_arg: Option<&str>,
    api_arg: bool,
    docker: bool,
    nix: bool,
    buildah: bool,
    use_defaults: bool,
    turso: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let wizard_opts = run_project_wizard(name_arg, api_arg, use_defaults, turso)?;

    let name = wizard_opts.name.clone();
    let project_name = name.clone();
    let project_name_safe = name.replace('-', "_");
    let api = wizard_opts.api;
    let mut db_needed = wizard_opts.db_needed;
    let db_provider = wizard_opts.db_provider.clone();
    let hot_reload = wizard_opts.hot_reload;
    let blueprint_selection = wizard_opts.blueprint_selection;
    let wants_ai = wizard_opts.wants_ai;
    let wants_redis = wizard_opts.wants_redis;

    if blueprint_selection != 0 {
        db_needed = true;
    }

    let path = Path::new(&name);
    if path.exists() {
        println!(
            "{}",
            format!("❌ Directory '{}' already exists.", name).red()
        );
        return Ok(());
    }

    fs::create_dir_all(path)?;
    let current_dir = std::env::current_dir()?;

    let cargo_toml_content = cargo_toml::build_cargo_toml(
        &project_name_safe,
        hot_reload,
        db_needed,
        &db_provider,
        wants_ai,
        wants_redis,
        blueprint_selection,
        &wizard_opts.frontend_engine,
        &current_dir,
    )?;
    fs::write(path.join("Cargo.toml"), cargo_toml_content)?;

    let app_key = generate_secure_app_key();
    env_config::generate_env_and_configs(
        path,
        db_needed,
        &db_provider,
        turso,
        blueprint_selection,
        &app_key,
    )?;

    // Apply Blueprint templates
    crate::blueprints::apply(
        blueprint_selection,
        path,
        &project_name,
        &project_name_safe,
        api,
        hot_reload,
        db_needed,
        &wizard_opts.orm_pattern,
        &wizard_opts.frontend_engine,
    )?;

    if docker {
        generate_docker_files(path, &project_name, Some(&db_provider), Some(wants_redis))?;
    }

    if nix {
        env_config::generate_nix_files(path)?;
    }

    if db_needed {
        println!("\n{}", "📦 Bootstrapping Database...".cyan().bold());
        let migrate_success = crate::ui::components::with_spinner(
            "Running initial migrations (this may take a moment to compile)...",
            || {
                std::process::Command::new("cargo")
                    .arg("run")
                    .arg("-q")
                    .arg("--")
                    .arg("db:migrate")
                    .current_dir(path)
                    .output()
                    .map(|s| s.status.success())
                    .unwrap_or(false)
            },
        );

        if migrate_success {
            println!("{}", "  ✅ Database tables created successfully.".green());
        } else {
            println!("{}", "  ⚠️ Warning: Failed to run initial database migrations.".yellow());
        }
    }

    if buildah {
        env_config::generate_buildah_script(path, &project_name).ok();
    }

    println!(
        "{}",
        format!("✨ Project '{}' created successfully!", name)
            .green()
            .bold()
    );
    println!("{}", "How to run:".magenta());
    println!("{}", format!("  cd {}", name).cyan());
    println!("{}", "  Then, choose your experience:".white().dimmed());
    println!("{}", "    cargo rullst dash  (interactive dashboard)".white().bold());
    println!("{}", "    cargo rullst dev   (standard output)".white());

    Ok(())
}

pub fn generate_docker_files(
    project_path: &Path,
    project_name: &str,
    db_provider_arg: Option<&str>,
    redis_arg: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🐳 Generating Docker files...".cyan().bold());
    let _db_provider = db_provider_arg.unwrap_or("Sqlite");
    let _wants_redis = redis_arg.unwrap_or(false);

    let dockerfile = format!(
        r#"FROM rust:1.97-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM docker.io/library/debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/{project_name} /app/{project_name}
CMD ["/app/{project_name}"]
"#
    );
    fs::write(project_path.join("Dockerfile"), dockerfile)?;
    println!("{}", "  ✅ Dockerfile generated.".green());
    Ok(())
}
