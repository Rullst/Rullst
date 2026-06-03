// src/ui/components.rs — Neon spinners, interactive dashboard, update banner,
// and the full Rullst CLI help reference. Zero file I/O here.

use crate::cli::{Cli, run_cli_command};
use clap::Parser;
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

        while is_running_clone.load(Ordering::SeqCst) {
            let frame = frames[i % frames.len()];
            let color = colors[(i / 2) % colors.len()];

            let mut animated_msg = String::new();
            let targets = ["Application", "migrations"];
            let mut found_target = None;

            for target in targets {
                if let Some(pos) = msg.find(target) {
                    found_target = Some((target, pos));
                    break;
                }
            }

            if let Some((target, pos)) = found_target {
                animated_msg.push_str(&msg[..pos].bold().to_string());
                for (j, ch) in target.chars().enumerate() {
                    let is_upper = ((i + j) % 4) < 2; // wave effect
                    let wave_char = if is_upper {
                        ch.to_ascii_uppercase()
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    let c_idx = (i + j) % colors.len();
                    animated_msg.push_str(
                        &wave_char
                            .to_string()
                            .color(colors[c_idx])
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
pub fn show_interactive_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen and print the neon ASCII banner
    print!("\x1B[2J\x1B[1;1H");
    let color_logo = |s: &str| s.truecolor(255, 165, 0).bold(); // Orange

    println!();
    println!(
        "{}",
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
    println!();
    println!(
        "  {} {} {}",
        "The".white(),
        "Ultimate Full-Stack Rust Framework".bright_cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_yellow()
    );
    println!(
        "  {}",
        "⚡ Security · Speed · Developer Experience ⚡"
            .bright_magenta()
            .bold()
    );

    println!("\n");

    let theme = dialoguer::theme::ColorfulTheme::default();
    let choices = &[
        "✨  Create a New Project     (Rullst App Creator + Blueprints)",
        "🔄  Safe Upgrade             (Self-Healing Updates & Codemods)",
        "🚀  Start Dev Server         (Fast dev build + Hot Reload. Shortcut for 'cargo rullst dev')",
        /* --- HIDDEN FOR MVP ---
        "💡  View Help & Commands     (Framework Reference)",
        "🛠️  Scaffold Code            (Controllers, Models, Middlewares, Workers)",
        "🗄️  Database Operations      (Migrate, Rollback, Status, Seed)",
        "🔐  Integrate Auth & Billing (Auth, Stripe/LemonSqueezy, Passkeys)",
        "🖥️  Package for Desktop/App  (Tauri Desktop, Dioxus Cross-Platform)",
        "🚀  Deploy to Cloud          (Foundry: AWS, GCP, Hetzner, Azure, DO)",
        "📚  Docs Site Generator      (RullstPress Static Site)",
        */
    ];

    let selection = dialoguer::Select::with_theme(&theme)
        .with_prompt("Navigate with ↑↓, confirm with Enter\n")
        .default(0)
        .items(&choices[..])
        .interact()?;

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
            // Upgrade
            let args_vec: Vec<String> = vec![
                std::env::args().next().unwrap_or_default(),
                "upgrade".to_string(),
            ];
            let cli = Cli::parse_from(args_vec);
            run_cli_command(&cli.command)?;
        }
        2 => {
            // Dev Server
            let args_vec: Vec<String> = vec![
                std::env::args().next().unwrap_or_default(),
                "dev".to_string(),
            ];
            let cli = Cli::parse_from(args_vec);
            run_cli_command(&cli.command)?;
        }
        _ => {}
    }
    Ok(())
}

// ─── Help Reference ───────────────────────────────────────────────────────────

/// Prints a beautiful grouped cheat-sheet of all Rullst CLI commands.
#[allow(dead_code)]
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
            "🗂️  PROJECT (MVP)",
            vec![
                ("cargo rullst new [name]", "Create a new Rullst application"),
                ("cargo rullst upgrade", "Upgrade Rullst with safe codemods"),
            ],
        ),
        /* --- HIDDEN FOR MVP ---
        (
            "🛠️  SCAFFOLDING",
            vec![
                ("cargo rullst make:controller <Name>", "New controller"),
                (
                    "cargo rullst make:model <Name> -m",
                    "New model (+migration)",
                ),
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
                (
                    "cargo rullst make:desktop",
                    "Tauri native desktop packaging",
                ),
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
                (
                    "cargo rullst build",
                    "Production binary + Brotli/Zstd assets",
                ),
                ("cargo rullst build:client", "Compile Wasm Islands"),
                ("cargo rullst generate:openapi", "Generate OpenAPI spec"),
                ("cargo rullst docs dev", "Live docs preview server"),
                ("cargo rullst docs build", "Build static docs site"),
            ],
        ),
        */
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
