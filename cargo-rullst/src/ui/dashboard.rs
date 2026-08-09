// src/ui/dashboard.rs — Interactive Rullst CLI dashboard (menus, logo, handlers).

use colored::*;

fn print_neon_logo() {
    let color_logo = |s: &str| s.truecolor(255, 165, 0).bold();
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
        "⚡  LiveView Reactive UI  (cargo rullst make:live)",
        "🏝️  Wasm Island Component (cargo rullst make:island)",
        "📡  Scalar API Playground (cargo rullst make:scalar)",
        "☸️  Kubernetes Manifests  (cargo rullst make:k8s)",
        "🔌  gRPC Microservice     (cargo rullst make:grpc)",
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
        5 => (
            "Enter LiveView component name (e.g. Counter):",
            "make:live",
            vec![],
        ),
        6 => (
            "Enter Wasm island component name (e.g. Chart):",
            "make:island",
            vec![],
        ),
        7 => {
            return execute_command(vec![
                std::env::args().next().unwrap_or_default(),
                "make:scalar".to_string(),
            ]);
        }
        8 => {
            return execute_command(vec![
                std::env::args().next().unwrap_or_default(),
                "make:k8s".to_string(),
            ]);
        }
        9 => {
            return execute_command(vec![
                std::env::args().next().unwrap_or_default(),
                "make:grpc".to_string(),
            ]);
        }
        10 => {
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
        "📲  Scaffold 2FA TOTP System   (cargo rullst make:mfa)",
        "🛡️  Run Security & IDOR Audit  (cargo rullst audit --ai --compliance --idor)",
        "💳  Scaffold Stripe Billing    (cargo rullst make:billing)",
        "🌐  Add CORS Middleware        (cargo rullst make:cors)",
        "🔑  Add JWT Middleware         (cargo rullst make:jwt)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose auth & security action:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let mut args = vec![std::env::args().next().unwrap_or_default()];
    match selection {
        0 => args.push("auth".to_string()),
        1 => args.push("make:mfa".to_string()),
        2 => {
            args.push("audit".to_string());
            args.push("--ai".to_string());
            args.push("--compliance".to_string());
            args.push("--idor".to_string());
        }
        3 => args.push("make:billing".to_string()),
        4 => args.push("make:cors".to_string()),
        5 => args.push("make:jwt".to_string()),
        _ => return Ok(()),
    };
    execute_command(args)
}

fn handle_deploy(
    theme: &dialoguer::theme::ColorfulTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    let choices = [
        "🚀  One-Click PaaS Deploy      (cargo rullst deploy)",
        "⚙️  Initialize Foundry Config  (cargo rullst foundry:init)",
        "🚀  Deploy via SSH Pipeline    (cargo rullst foundry:deploy)",
    ];
    let selection = dialoguer::Select::with_theme(theme)
        .with_prompt("Choose deployment action:\n")
        .default(0)
        .items(&choices[..])
        .interact()?;
    let cmd = match selection {
        0 => "deploy",
        1 => "foundry:init",
        2 => "foundry:deploy",
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
            "(Controllers, Models, LiveView, Islands, gRPC)".dimmed()
        ),
        format!(
            "🗄  Database Operations      {}",
            "(Migrate, Rollback, Status, Seed)".dimmed()
        ),
        format!(
            "🔐  Integrate Auth & Security {}",
            "(Auth, 2FA MFA, IDOR/SOC Audit, Billing)".dimmed()
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
            "(1-Click PaaS: Fly, Railway, Render, VPS)".dimmed()
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
            super::help::show_help_reference();
            Ok(())
        }
        3 => {
            println!("{}", "Exiting. Happy coding with Rullst! 🦀🚀".dimmed());
            Ok(())
        }
        _ => Ok(()),
    }
}
