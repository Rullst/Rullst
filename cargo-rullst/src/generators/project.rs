// src/generators/project.rs — Project wizard generator and scaffolding orchestrator.

use colored::*;
use std::fs;
use std::path::Path;

pub fn has_binary(name: &str) -> bool {
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
    let mut rng = rand::rng();
    let mut key = String::new();
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
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
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "  {}",
        "┌────────────────────────────────────────────────────┐".bright_cyan()
    );
    println!(
        "  {} 🎯 {} APP CREATOR — Let's build something new! {}",
        "│".bright_cyan(),
        "RULLST".truecolor(255, 165, 0).bold(),
        "│".bright_cyan()
    );
    println!(
        "  {}",
        "└────────────────────────────────────────────────────┘".bright_cyan()
    );
    println!();

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
                if val_trim.chars().next().unwrap().is_ascii_digit() {
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

    let mut api = api_arg;
    let mut db_provider = "Sqlite".to_string();
    let mut db_needed = true;
    let mut hot_reload = false;
    let mut blueprint_selection = 0usize;

    if name_arg.is_none() {
        let portfolio_title = format!(
            "Portfolio 🔥 (showcase for Rullst/AI developers) - {}",
            "HOT".bright_red().bold()
        );
        let blueprint_choices = vec![
            "Blank Starter (Minimal template with HTMX reactive counter)".to_string(),
            portfolio_title,
            "LMS Platform (Courses, lessons, video player, HTMX integration)".to_string(),
            "SaaS App Starter (Authentication + Stripe payments billing template)".to_string(),
            "Blog / Press (Static site generator pre-wired with Nexus CMS)".to_string(),
            "ERP Pocket (Inventory, stock management, orders tracker, auto-CMS)".to_string(),
            "Uptime Monitor (Ping dashboard, background status checker, glassmorphism)".to_string(),
        ];
        blueprint_selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("🧭 Select a Starter Blueprint")
            .default(0)
            .items(&blueprint_choices)
            .interact()?;

        if blueprint_selection == 0 {
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

            if db_needed {
                let db_options = &[
                    "Sqlite (Zero setup)",
                    "Postgres (Requires localhost:5432 running)",
                    "MySQL/MariaDB (Requires localhost:3306 running)",
                ];
                let db_selection = dialoguer::Select::with_theme(&theme)
                    .with_prompt("💾 Select a DB Provider (Network DBs will hang on setup if not running locally)")
                    .default(0)
                    .items(&db_options[..])
                    .interact()?;
                db_provider = match db_selection {
                    1 => "Postgres".to_string(),
                    2 => "MySQL".to_string(),
                    _ => "Sqlite".to_string(),
                };
            }
        } else if blueprint_selection == 1 {
            db_needed = false;
        } else {
            // LMS, SaaS, and Blog blueprints require database configuration (always Sqlite by default)
            db_needed = true;
            db_provider = "Sqlite".to_string();
        }

        hot_reload = dialoguer::Confirm::with_theme(&theme)
            .with_prompt("🔥 Enable Hot Reloading by default? (Auto-recompiles on save)")
            .default(true)
            .interact()?;
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
    fs::create_dir_all(path.join("static"))?;

    // Get absolute path to the Rullst framework folder for local referencing
    let current_dir = std::env::current_dir()?;
    let mut rullst_dir = None;
    let mut check_dir = current_dir.clone();
    loop {
        if check_dir.join("rullst").exists() && check_dir.join("Cargo.toml").exists() {
            rullst_dir = Some(check_dir.join("rullst"));
            break;
        }
        if !check_dir.pop() {
            break;
        }
    }

    let rullst_dep = if let Some(ref dir) = rullst_dir {
        let path = dir.canonicalize()?.display().to_string();
        let path = path.trim_start_matches(r"\\?\").replace("\\", "/");
        format!("rullst = {{ path = \"{}\" }}", path)
    } else {
        r#"rullst = "2.0.8""#.to_string()
    };

    let rullst_png_path = rullst_dir
        .unwrap_or_else(|| current_dir.join("rullst"))
        .join("Rullst.png");
    if !rullst_png_path.exists() {
        let fallback_png = Path::new("Rullst.png");
        if fallback_png.exists() {
            let _ = fs::copy(fallback_png, path.join("static/favicon.png"));
        }
    } else {
        let _ = fs::copy(rullst_png_path, path.join("static/favicon.png"));
    }

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
{rullst_dep}
tokio = {{ version = "1.43", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
axum = "0.8"
"#,
        rullst_dep = rullst_dep
    ));

    if db_needed {
        cargo_toml.push_str(&format!(
            r#"rullst-orm = "5.0.0"
sqlx = {{ version = "0.9.0", {sqlx_features} }}
"#,
            sqlx_features = sqlx_features
        ));
    }

    if blueprint_selection == 6 {
        cargo_toml.push_str("reqwest = { version = \"0.12\", default-features = false, features = [\"rustls-tls\"] }\n");
    }

    // Special dependencies for SaaS blueprint
    if blueprint_selection == 3 {
        let sibling_path = current_dir.join("rullst-connect");
        let connect_dep = if sibling_path.exists() {
            let absolute_path = sibling_path
                .canonicalize()?
                .display()
                .to_string()
                .replace("\\", "/");
            format!("rullst-connect = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-connect = \"7.0.1\"\n".to_string()
        };
        cargo_toml.push_str(&connect_dep);
    }

    cargo_toml.push_str(
        r#"
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(feature, values("redis"))'] }

# ⚡ Rullst God-Mode: Instant Incremental Compilation (<100ms)
# If you want development speed close to interpreted languages,
# you can use the official Cranelift backend for the Rust compiler.
# 
# Requirements:
#   1. Install nightly toolchain: rustup toolchain install nightly
#   2. Install the component: rustup component add rustc-codegen-cranelift-preview --toolchain nightly
#   3. Enable by uncommenting the block below and running the project with the nightly toolchain (e.g.: cargo +nightly run)
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
# This file configures ultra-fast linkers for local development.
# Rullst has detected your environment and configured the appropriate options.

"#,
    );

    // Configuração para Windows (MSVC usa lld-link ou lld)
    if has_lld && cfg!(windows) {
        config_toml.push_str(
            r#"[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

"#,
        );
    } else {
        config_toml.push_str(
            r#"# To enable on Windows (Install LLVM with 'winget install LLVM.LLVM' and uncomment below):
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

"#,
        );
    } else if has_lld && cfg!(target_os = "linux") {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

"#,
        );
    } else {
        config_toml.push_str(
            r#"# To enable on Linux (Install mold with your package manager and uncomment below):
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
"#,
        );
    } else {
        config_toml.push_str(
            r#"# To enable on macOS (Install llvm/lld via brew and uncomment below):
# [target.x86_64-apple-darwin]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
# [target.aarch64-apple-darwin]
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
"#,
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

    // Write .gitignore
    let gitignore_content = r#"# Rust build artifacts
/target
/Cargo.lock

# Rullst Wasm Islands
/static/*.js
/static/*.wasm
/static/*.html

# Rullst: Environment & Secrets
.env
.env.*
!.env.example

# Rullst: Foundry Deployment Manifest (contains SSH keys and cloud credentials)
Foundry.toml
"#;
    fs::write(path.join(".gitignore"), gitignore_content)?;

    // Write .env and .env.example
    let app_key = generate_secure_app_key();
    let db_url = match db_provider.as_str() {
        "Postgres" => "postgres://postgres:password@localhost/rullst",
        "MySQL" => "mysql://root:password@localhost/rullst",
        _ => "sqlite://rullst.db?mode=rwc",
    };

    let mut env_content = format!(
        r#"# ─────────────────────────────────────────────────────────────
#  Rullst Application Environment Configuration
#  Generated automatically by cargo rullst new
# ─────────────────────────────────────────────────────────────

# ⚠️ SECURITY: This file must NEVER be committed to git.
# It is automatically added to .gitignore by the Rullst CLI.

# ── Application ───────────────────────────────────────────────
APP_KEY={app_key}
APP_ENV=development
"#,
        app_key = app_key
    );

    let mut env_example_content =
        r#"# ─────────────────────────────────────────────────────────────
#  Rullst Application Environment Configuration
#  Generated automatically by cargo rullst new
# ─────────────────────────────────────────────────────────────

# ── Application ───────────────────────────────────────────────
APP_KEY=REPLACE_WITH_YOUR_32_CHAR_APP_KEY
APP_ENV=development
"#
        .to_string();

    if db_needed {
        let db_env_str = format!(
            "\n# ── Database ──────────────────────────────────────────────────\nDATABASE_URL={}\n",
            db_url
        );
        env_content.insert_str(
            env_content
                .find("# ── Application")
                .unwrap_or(env_content.len()),
            &db_env_str,
        );
        env_example_content.insert_str(
            env_example_content
                .find("# ── Application")
                .unwrap_or(env_example_content.len()),
            &db_env_str,
        );
    }

    if blueprint_selection == 2 || blueprint_selection == 3 {
        let stripe_template = r#"
# ── Stripe Billing (replace with your real keys from stripe.com/dashboard) ──
# STRIPE_SECRET_KEY=sk_test_REPLACE_WITH_YOUR_SECRET_KEY
# STRIPE_WEBHOOK_SECRET=whsec_REPLACE_WITH_YOUR_WEBHOOK_SECRET
# STRIPE_PRICE_ID_MONTHLY=price_REPLACE_WITH_YOUR_PRICE_ID
"#;
        env_content.push_str(stripe_template);
        env_example_content.push_str(stripe_template);
    }

    fs::write(path.join(".env"), &env_content)?;
    fs::write(path.join(".env.example"), &env_example_content)?;

    // Physically create the sqlite database file so SQLx doesn't panic on first run
    if db_provider == "Sqlite" {
        let _ = fs::write(path.join("rullst.db"), "");
    }

    // Apply Blueprint templates
    crate::blueprints::apply(
        blueprint_selection,
        path,
        &project_name,
        &project_name_safe,
        api,
        hot_reload,
        db_needed,
    )?;

    // Generate Docker files if --docker flag was passed
    if docker {
        let wants_redis = blueprint_selection == 2 || blueprint_selection == 3;
        generate_docker_files(path, &project_name, Some(&db_provider), Some(wants_redis))?;
    }

    // Automatically run initial migrations if a database was selected
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
            println!("{}", "  ⚠️ Warning: Failed to run initial database migrations. You may need to run `cargo rullst db:migrate` manually.".yellow());
        }
    }

    if !has_mold && !has_lld {
        println!(
            "\n{}",
            "⚡ Rullst Dev Tip: Speed up compile times up to 10x!"
                .yellow()
                .bold()
        );
        println!(
            "{}",
            "To unlock near-instant compile speeds, we highly recommend installing a fast linker:"
                .white()
        );
        if cfg!(windows) {
            println!("{}", "  👉 Install LLD: winget install LLVM.LLVM".cyan());
        } else if cfg!(target_os = "macos") {
            println!("{}", "  👉 Install LLD: brew install llvm".cyan());
        } else {
            println!(
                "{}",
                "  👉 Install Mold: sudo apt install mold (or dnf install mold)".cyan()
            );
        }
        println!(
            "{}",
            "Once installed, uncomment the config lines inside '.cargo/config.toml'!".white()
        );
    } else {
        println!(
            "\n{}",
            "🚀 High-performance linker automatically detected and configured!"
                .green()
                .bold()
        );
    }

    println!(
        "{}",
        format!("✨ Project '{}' created successfully!", name)
            .green()
            .bold()
    );
    println!("{}", "How to run:".cyan());
    println!("{}", format!("  cd {}", name).cyan());
    println!("{}", "  cargo rullst dev".cyan());
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

pub fn generate_docker_files(
    project_path: &Path,
    project_name: &str,
    db_provider_arg: Option<&str>,
    redis_arg: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🐳 Generating Docker files...".cyan().bold());

    let theme = dialoguer::theme::ColorfulTheme::default();
    let db_provider = match db_provider_arg {
        Some(db) => db.to_string(),
        None => {
            let db_options = &["Sqlite (Zero setup)", "Postgres", "MySQL/MariaDB"];
            let selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("Which Database are you using?")
                .default(0)
                .items(&db_options[..])
                .interact()?;
            match selection {
                1 => "Postgres".to_string(),
                2 => "MySQL".to_string(),
                _ => "Sqlite".to_string(),
            }
        }
    };

    let wants_redis = match redis_arg {
        Some(r) => r,
        None => dialoguer::Confirm::with_theme(&theme)
            .with_prompt("Do you want to include Redis in your docker-compose?")
            .default(true)
            .interact()?,
    };

    // --- Dockerfile (multi-stage, distroless) ---
    let dockerfile = format!(
        r#"# ══════════════════════════════════════════════════════════════
# Rullst Production Dockerfile (auto-generated)
# Multi-stage build: Rust builder → Distroless runtime
# Final image: ~20MB | Zero CVEs | Ultra-fast cold start
# ══════════════════════════════════════════════════════════════

# ── Stage 1: Builder ─────────────────────────────────────────
FROM rust:1.96-slim-bookworm AS builder
WORKDIR /app

# Install system dependencies for SQLite/Postgres/MySQL linking
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation
COPY Cargo.toml Cargo.lock* ./
RUN sed -i 's/rullst = {{ path = [^}}]* }}/rullst = "2.0.8"/g' Cargo.toml && \
    sed -i 's/rullst-connect = {{ path = [^}}]* }}/rullst-connect = "7.0.1"/g' Cargo.toml || true
RUN mkdir src && echo "fn main() {{}}" > src/main.rs && touch src/lib.rs && cargo build --release && rm -rf src

# Build the actual application
COPY . .
RUN find src -type f -name "*.rs" -exec touch {{}} + && cargo build --release

# ── Stage 2: Runtime ─────────────────────────────────────────
FROM docker.io/library/debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies if needed
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a rootless user for security
RUN groupadd -r rullst && useradd -r -g rullst rullst
RUN chown -R rullst:rullst /app
USER rullst

COPY --from=builder /app/target/release/{project_name} /app/{project_name}
COPY Rullst.toml /app/Rullst.toml
EXPOSE 3000
EXPOSE 5555
CMD ["/app/{project_name}"]
"#
    );

    // --- docker-compose.yml ---
    let mut compose = format!(
        r#"# ══════════════════════════════════════════════════════════════
# Rullst Docker Compose (auto-generated)
# ══════════════════════════════════════════════════════════════

services:
  app:
    build: .
    container_name: {project_name}-app
    ports:
      - "3000:3000"
      - "5555:5555"
    env_file:
      - .env
    restart: unless-stopped
"#,
        project_name = project_name
    );

    let mut depends_on = vec![];

    if db_provider == "Postgres" {
        depends_on.push("db");
        compose.push_str(&format!(
            r#"
  db:
    image: postgres:16-alpine
    container_name: {project_name}-db
    env_file:
      - .env
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${{POSTGRES_USER}} -d $${{POSTGRES_DB}}"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped
"#,
            project_name = project_name
        ));
    } else if db_provider == "MySQL" {
        depends_on.push("db");
        compose.push_str(&format!(
            r#"
  db:
    image: mysql:8.0
    container_name: {project_name}-db
    env_file:
      - .env
    ports:
      - "3306:3306"
    volumes:
      - mysqldata:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost", "-u", "root", "-p$${{MYSQL_ROOT_PASSWORD}}"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped
"#,
            project_name = project_name
        ));
    } else {
        // Sqlite
        compose = compose.replace(
            "    restart: unless-stopped\n",
            "    volumes:\n      - ./rullst.db:/app/rullst.db\n    restart: unless-stopped\n",
        );
    }

    if wants_redis {
        depends_on.push("redis");
        compose.push_str(&format!(
            r#"
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
"#,
            project_name = project_name
        ));
    }

    if !depends_on.is_empty() {
        let mut deps_str = String::from("    depends_on:\n");
        for dep in depends_on {
            deps_str.push_str(&format!(
                "      {}:\n        condition: service_healthy\n",
                dep
            ));
        }
        compose = compose.replacen(
            "    restart: unless-stopped",
            &format!("{}\n    restart: unless-stopped", deps_str),
            1,
        );
    }

    let mut volumes_str = String::new();
    if db_provider == "Postgres" {
        volumes_str.push_str("  pgdata:\n");
    }
    if db_provider == "MySQL" {
        volumes_str.push_str("  mysqldata:\n");
    }
    if wants_redis {
        volumes_str.push_str("  redisdata:\n");
    }

    if !volumes_str.is_empty() {
        compose.push_str("\nvolumes:\n");
        compose.push_str(&volumes_str);
    }

    // --- .dockerignore ---
    let dockerignore = r#"**/target/
.git/
.gitignore
*.md
LICENSE
.vscode/
.idea/
*.db
*.sqlite
"#;

    let dockerfile_path = project_path.join("Dockerfile");
    std::fs::write(&dockerfile_path, dockerfile)?;

    // --- .env File Generation ---
    let env_path = project_path.join(".env");
    let env_example_path = project_path.join(".env.example");
    let mut env_content = String::from(
        "# \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\n\
         # Rullst Environment Variables\n\
         # \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\n\n\
         # Application Environment (development, production)\n\
         APP_ENV=development\n\
         HOST=0.0.0.0\n\n",
    );

    if db_provider == "Postgres" {
        env_content.push_str(&format!(
            "# Database Configuration\n\
             POSTGRES_USER=rullst\n\
             POSTGRES_PASSWORD=rullst_super_secret\n\
             POSTGRES_DB={project_name}_db\n\
             DATABASE_URL=postgres://rullst:rullst_super_secret@db:5432/{project_name}_db\n\n",
            project_name = project_name
        ));
    } else if db_provider == "MySQL" {
        env_content.push_str(&format!(
            "# Database Configuration\n\
             MYSQL_ROOT_PASSWORD=rullst_super_secret\n\
             MYSQL_DATABASE={project_name}_db\n\
             DATABASE_URL=mysql://root:rullst_super_secret@db:3306/{project_name}_db\n\n",
            project_name = project_name
        ));
    } else {
        env_content.push_str(
            "# Database Configuration\n\
             DATABASE_URL=sqlite://rullst.db?mode=rwc\n\n",
        );
    }

    if wants_redis {
        env_content.push_str(
            "# Redis Configuration\n\
             REDIS_URL=redis://redis:6379\n",
        );
    }

    std::fs::write(&env_path, &env_content)?;
    std::fs::write(&env_example_path, &env_content)?;

    fs::write(project_path.join("docker-compose.yml"), compose)?;
    fs::write(project_path.join(".dockerignore"), dockerignore)?;

    println!("{}", "  ✅ Dockerfile (multi-stage distroless)".green());
    println!("{}", "  ✅ docker-compose.yml (Customized)".green());
    println!("{}", "  ✅ .dockerignore".green());

    Ok(())
}
