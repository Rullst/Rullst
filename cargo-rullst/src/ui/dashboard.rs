// src/ui/dashboard.rs — Interactive Rullst CLI dashboard (menus, logo, handlers).

use super::dashboard_brand::{play_launch_pulse, print_neon_logo};
use colored::*;

type DashboardResult<T> = Result<T, Box<dyn std::error::Error>>;

trait DashboardUi {
    fn show_brand(&mut self) -> DashboardResult<()>;
    fn select(&mut self, prompt: &str, choices: &[String]) -> DashboardResult<usize>;
    fn input(&mut self, prompt: &str) -> DashboardResult<String>;
}

struct DialoguerUi {
    theme: dialoguer::theme::ColorfulTheme,
}

impl DialoguerUi {
    fn new() -> Self {
        Self {
            theme: dialoguer::theme::ColorfulTheme::default(),
        }
    }
}

impl DashboardUi for DialoguerUi {
    fn show_brand(&mut self) -> DashboardResult<()> {
        print!("\x1B[2J\x1B[1;1H");
        print_neon_logo()?;
        play_launch_pulse()?;
        Ok(())
    }

    fn select(&mut self, prompt: &str, choices: &[String]) -> DashboardResult<usize> {
        Ok(dialoguer::Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(0)
            .items(choices)
            .interact()?)
    }

    fn input(&mut self, prompt: &str) -> DashboardResult<String> {
        Ok(dialoguer::Input::with_theme(&self.theme)
            .with_prompt(prompt)
            .interact_text()?)
    }
}

pub fn execute_command(cmd_args: Vec<String>) -> DashboardResult<()> {
    let Some((program, arguments)) = cmd_args.split_first() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "dashboard command cannot be empty",
        )
        .into());
    };
    let status = std::process::Command::new(program)
        .args(arguments)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "dashboard command `{program}` failed with status {status}"
        ))
        .into());
    }
    Ok(())
}

fn handle_scaffold_code<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
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
    ]
    .map(str::to_string);
    let selection = ui.select("Choose component to scaffold:\n", &choices)?;

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
            return run(vec![program.to_string(), "make:scalar".to_string()]);
        }
        8 => {
            return run(vec![program.to_string(), "make:k8s".to_string()]);
        }
        9 => {
            return run(vec![program.to_string(), "make:grpc".to_string()]);
        }
        10 => {
            return run(vec![program.to_string(), "generate:models".to_string()]);
        }
        _ => return Ok(()),
    };

    let name = ui.input(prompt)?;
    let mut args = vec![program.to_string(), action.to_string(), name];
    args.extend(extra_args);
    run(args)
}

fn handle_database_operations<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
    let choices = [
        "🚀  Run Migrations       (cargo rullst db:migrate)",
        "🔄  Rollback Last Batch  (cargo rullst db:rollback)",
        "📊  Migration Status     (cargo rullst db:status)",
        "🌱  Run Seeders          (cargo rullst db:seed)",
        "🖥️  Open Studio Browser  (cargo rullst studio)",
    ]
    .map(str::to_string);
    let selection = ui.select("Choose database operation:\n", &choices)?;
    let cmd = match selection {
        0 => "db:migrate",
        1 => "db:rollback",
        2 => "db:status",
        3 => "db:seed",
        4 => "studio",
        _ => return Ok(()),
    };
    run(vec![program.to_string(), cmd.to_string()])
}

fn handle_auth_billing<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
    let choices = [
        "🔐  Scaffold Full Auth System  (cargo rullst auth)",
        "📲  Scaffold 2FA TOTP System   (cargo rullst make:mfa)",
        "🛡️  Run Security & IDOR Audit  (bounded evidence; optional Geiger)",
        "💳  Scaffold Stripe Billing    (cargo rullst make:billing)",
        "🌐  Add CORS Middleware        (cargo rullst make:cors)",
        "🔑  Add JWT Middleware         (cargo rullst make:jwt)",
    ]
    .map(str::to_string);
    let selection = ui.select("Choose auth & security action:\n", &choices)?;
    let mut args = vec![program.to_string()];
    match selection {
        0 => args.push("auth".to_string()),
        1 => args.push("make:mfa".to_string()),
        2 => {
            args.push("audit".to_string());
            args.push("--ai".to_string());
            args.push("--compliance".to_string());
            args.push("--idor".to_string());
            args.push("--geiger".to_string());
        }
        3 => args.push("make:billing".to_string()),
        4 => args.push("make:cors".to_string()),
        5 => args.push("make:jwt".to_string()),
        _ => return Ok(()),
    };
    run(args)
}

fn handle_deploy<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
    let choices = [
        "🚀  Guided PaaS Deploy         (cargo rullst deploy)",
        "⚙️  Initialize Foundry Config  (cargo rullst foundry:init)",
        "🚀  Deploy via SSH Pipeline    (cargo rullst foundry:deploy)",
    ]
    .map(str::to_string);
    let selection = ui.select("Choose deployment action:\n", &choices)?;
    let cmd = match selection {
        0 => "deploy",
        1 => "foundry:init",
        2 => "foundry:deploy",
        _ => return Ok(()),
    };
    run(vec![program.to_string(), cmd.to_string()])
}

fn handle_existing_project<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
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
            "(Guided PaaS: Fly, Railway, Render, VPS)".dimmed()
        ),
        format!(
            "🔄  Safe Upgrade             {}",
            "(Self-Healing Updates & Codemods)".dimmed()
        ),
        "🔙  Back to Main Menu        ".to_string(),
    ];

    let selection = ui.select("Project Operations:\n", &choices)?;

    match selection {
        0 => run(vec![program.to_string(), "dev".to_string()]),
        1 => run(vec![program.to_string(), "dash".to_string()]),
        2 => handle_scaffold_code(ui, program, run),
        3 => handle_database_operations(ui, program, run),
        4 => handle_auth_billing(ui, program, run),
        5 => run(vec![program.to_string(), "make:omni".to_string()]),
        6 => run(vec![program.to_string(), "dockerize".to_string()]),
        7 => run(vec![program.to_string(), "nixify".to_string()]),
        8 => handle_deploy(ui, program, run),
        9 => run(vec![program.to_string(), "upgrade".to_string()]),
        10 => {
            ui.show_brand()?;
            run_dashboard(ui, program, run)
        }
        _ => Ok(()),
    }
}

fn run_dashboard<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
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

    let selection = ui.select("Navigate with ↑↓, confirm with Enter\n", &choices)?;

    match selection {
        0 => run(vec![program.to_string(), "new".to_string()]),
        1 => handle_existing_project(ui, program, run),
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

pub fn show_interactive_dashboard() -> DashboardResult<()> {
    let program = std::env::args().next().unwrap_or_default();
    let mut ui = DialoguerUi::new();
    ui.show_brand()?;
    run_dashboard(&mut ui, &program, &mut execute_command)
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
