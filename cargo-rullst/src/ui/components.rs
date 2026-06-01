// src/ui/components.rs — Neon spinners, interactive dashboard, update banner,
// and the full Rullst CLI help reference. Zero file I/O here.

use crate::cli::{Commands, DocsCommands, run_cli_command, Cli};
use clap::Parser;
use colored::*;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

/// Runs a closure while showing an animated neon spinner in a background thread.
pub fn with_spinner<F, T>(msg: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    let msg_owned = msg.to_string();
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let colors: [fn(&str) -> String; 4] = [
        |s: &str| s.cyan().bold().to_string(),
        |s: &str| s.magenta().bold().to_string(),
        |s: &str| s.bright_cyan().bold().to_string(),
        |s: &str| s.blue().bold().to_string(),
    ];
    let handle = std::thread::spawn(move || {
        let mut i = 0usize;
        while !done_clone.load(Ordering::Relaxed) {
            let frame = frames[i % frames.len()];
            let color_fn = colors[i % colors.len()];
            print!("\r  {} {}", color_fn(frame), msg_owned.white().bold());
            let _ = std::io::stdout().flush();
            std::thread::sleep(std::time::Duration::from_millis(80));
            i += 1;
        }
    });
    let result = f();
    done.store(true, Ordering::Relaxed);
    let _ = handle.join();
    print!("\r");
    let _ = std::io::stdout().flush();
    result
}

// ─── Interactive Dashboard ────────────────────────────────────────────────────

