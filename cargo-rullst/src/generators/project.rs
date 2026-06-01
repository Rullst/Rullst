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
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(123456789);
    
    let mut rng = seed;
    let mut key = String::new();
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    for _ in 0..32 {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (rng % (chars.len() as u128)) as usize;
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
                    .with_prompt("🚀 App name? (no spaces allowed)")
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
    let mut blueprint_selection = 0usize;

    if name_arg.is_none() {
        let blueprint_choices = &[
            "Blank Starter (Minimal template with HTMX reactive counter)",
            "LMS Platform (Courses, lessons, video player, HTMX integration)",
            "SaaS App Starter (Authentication + Stripe payments billing template)",
            "Blog / Press (Static site generator pre-wired with Nexus CMS)",
            "ERP Pocket (Inventory, stock management, orders tracker, auto-CMS)",
            "Uptime Monitor (Ping dashboard, background status checker, glassmorphism)",
        ];
        blueprint_selection = dialoguer::Select::with_theme(&theme)
            .with_prompt("🧭 Select a Starter Blueprint")
            .default(0)
            .items(&blueprint_choices[..])
            .interact()?;

        if blueprint_selection == 0 {
            let build_options = &[
                "Full-Stack Web App (SaaS, Portfolio, Blog)",
                "Headless REST API",
            ];
            let build_selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("🏗️ What would you like to build?")
                .default(0)
                .items(&build_options[..])
                .interact()?;
            api = build_selection == 1;

            hot_reload = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("🔥 Enable Hot Reloading by default?")
                .default(false)
                .interact()?;

            db_needed = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("🗄️ Will your project need a Database?")
                .default(true)
                .interact()?;

            if db_needed {
                let db_options = &["Sqlite", "Postgres", "MySQL/MariaDB"];
                let db_selection = dialoguer::Select::with_theme(&theme)
                    .with_prompt("💾 Select a DB Provider")
                    .default(0)
                    .items(&db_options[..])
                    .interact()?;
                db_provider = match db_options[db_selection] {
                    "MySQL/MariaDB" => "MySQL".to_string(),
                    other => other.to_string(),
                };
            }
        } else {
            // LMS, SaaS, and Blog blueprints require database configuration (always Sqlite by default)
            db_needed = true;
            db_provider = "Sqlite".to_string();
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

    // Fix Windows path escaping in Cargo.toml and strip UNC prefix \\?\ if present
    let rullst_path = rullst_path.trim_start_matches(r"\\?\").replace("\\", "/");

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

    if blueprint_selection == 5 {
        cargo_toml.push_str("reqwest = { version = \"0.12\", default-features = false, features = [\"rustls-tls\"] }\n");
    }

    // Special dependencies for SaaS blueprint
    if blueprint_selection == 2 {
        let sibling_path = current_dir.join("rullst-connect");
        let connect_dep = if sibling_path.exists() {
            let absolute_path = sibling_path
                .canonicalize()?
                .display()
                .to_string()
                .replace("\\", "/");
            format!("rullst-connect = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-connect = \"0.4.0\"\n".to_string()
        };
        cargo_toml.push_str(&connect_dep);
        cargo_toml.push_str("webauthn-rs = { version = \"0.5\", default-features = false }\n");
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
        _ => "sqlite://rullst.db",
    };

    let mut env_content = format!(
        r#"# ─────────────────────────────────────────────────────────────
#  Rullst Application Environment Configuration
#  Generated automatically by cargo rullst new
# ─────────────────────────────────────────────────────────────

# ⚠️ SECURITY: This file must NEVER be committed to git.
# It is automatically added to .gitignore by the Rullst CLI.

# ── Database ──────────────────────────────────────────────────
DATABASE_URL={db_url}

# ── Application ───────────────────────────────────────────────
APP_KEY={app_key}
APP_ENV=development
"#,
        db_url = db_url,
        app_key = app_key
    );

    let mut env_example_content = format!(
        r#"# ─────────────────────────────────────────────────────────────
#  Rullst Application Environment Configuration
#  Generated automatically by cargo rullst new
# ─────────────────────────────────────────────────────────────

# ── Database ──────────────────────────────────────────────────
DATABASE_URL={db_url}

# ── Application ───────────────────────────────────────────────
APP_KEY=REPLACE_WITH_YOUR_32_CHAR_APP_KEY
APP_ENV=development
"#,
        db_url = db_url
    );

    if blueprint_selection == 1 || blueprint_selection == 2 {
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
        generate_docker_files(path, &project_name)?;
    }

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

pub fn generate_docker_files(
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
