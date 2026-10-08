// cargo-rullst/src/generators/project/create.rs — Writes a planned project, bootstraps
// its database and prints the next steps.

use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

use super::vcs::{self, Vcs};
use super::wizard::plan::ProjectPlan;
use super::wizard::{self, NewProjectRequest, Planned, ProjectWizardOptions};
use super::{ProjectIdentity, cargo_toml, docker, env_config, generate_secure_app_key};
use crate::blueprints::BLANK_BLUEPRINT_ID;
use crate::ui::screen::{Line, Terminal, Tone};

/// Whether writers print their progress lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Report {
    Print,
    Quiet,
}

/// Every file written before the initial migration, in generation order.
pub(crate) fn write_project_files(
    path: &Path,
    plan: &ProjectPlan,
    options: &ProjectWizardOptions,
    identity: &ProjectIdentity,
    current_dir: &Path,
    report: Report,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_name = identity.package_name();
    let db_needed = options.db_needed || options.blueprint_selection != BLANK_BLUEPRINT_ID;
    let cargo_toml_content = cargo_toml::build_cargo_toml(
        project_name,
        options.hot_reload,
        db_needed,
        &options.db_provider,
        &options.polyglot_integrations,
        options.wants_ai,
        options.wants_redis,
        options.blueprint_selection,
        &options.frontend_engine,
        current_dir,
    )?;
    fs::write(path.join("Cargo.toml"), cargo_toml_content)?;

    let app_key = generate_secure_app_key();
    env_config::generate_env_and_configs(
        path,
        db_needed,
        &options.db_provider,
        &options.polyglot_integrations,
        options.blueprint_selection,
        &app_key,
    )?;

    crate::blueprints::apply(
        options.blueprint_selection,
        path,
        project_name,
        identity.module_name(),
        options.api,
        options.hot_reload,
        db_needed,
        &options.orm_pattern,
        &options.frontend_engine,
    )?;

    if plan.docker || plan.buildah {
        let (provider, redis) = (
            Some(options.db_provider.as_str()),
            Some(options.wants_redis),
        );
        match report {
            Report::Print => docker::generate_docker_files(path, project_name, provider, redis)?,
            Report::Quiet => docker::write_docker_files(path, project_name, provider, redis)?,
        }
    }
    if plan.nix {
        match report {
            Report::Print => env_config::generate_nix_files(path)?,
            Report::Quiet => env_config::write_nix_files(path)?,
        }
    }
    match report {
        Report::Print => crate::generators::ai_context::generate_ai_context(Some(path))?,
        Report::Quiet => crate::generators::ai_context::write_project_context(path)?,
    }
    Ok(())
}

/// Files written after the initial migration.
pub(crate) fn write_late_files(
    path: &Path,
    plan: &ProjectPlan,
    identity: &ProjectIdentity,
    report: Report,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.buildah {
        match report {
            Report::Print => env_config::generate_buildah_script(path, identity.package_name())?,
            Report::Quiet => env_config::write_buildah_script(path, identity.package_name())?,
        }
    }
    Ok(())
}

fn bootstrap_database(path: &Path) {
    println!("\n{}", "📦 Bootstrapping Database...".cyan().bold());
    let migration = crate::ui::components::with_spinner(
        "First build + initial migrations (the first compile can take several minutes)...",
        || {
            std::process::Command::new("cargo")
                .arg("run")
                .arg("-q")
                .arg("--")
                .arg("db:migrate")
                .current_dir(path)
                .output()
        },
    );

    match migration {
        Ok(output) if output.status.success() => {
            println!("{}", "  ✅ Database tables created successfully.".green());
        }
        Ok(output) => {
            println!(
                "{}",
                format!(
                    "  ⚠️ Initial migration exited with {}. Project files were kept.",
                    output.status
                )
                .yellow()
            );
            println!(
                "  Configure the selected database, then run: cd {path:?} && cargo run -- db:migrate"
            );
        }
        Err(error) => {
            println!(
                "{}",
                format!("  ⚠️ Could not invoke Cargo for the initial migration: {error}").yellow()
            );
            println!(
                "  Project files were kept. Retry with: cd {path:?} && cargo run -- db:migrate"
            );
        }
    }
}