/// Shows the beautiful interactive Rullst CLI dashboard when called with no arguments.
pub fn show_interactive_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen and print the neon ASCII banner
    print!("\x1B[2J\x1B[1;1H");
    println!();
    println!(
        "{}",
        r#"  ██████╗ ██╗   ██╗██╗     ██╗     ███████╗████████╗"#
            .bright_magenta()
            .bold()
    );
    println!(
        "{}",
        r#"  ██╔══██╗██║   ██║██║     ██║     ██╔════╝╚══██╔══╝"#
            .magenta()
            .bold()
    );
    println!(
        "{}",
        r#"  ██████╔╝██║   ██║██║     ██║     ███████╗   ██║   "#
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        r#"  ██╔══██╗██║   ██║██║     ██║     ╚════██║   ██║   "#
            .cyan()
            .bold()
    );
    println!(
        "{}",
        r#"  ██║  ██║╚██████╔╝███████╗███████╗███████║   ██║   "#
            .bright_blue()
            .bold()
    );
    println!(
        "{}",
        r#"  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝╚══════╝   ╚═╝   "#
            .blue()
            .bold()
    );
    println!();
    println!(
        "  {} {} {}",
        "The".white(),
        "Ultimate Full-Stack Rust Framework".bright_cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_yellow().bold()
    );
    println!(
        "  {}",
        "⚡ Security · Speed · Developer Experience ⚡"
            .bright_magenta()
            .bold()
    );
    println!();
    println!(
        "  {}",
        "┌─────────────────────────────────────────────────────────────────┐"
            .bright_cyan()
    );
    println!(
        "  {} {:<65}{}",
        "│".bright_cyan(),
        "  🎯 RULLST APP CREATOR — What would you like to do?",
        "│".bright_cyan()
    );
    println!(
        "  {}",
        "└─────────────────────────────────────────────────────────────────┘"
            .bright_cyan()
    );
    println!();

    let theme = dialoguer::theme::ColorfulTheme::default();
    let choices = &[
        "✨  Create a New Project     (Rullst App Creator + Blueprints)",
        "🛠️  Scaffold Code            (Controllers, Models, Middlewares, Workers)",
        "🗄️  Database Operations      (Migrate, Rollback, Status, Seed)",
        "🔐  Integrate Auth & Billing (Auth, Stripe/LemonSqueezy, Passkeys)",
        "🖥️  Package for Desktop/App  (Tauri Desktop, Dioxus Cross-Platform)",
        "🚀  Deploy to Cloud          (Foundry: AWS, GCP, Hetzner, Azure, DO)",
        "📚  Docs Site Generator      (RullstPress Static Site)",
        "💡  View Full Help & Commands Reference",
    ];

    let selection = dialoguer::Select::with_theme(&theme)
        .with_prompt("Navigate with ↑↓, confirm with Enter")
        .default(0)
        .items(&choices[..])
        .interact()?;

    println!();

    match selection {
        0 => {
            // Create new project — run the wizard
            let args_vec: Vec<String> = vec![
                std::env::args().next().unwrap_or_default(),
                "new".to_string(),
            ];
            let cli = Cli::parse_from(args_vec);
            run_cli_command(&cli.command)?;
        }
        1 => {
            // Scaffold submenu
            let scaffold_choices = &[
                "make:controller  — Create a new Controller",
                "make:model       — Create a new Model (+migration)",
                "make:middleware  — Create a new Middleware",
                "make:worker      — Create a Background Worker",
                "make:migration   — Create a blank Migration",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("🛠️  Scaffold — pick a type")
                .default(0)
                .items(&scaffold_choices[..])
                .interact()?;
            let name: String = dialoguer::Input::with_theme(&theme)
                .with_prompt("Name?")
                .interact_text()?;
            match s {
                0 => run_cli_command(&Commands::MakeController {
                    name: name.clone(),
                    api: false,
                })?,
                1 => run_cli_command(&Commands::MakeModel {
                    name: name.clone(),
                    migration: true,
                })?,
                2 => run_cli_command(&Commands::MakeMiddleware { name: name.clone() })?,
                3 => run_cli_command(&Commands::MakeWorker { name: name.clone() })?,
                4 => run_cli_command(&Commands::MakeMigration { name: name.clone() })?,
                _ => {}
            }
        }
        2 => {
            // DB submenu
            let db_choices = &[
                "db:migrate   — Run pending migrations",
                "db:rollback  — Rollback last batch",
                "db:status    — Show migration status",
                "db:seed      — Run seeders",
                "studio       — Open Rullst Studio DB browser",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("🗄️  Database Operation")
                .default(0)
                .items(&db_choices[..])
                .interact()?;
            match s {
                0 => run_cli_command(&Commands::DbMigrate)?,
                1 => run_cli_command(&Commands::DbRollback)?,
                2 => run_cli_command(&Commands::DbStatus)?,
                3 => run_cli_command(&Commands::DbSeed)?,
                4 => run_cli_command(&Commands::Studio)?,
                _ => {}
            }
        }
        3 => {
            // Auth & Billing submenu
            let auth_choices = &[
                "auth          — Scaffold full Auth system (Login, Register, Passkeys)",
                "make:billing  — Scaffold Stripe/LemonSqueezy billing",
                "make:cors     — Add CORS middleware",
                "make:jwt      — Add JWT middleware",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("🔐  Auth & Billing")
                .default(0)
                .items(&auth_choices[..])
                .interact()?;
            match s {
                0 => run_cli_command(&Commands::Auth)?,
                1 => run_cli_command(&Commands::MakeBilling)?,
                2 => run_cli_command(&Commands::MakeCors)?,
                3 => run_cli_command(&Commands::MakeJwt)?,
                _ => {}
            }
        }
        4 => {
            // Desktop/Multi-platform submenu
            let platform_choices = &[
                "make:desktop — Rullst Hyper (Tauri native desktop window)",
                "make:omni    — Rullst Omni (Dioxus cross-platform app)",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("🖥️  Platform Packaging")
                .default(0)
                .items(&platform_choices[..])
                .interact()?;
            match s {
                0 => run_cli_command(&Commands::MakeDesktop)?,
                1 => run_cli_command(&Commands::MakeOmni)?,
                _ => {}
            }
        }
        5 => {
            // Cloud deploy submenu
            let cloud_choices = &[
                "foundry:init    — Create Foundry.toml deployment manifest",
                "foundry:deploy  — Deploy to cloud via SSH pipeline",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("🚀  Cloud Deploy (Rullst Foundry)")
                .default(0)
                .items(&cloud_choices[..])
                .interact()?;
            match s {
                0 => run_cli_command(&Commands::FoundryInit)?,
                1 => run_cli_command(&Commands::FoundryDeploy)?,
                _ => {}
            }
        }
        6 => {
            // Docs submenu
            let docs_choices = &[
                "docs dev   — Start local live-preview server",
                "docs build — Compile Markdown to static HTML",
            ];
            let s = dialoguer::Select::with_theme(&theme)
                .with_prompt("📚  RullstPress Static Docs")
                .default(0)
                .items(&docs_choices[..])
                .interact()?;
            match s {
                0 => run_cli_command(&Commands::Docs {
                    action: DocsCommands::Dev,
                })?,
                1 => run_cli_command(&Commands::Docs {
                    action: DocsCommands::Build,
                })?,
                _ => {}
            }
        }
        _ => {
            show_help_reference();
        }
    }
    Ok(())
}

// ─── Help Reference ───────────────────────────────────────────────────────────

/// Prints a beautiful grouped cheat-sheet of all Rullst CLI commands.
pub fn show_help_reference() {
    println!();
    println!(
        "  {}",
        "╔═══════════════════════════════════════════════════════════════╗"
            .bright_cyan()
            .bold()
    );
    println!(
        "  {} {:<63} {}",
        "║".bright_cyan().bold(),
        "  💡 Rullst CLI — Full Command Reference",
        "║".bright_cyan().bold()
    );
    println!(
        "  {}",
        "╠═══════════════════════════════════════════════════════════════╣"
            .bright_cyan()
            .bold()
    );
    let groups = [
        (
            "🗂️  PROJECT",
            vec![
                ("cargo rullst new [name]", "Create a new Rullst application"),
                ("cargo rullst upgrade", "Upgrade Rullst with safe codemods"),
            ],
        ),
        (
            "🛠️  SCAFFOLDING",
            vec![
                ("cargo rullst make:controller <Name>", "New controller"),
                ("cargo rullst make:model <Name> -m", "New model (+migration)"),
                ("cargo rullst make:middleware <Name>", "New middleware"),
                ("cargo rullst make:worker <Name>", "New background worker"),
                ("cargo rullst make:migration <name>", "Blank migration"),
            ],
        ),
        (
            "🗄️  DATABASE",
            vec![
                ("cargo rullst db:migrate", "Run pending migrations"),
                ("cargo rullst db:rollback", "Rollback last batch"),
                ("cargo rullst db:status", "Show migration status"),
                ("cargo rullst db:seed", "Run seeders"),
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
            "🖥️  DESKTOP & CROSS-PLATFORM",
            vec![
                ("cargo rullst make:desktop", "Tauri native desktop packaging"),
                ("cargo rullst make:omni", "Dioxus cross-platform app"),
            ],
        ),
        (
            "🚀  DEPLOY",
            vec![
                ("cargo rullst foundry:init", "Create Foundry.toml manifest"),
                ("cargo rullst foundry:deploy", "Deploy via SSH pipeline"),
            ],
        ),
        (
            "📦  BUILD & DOCS",
            vec![
                ("cargo rullst build", "Production binary + Brotli/Zstd assets"),
                ("cargo rullst build:client", "Compile Wasm Islands"),
                ("cargo rullst generate:openapi", "Generate OpenAPI spec"),
                ("cargo rullst docs dev", "Live docs preview server"),
                ("cargo rullst docs build", "Build static docs site"),
            ],
        ),
    ];
    for (group_name, cmds) in &groups {
        println!(
            "  {} {} {}",
            "║".bright_cyan().bold(),
            format!(" {:<63}", group_name).bright_yellow().bold(),
            "║".bright_cyan().bold()
        );
        for (cmd, desc) in cmds {
            println!(
                "  {} {:<40} {} {}",
                "║".bright_cyan().bold(),
                cmd.bright_cyan(),
                format!("{:<22}", desc).white(),
                "║".bright_cyan().bold()
            );
        }
        println!(
            "  {} {:<63} {}",
            "║".bright_cyan().bold(),
            "",
            "║".bright_cyan().bold()
        );
    }
    println!(
        "  {}",
        "╚═══════════════════════════════════════════════════════════════╝"
            .bright_cyan()
            .bold()
    );
    println!();
}
