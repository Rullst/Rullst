// src/ui/components.rs — Neon spinners, interactive dashboard, update banner,
// and the full Rullst CLI help reference. Zero file I/O here.

use colored::*;

// ─── Update Check ────────────────────────────────────────────────────────────

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

pub fn check_update_available() -> Option<String> {
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

pub fn trigger_background_update_check() {
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

// ─── Update Banner ────────────────────────────────────────────────────────────

pub fn print_update_banner(latest_version: &str) {
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

// ─── Spinner ─────────────────────────────────────────────────────────────────

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn with_spinner<F, T>(msg: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let msg = msg.to_string();
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    let handle = thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        let colors = [
            colored::Color::Cyan,
            colored::Color::Magenta,
            colored::Color::BrightCyan,
            colored::Color::Blue,
        ];

        let re = regex::Regex::new(r"Application|migrations|Omni").unwrap();

        while is_running_clone.load(Ordering::SeqCst) {
            let frame = frames[i % frames.len()];
            let color = colors[(i / 2) % colors.len()];

            let mut animated_msg = String::new();

            let mut found_target = None;
            if let Some(mat) = re.find(&msg) {
                found_target = Some((mat.as_str(), mat.start()));
            }

            if let Some((target, pos)) = found_target {
                animated_msg.push_str(&msg[..pos].bold().to_string());

                // Use special colors for Omni (orange/red)
                let custom_colors = if target == "Omni" {
                    vec![
                        colored::Color::Red,
                        colored::Color::TrueColor {
                            r: 255,
                            g: 165,
                            b: 0,
                        },
                        colored::Color::BrightRed,
                        colored::Color::Yellow,
                    ]
                } else {
                    colors.to_vec()
                };

                for (j, ch) in target.chars().enumerate() {
                    let is_upper = ((i + j) % 4) < 2; // wave effect
                    let wave_char = if is_upper {
                        ch.to_ascii_uppercase()
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    let c_idx = (i + j) % custom_colors.len();
                    animated_msg.push_str(
                        &wave_char
                            .to_string()
                            .color(custom_colors[c_idx])
                            .bold()
                            .to_string(),
                    );
                }
                animated_msg.push_str(&msg[pos + target.len()..].bold().to_string());
            } else {
                animated_msg = msg.bold().to_string();
            }

            print!("\r\x1B[K{} {}", frame.color(color).bold(), animated_msg);
            let _ = std::io::stdout().flush();

            thread::sleep(Duration::from_millis(80));
            i += 1;
        }
        print!("\r\x1B[K");
        let _ = std::io::stdout().flush();
    });

    let result = f();

    is_running.store(false, Ordering::SeqCst);
    let _ = handle.join();

    result
}

// ─── Interactive Dashboard ────────────────────────────────────────────────────

/// Shows the beautiful interactive Rullst CLI dashboard when called with no arguments.
fn print_neon_logo() {
    let color_logo = |s: &str| s.truecolor(255, 165, 0).bold(); // Orange
    println!(
        "\n{}",
        color_logo(r#"  ██████╗ ██╗   ██╗██╗     ██╗     ███████╗████████╗"#)
    );
    println!(
        "{}",
        color_logo(r#"  ██╔══██╗██║   ██║██║     ██║     ██╔════╝╚══██╔══╝"#)
    );
    println!(
        "{}",
        color_logo(r#"  ██████╔╝██║   ██║██║     ██║     ███████╗   ██║   "#)
    );
    println!(
        "{}",
        color_logo(r#"  ██╔══██╗██║   ██║██║     ██║     ╚════██║   ██║   "#)
    );
    println!(
        "{}",
        color_logo(r#"  ██║  ██║╚██████╔╝███████╗███████╗███████║   ██║   "#)
    );
    println!(
        "{}",
        color_logo(r#"  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝╚══════╝   ╚═╝   "#)
    );
    println!(
        "\n  {} {} {}",
        "The".white(),
        "Ultimate Full-Stack Rust Framework".bright_cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_yellow()
    );
    println!(
        "  {}\n",
        "⚡ Security · Speed · Developer Experience ⚡"
            .bright_magenta()
            .bold()
    );
}

pub fn execute_command(cmd_args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    std::process::Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .status()?;
    Ok(())
}

fn handle_scaffold_code(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        "🎮  Controller            (cargo rullst make:controller)",
        "💾  Model & Migration     (cargo rullst make:model -m)",
        "🚪  Middleware            (cargo rullst make:middleware)",
        "⚙️  Background Worker     (cargo rullst make:worker)",
        "📂  Blank Migration       (cargo rullst make:migration)",
        "🤖  Introspect DB Models  (cargo rullst generate:models)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose component to scaffold:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;

    let (prompt, action, extra_args) = match selection {
        0 => (
            "Enter controller name (e.g. UsersController):",
            "make:controller",
            vec![],
        ),
        1 => (
            "Enter model name (e.g. Product):",
            "make:model",
            vec!["-m".to_string()],
        ),
        2 => (
            "Enter middleware name (e.g. RateLimiter):",
            "make:middleware",
            vec![],
        ),
        3 => (
            "Enter worker name (e.g. EmailSender):",
            "make:worker",
            vec![],
        ),
        4 => (
            "Enter migration name (e.g. add_status_to_users):",
            "make:migration",
            vec![],
        ),
        5 => {
            return execute_command(vec![
                std::env::args().next().unwrap_or_default(),
                "generate:models".to_string(),
            ]);
        }
        _ => return Ok(()),
    };

    let name: String = dialoguer::Input::with_theme(theme)
        .with_prompt(prompt)
        .interact_text()?;
    let mut args = vec![
        std::env::args().next().unwrap_or_default(),
        action.to_string(),
        name,
    ];
    args.extend(extra_args);
    execute_command(args)
}

fn handle_database_operations(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        "🚀  Run Migrations       (cargo rullst db:migrate)",
        "🔄  Rollback Last Batch  (cargo rullst db:rollback)",
        "📊  Migration Status     (cargo rullst db:status)",
        "🌱  Run Seeders          (cargo rullst db:seed)",
        "🖥️  Open Studio Browser  (cargo rullst studio)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose database operation:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let cmd = match selection {
        0 => "db:migrate",
        1 => "db:rollback",
        2 => "db:status",
        3 => "db:seed",
        4 => "studio",
        _ => return Ok(()),
    };
    execute_command(vec![
        std::env::args().next().unwrap_or_default(),
        cmd.to_string(),
    ])
}

fn handle_auth_billing(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        "🔐  Scaffold Full Auth System  (cargo rullst auth)",
        "💳  Scaffold Stripe Billing    (cargo rullst make:billing)",
        "🌐  Add CORS Middleware        (cargo rullst make:cors)",
        "🔑  Add JWT Middleware         (cargo rullst make:jwt)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose auth & billing action:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let cmd = match selection {
        0 => "auth",
        1 => "make:billing",
        2 => "make:cors",
        3 => "make:jwt",
        _ => return Ok(()),
    };
    execute_command(vec![
        std::env::args().next().unwrap_or_default(),
        cmd.to_string(),
    ])
}

fn handle_deploy(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        "⚙️  Initialize Foundry Config  (cargo rullst foundry:init)",
        "🚀  Deploy via SSH Pipeline    (cargo rullst foundry:deploy)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose deployment action:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let cmd = match selection {
        0 => "foundry:init",
        1 => "foundry:deploy",
        _ => return Ok(()),
    };
    execute_command(vec![
        std::env::args().next().unwrap_or_default(),
        cmd.to_string(),
    ])
}

fn handle_existing_project(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        format!(
            "🚀  Start Dev Server (Standard)  {}",
            "(Fast dev build + Hot Reload)".dimmed()
        ),
        format!(
            "📺  Start Dev Server (Dashboard) {}",
            "(Ratatui Interactive Visuals)".dimmed()
        ),
        format!(
            "🛠  Scaffold Code            {}",
            "(Controllers, Models, Middlewares, Workers)".dimmed()
        ),
        format!(
            "🗄  Database Operations      {}",
            "(Migrate, Rollback, Status, Seed)".dimmed()
        ),
        format!(
            "🔐  Integrate Auth & Billing {}",
            "(Auth, Stripe/LemonSqueezy, Passkeys)".dimmed()
        ),
        format!(
            "🖥  Package for Desktop/App  {}",
            "(Omni Desktop & Mobile)".dimmed()
        ),
        format!(
            "🐳  Dockerize Project        {}",
            "(Generate Dockerfile & docker-compose)".dimmed()
        ),
        format!(
            "❄️  Nixify Project           {}",
            "(Generate Nix flake for reproducible env)".dimmed()
        ),
        format!(
            "🚀  Deploy to Cloud          {}",
            "(Foundry: AWS, GCP, Hetzner, Azure, DO)".dimmed()
        ),
        format!(
            "🔄  Safe Upgrade             {}",
            "(Self-Healing Updates & Codemods)".dimmed()
        ),
        "🔙  Back to Main Menu        ".to_string(),
    ];

    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Project Operations:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let base_cmd = std::env::args().next().unwrap_or_default();

    match selection {
        0 => execute_command(vec![base_cmd, "dev".to_string()]),
        1 => execute_command(vec![base_cmd, "dash".to_string()]),
        2 => handle_scaffold_code(theme),
        3 => handle_database_operations(theme),
        4 => handle_auth_billing(theme),
        5 => execute_command(vec![base_cmd, "make:omni".to_string()]),
        6 => execute_command(vec![base_cmd, "dockerize".to_string()]),
        7 => execute_command(vec![base_cmd, "nixify".to_string()]),
        8 => handle_deploy(theme),
        9 => execute_command(vec![base_cmd, "upgrade".to_string()]),
        10 => show_interactive_dashboard(),
        _ => Ok(()),
    }
}

pub fn show_interactive_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    print!("\x1B[2J\x1B[1;1H");
    print_neon_logo();

    let theme = dialoguer::theme::ColorfulTheme::default();
    let choices = [
        format!(
            "✨  Create New Project       {}",
            "(API, Fullstack or Dockerized)".dimmed()
        ),
        format!(
            "📁  Already have a project?  {}",
            "(Dev, Scaffold, DB, Auth, Deploy...)".dimmed()
        ),
        format!(
            "💡  View Help & Commands     {}",
            "(Framework Reference)".dimmed()
        ),
        format!(
            "❌  Exit                     {}",
            "(Close interactive menu)".dimmed()
        ),
    ];

    let selection = dialoguer::Select::with_theme(&theme)
        .with_prompt("Navigate with ↑↓, confirm with Enter\n")
        .default(0)
        .items(&choices[..])
        .interact()?;

    match selection {
        0 => execute_command(vec![
            std::env::args().next().unwrap_or_default(),
            "new".to_string(),
        ]),
        1 => handle_existing_project(&theme),
        2 => {
            show_help_reference();
            Ok(())
        }
        3 => {
            println!("{}", "Exiting. Happy coding with Rullst! 🦀🚀".dimmed());
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn get_help_groups() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    vec![
        (
            "🗂️  PROJECT & COMMUNITY",
            vec![
                ("cargo rullst new [name]", "Create a new Rullst application"),
                (
                    "cargo rullst dev [--ts-sync]",
                    "Start dev server (optional auto TS SDK sync)",
                ),
                (
                    "cargo rullst pkg add <name>",
                    "Install RullstPackage community extension",
                ),
                ("cargo rullst upgrade", "Upgrade Rullst with safe codemods"),
                (
                    "cargo rullst eject",
                    "Zero lock-in framework escape hatch to pure Axum",
                ),
            ],
        ),
        (
            "🛠️  SCAFFOLDING",
            vec![
                (
                    "cargo rullst make:resource <Name>",
                    "Full CRUD stack (Model, Migration, Controller, Views)",
                ),
                ("cargo rullst make:controller <Name>", "New controller"),
                (
                    "cargo rullst make:model <Name> -m",
                    "New model (+migration)",
                ),
                ("cargo rullst make:middleware <Name>", "New middleware"),
                ("cargo rullst make:worker <Name>", "New background worker"),
                ("cargo rullst make:migration <name>", "Blank migration"),
                (
                    "cargo rullst make:island <name>",
                    "New interactive Wasm Island",
                ),
                (
                    "cargo rullst make:live <Name>",
                    "Scaffold LiveView Server-Driven UI component",
                ),
                (
                    "cargo rullst make:grpc <Name>",
                    "Scaffold gRPC microservice & Protobuf schema",
                ),
                (
                    "cargo rullst make:k8s",
                    "Generate Kubernetes manifests & health probes",
                ),
                (
                    "cargo rullst make:scalar",
                    "Scaffold interactive Scalar OpenAPI docs",
                ),
                (
                    "cargo rullst make:iot <Name>",
                    "Scaffold bare-metal IoT edge sensor node",
                ),
                (
                    "cargo rullst generate:models",
                    "Reverse-engineer live DB to Models",
                ),
            ],
        ),
        (
            "🤖  AI & AGENTS",
            vec![
                (
                    "cargo rullst make:chat-session",
                    "Scaffold stateful AI chat & memory",
                ),
                (
                    "cargo rullst generate:ai-context",
                    "Generate .llms.txt context file",
                ),
            ],
        ),
        (
            "🗄️  DATABASE",
            vec![
                ("cargo rullst db:migrate", "Run pending migrations"),
                ("cargo rullst db:rollback", "Rollback last batch"),
                ("cargo rullst db:status", "Show migration status"),
                ("cargo rullst db:seed", "Run seeders"),
                (
                    "cargo rullst make:migration:auto",
                    "Auto-diff ORM models & DB schema",
                ),
                ("cargo rullst studio", "Open DB studio browser"),
            ],
        ),
        (
            "🔐  AUTH & BILLING",
            vec![
                ("cargo rullst auth", "Scaffold full auth system"),
                ("cargo rullst make:billing", "Scaffold Stripe billing"),
                ("cargo rullst make:cors", "Add CORS middleware"),
                ("cargo rullst make:jwt", "Add JWT middleware"),
            ],
        ),
        (
            "🖥️  DESKTOP & MOBILE (OMNI)",
            vec![
                (
                    "cargo rullst make:omni",
                    "Scaffold Omni desktop & mobile app wrapper",
                ),
                (
                    "cargo rullst omni [target]",
                    "Run Omni app (desktop, android, ios)",
                ),
            ],
        ),
        (
            "🚀  DEPLOY",
            vec![
                (
                    "cargo rullst deploy [--platform=...]",
                    "1-Click PaaS deploy (Fly, Railway, Render, VPS Caddy)",
                ),
                ("cargo rullst dockerize", "Generate Docker files"),
                (
                    "cargo rullst generate:buildah",
                    "Generate rootless OCI build script",
                ),
                ("cargo rullst nixify", "Generate Nix environment files"),
                ("cargo rullst foundry:init", "Create Foundry.toml manifest"),
                ("cargo rullst foundry:deploy", "Deploy via SSH pipeline"),
            ],
        ),
        (
            "📦  BUILD & DOCS",
            vec![
                (
                    "cargo rullst build",
                    "Production binary + Brotli/Zstd assets",
                ),
                ("cargo rullst build:client", "Compile Wasm Islands"),
                ("cargo rullst generate:openapi", "Generate OpenAPI spec"),
                (
                    "cargo rullst generate:diagram",
                    "Generate Mermaid ER diagram",
                ),
                (
                    "cargo rullst generate:ai-context",
                    "Generate AI context (.llms.txt)",
                ),
                (
                    "cargo rullst inspect [target]",
                    "Inspect expanded macro code & route schemas",
                ),
                (
                    "cargo rullst dash",
                    "Start interactive Ratatui Dev Dashboard",
                ),
                ("cargo rullst docs dev", "Live docs preview server"),
                ("cargo rullst docs build", "Build static docs site"),
            ],
        ),
    ]
}

pub fn show_help_reference() {
    print!("\x1B[2J\x1B[1;1H");
    println!(
        "\n  {}",
        "╔══════════════════════════════════════════╗"
            .bright_cyan()
            .bold()
    );
    println!(
        "  {}  💡 Rullst CLI - Full Command Reference  {}",
        "║".bright_cyan().bold(),
        "║".bright_cyan().bold()
    );
    println!(
        "  {}",
        "╚══════════════════════════════════════════╝"
            .bright_cyan()
            .bold()
    );

    for (group_name, cmds) in get_help_groups() {
        println!("  {}", group_name.bright_yellow().bold());
        for (cmd, desc) in cmds {
            println!("    {:<35} {}", cmd.bright_cyan(), desc.white());
        }
        println!();
    }
}