fn print_dry_run(
    terminal: &Terminal,
    plan: &ProjectPlan,
    reviewed: bool,
    vcs: Vcs,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    if !reviewed {
        let files = wizard::preview::rendered_files(plan);
        let port =
            super::next_steps::resolve_port(std::env::var("PORT").ok().as_deref(), None, None);
        lines.push(
            Line::new()
                .push(Tone::Accent, "◆ ")
                .push(Tone::Brand, wizard::flow::TITLE)
                .push(Tone::Muted, "  dry run"),
        );
        lines.push(Line::new());
        lines.extend(wizard::summary::summary(plan, files.as_deref(), port).lines());
        lines.push(Line::new());
    }
    if vcs == Vcs::Git {
        lines.push(Line::new().push(
            Tone::Muted,
            "Then: git init, unless the destination is inside a Git work tree (--vcs none skips it).",
        ));
    }
    lines.push(Line::new().push(
        Tone::Muted,
        "Dry run: nothing was created. Run the same command without --dry-run to create it.",
    ));
    terminal.print(&lines)?;
    Ok(())
}

/// `cargo rullst new`: plan (flags or wizard), then write, bootstrap and
/// print the next steps. `--dry-run` stops after the plan. A Git repository
/// is initialised as with `--vcs git`.
pub(crate) fn run_new(request: &NewProjectRequest<'_>) -> Result<(), Box<dyn std::error::Error>> {
    run_new_with_vcs(request, Vcs::Git)
}

/// [`run_new`] with an explicit `--vcs` choice.
pub(crate) fn run_new_with_vcs(
    request: &NewProjectRequest<'_>,
    vcs: Vcs,
) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::detect();
    let requested = wizard::requested_integrations(&request.options);
    let (plan, reviewed) = match wizard::plan_project(request, &requested, &terminal, true)? {
        Planned::Ready { plan, reviewed } => (plan, reviewed),
        Planned::Cancelled => {
            terminal.print(&[Line::new().push(Tone::Muted, "Cancelled. Nothing was created.")])?;
            return Ok(());
        }
    };
    let wizard_opts = plan.wizard_options();

    let identity = ProjectIdentity::from_destination(&wizard_opts.name)?;
    let project_name = identity.package_name();
    let blueprint_selection = wizard_opts.blueprint_selection;
    let db_needed = wizard_opts.db_needed || blueprint_selection != BLANK_BLUEPRINT_ID;

    if wizard_opts.db_provider == "Turso" && blueprint_selection != BLANK_BLUEPRINT_ID {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "Turso-primary currently requires the blank starter while the SQLx-specific blueprints are being ported",
        )
        .into());
    }

    let path = identity.destination_path();
    if path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("directory '{}' already exists", path.display()),
        )
        .into());
    }
    if request.dry_run {
        return print_dry_run(&terminal, &plan, reviewed, vcs);
    }

    fs::create_dir_all(path)?;
    let current_dir = std::env::current_dir()?;
    write_project_files(
        path,
        &plan,
        &wizard_opts,
        &identity,
        &current_dir,
        Report::Print,
    )?;
    if db_needed && !request.skip_initial_migration {
        bootstrap_database(path);
    }
    write_late_files(path, &plan, &identity, Report::Print)?;
    if vcs == Vcs::Git && vcs::ensure_gitignore(path)? {
        println!(
            "{}",
            "  ✅ .gitignore (keeps .env and target/ out of Git)".green()
        );
    }
    if let Some(message) = vcs::initialize(path, vcs).message() {
        println!("{message}");
    }

    println!(
        "{}",
        format!("✨ Project '{}' created successfully!", project_name)
            .green()
            .bold()
    );
    let generated_application_profile = if wizard_opts.api {
        "Headless JSON API"
    } else {
        "Zero-Bundle HTMX (html! SSR)"
    };
    println!(
        "{}",
        format!("  Application profile: {generated_application_profile}")
            .white()
            .dimmed()
    );
    if db_needed {
        println!(
            "{}",
            format!("  ORM profile: {}", wizard_opts.orm_pattern)
                .white()
                .dimmed()
        );
    }
    let mut lines = super::next_steps::next_steps(
        path,
        wizard_opts.api,
        db_needed,
        super::next_steps::app_port(path),
    );
    if blueprint_selection == BLANK_BLUEPRINT_ID {
        lines.push(Line::new().push(
            Tone::Muted,
            "  Nexus CMS: not included in the minimal Blank starter. CMS-backed blueprints configure it explicitly.",
        ));
    }
    terminal.print(&lines)?;
    Ok(())
}
