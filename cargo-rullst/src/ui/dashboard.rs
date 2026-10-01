// src/ui/dashboard.rs — Interactive Rullst CLI dashboard (menus, logo, handlers).

use super::dashboard_brand::print_opening;
use super::home::{Home, HomeAction, home_entries, write_next_steps, write_summary};
use super::terminal::TerminalProfile;
use colored::*;
use std::io::Write;
use std::path::Path;

type DashboardResult<T> = Result<T, Box<dyn std::error::Error>>;

trait DashboardUi {
    /// Draws the opening and the context summary for `home`.
    fn show_home(&mut self, home: &Home) -> DashboardResult<()>;
    fn select(&mut self, prompt: &str, choices: &[String]) -> DashboardResult<usize>;
    fn input(&mut self, prompt: &str) -> DashboardResult<String>;
    /// Whether `cargo geiger` runs; the audit item requests it only then,
    /// because a requested but missing Geiger fails the whole audit.
    fn geiger_available(&mut self) -> bool;
}

struct DialoguerUi {
    theme: dialoguer::theme::ColorfulTheme,
    profile: TerminalProfile,
}

impl DialoguerUi {
    fn new(profile: TerminalProfile) -> Self {
        Self {
            theme: dialoguer::theme::ColorfulTheme::default(),
            profile,
        }
    }
}

impl DashboardUi for DialoguerUi {
    fn show_home(&mut self, home: &Home) -> DashboardResult<()> {
        let mut stdout = std::io::stdout();
        write!(stdout, "\x1B[2J\x1B[1;1H")?;
        print_opening(&self.profile, &mut stdout)?;
        write_summary(&mut stdout, home, self.profile.color)?;
        stdout.flush()?;
        if let Some(version) = super::update_check::check_update_available() {
            super::update_check::print_update_banner(&version);
        }
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

    fn geiger_available(&mut self) -> bool {
        std::process::Command::new("cargo")
            .args(["geiger", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

pub fn execute_command(cmd_args: Vec<String>) -> DashboardResult<()> {
    execute_command_in(None, cmd_args)
}

/// Runs a menu command, optionally from `directory` (the project root).
fn execute_command_in(directory: Option<&Path>, cmd_args: Vec<String>) -> DashboardResult<()> {
    let Some((program, arguments)) = cmd_args.split_first() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "dashboard command cannot be empty",
        )
        .into());
    };
    let mut command = std::process::Command::new(program);
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let status = command.status()?;
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
        9 => (
            "Enter gRPC service name (e.g. UserService):",
            "make:grpc",
            vec![],
        ),
        10 => return introspect_models(ui, program, run),
        _ => return Ok(()),
    };

    let name = ui.input(prompt)?;
    let mut args = vec![program.to_string(), action.to_string(), name];
    args.extend(extra_args);
    run(args)
}

/// `generate:models` requires `--driver` and `--url`; collect both first.
fn introspect_models<U, F>(ui: &mut U, program: &str, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
    const DRIVERS: [&str; 3] = ["sqlite", "postgres", "mysql"];
    let choices = DRIVERS.map(str::to_string);
    let Some(driver) = DRIVERS.get(ui.select("Choose the database driver:\n", &choices)?) else {
        return Ok(());
    };
    let url = ui.input("Enter the database connection URL (e.g. sqlite://app.db):")?;
    run(vec![
        program.to_string(),
        "generate:models".to_string(),
        "--driver".to_string(),
        (*driver).to_string(),
        "--url".to_string(),
        url,
    ])
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
            if ui.geiger_available() {
                args.push("--geiger".to_string());
            }
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

fn handle_existing_project<U, F>(
    ui: &mut U,
    program: &str,
    home: &Home,
    run: &mut F,
) -> DashboardResult<()>
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
            ui.show_home(home)?;
            run_dashboard(ui, program, home, run)
        }
        _ => Ok(()),
    }
}

fn run_dashboard<U, F>(ui: &mut U, program: &str, home: &Home, run: &mut F) -> DashboardResult<()>
where
    U: DashboardUi,
    F: FnMut(Vec<String>) -> DashboardResult<()>,
{
    let entries = home_entries(home);
    let choices: Vec<String> = entries.iter().map(|entry| entry.label()).collect();
    let selection = ui.select("Navigate with ↑↓, confirm with Enter\n", &choices)?;
    let Some(entry) = entries.get(selection) else {
        return Ok(());
    };
    let command = |name: &str| vec![program.to_string(), name.to_string()];

    match entry.action {
        HomeAction::Command(name) => run(command(name)),
        HomeAction::Scaffold => handle_scaffold_code(ui, program, run),
        HomeAction::Database => handle_database_operations(ui, program, run),
        HomeAction::Deploy => handle_deploy(ui, program, run),
        HomeAction::ProjectOperations => handle_existing_project(ui, program, home, run),
        HomeAction::NewProject => run(command("new")),
        HomeAction::Help => {
            super::help::show_help_reference();
            Ok(())
        }
        HomeAction::Exit => {
            println!("{}", "Exiting. Happy coding with Rullst! 🦀🚀".dimmed());
            Ok(())
        }
    }
}

/// A relative `argv[0]` with a directory part would break once a command
/// runs from the project root, so it is anchored to `current_dir`.
fn resolve_program(program: &str, current_dir: &Path) -> String {
    let path = Path::new(program);
    if path.is_relative() && path.components().count() > 1 {
        current_dir.join(path).display().to_string()
    } else {
        program.to_string()
    }
}

/// The no-argument entry point: the opening, a context-aware home and the
/// menu. Without a fully interactive terminal (pipes, CI, `TERM=dumb`) it
/// prints the plain home with command equivalents and never prompts.
pub fn show_interactive_dashboard() -> DashboardResult<()> {
    let profile = TerminalProfile::detect();
    let home = Home::detect();
    if !profile.interactive {
        let mut stdout = std::io::stdout();
        print_opening(&profile, &mut stdout)?;
        write_summary(&mut stdout, &home, profile.color)?;
        write_next_steps(&mut stdout, &home)?;
        return Ok(());
    }

    let argv0 = std::env::args().next().unwrap_or_default();
    let program = match std::env::current_dir() {
        Ok(current_dir) => resolve_program(&argv0, &current_dir),
        Err(_) => argv0,
    };
    let mut ui = DialoguerUi::new(profile);
    ui.show_home(&home)?;
    let mut run =
        |arguments: Vec<String>| execute_command_in(home.command_directory(&arguments), arguments);
    run_dashboard(&mut ui, &program, &home, &mut run)
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
