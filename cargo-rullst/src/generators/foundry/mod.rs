// src/generators/foundry/mod.rs — Rullst Foundry: cloud deployment manifest & SSH pipeline.

mod config;
mod deploy;

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;

fn add_foundry_to_gitignore() -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = std::path::Path::new(".gitignore");
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path).unwrap_or_default();
        if !content.contains("Foundry.toml") {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str("# Rullst Foundry (contains server secrets)\nFoundry.toml\n");
            fs::write(gitignore_path, new_content)?;
            println!(
                "{}",
                "🔒 Automatically added Foundry.toml to .gitignore to protect your secrets."
                    .green()
            );
        }
    }
    Ok(())
}

fn ensure_is_rullst_project() {
    if !is_rullst_project() {
        println!(
            "{}{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold(),
            "\nMake sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }
}

pub fn scaffold_foundry_config() -> Result<(), Box<dyn std::error::Error>> {
    ensure_is_rullst_project();

    let foundry_path = std::path::Path::new("Foundry.toml");
    if foundry_path.exists() {
        println!(
            "{}",
            "⚠️  Foundry.toml already exists. Delete it first to re-initialize."
                .yellow()
                .bold()
        );
        std::process::exit(0);
    }

    println!(
        "{}",
        "🏭 Initializing Rullst Foundry deployment manifest (Foundry.toml)..."
            .cyan()
            .bold()
    );

    let cargo_content = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let project_name = cargo_content
        .lines()
        .find(|l| l.trim_start().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "my-rullst-app".to_string());

    let foundry_toml = config::generate_foundry_toml_template(&project_name);
    fs::write(foundry_path, &foundry_toml)?;

    println!(
        "{}",
        "✅ Foundry.toml generated successfully!".green().bold()
    );
    println!("\n{}", "📋 Next steps:".bold());
    println!(
        "  1. Edit {} with your server IP, domain, and secrets.",
        "Foundry.toml".cyan()
    );
    println!(
        "  2. Add {} to your {} to keep secrets safe.",
        "Foundry.toml".cyan(),
        ".gitignore".yellow()
    );
    println!(
        "  3. Run {} to deploy to your cloud provider.\n",
        "cargo rullst foundry:deploy".magenta().bold()
    );

    add_foundry_to_gitignore()?;
    Ok(())
}

pub fn run_foundry_deploy() -> Result<(), Box<dyn std::error::Error>> {
    ensure_is_rullst_project();

    let foundry_path = std::path::Path::new("Foundry.toml");
    if !foundry_path.exists() {
        println!(
            "{}",
            "❌ Foundry.toml not found. Run 'cargo rullst foundry:init' first."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let content = fs::read_to_string(foundry_path)?;
    let cfg = config::parse_foundry_config(&content);
    config::validate_foundry_config(&cfg);

    deploy::print_deployment_summary(&cfg);

    let ssh_base_args = deploy::get_ssh_base_args(&cfg);

    let local_bin = deploy::execute_build_step(&cfg)?;
    deploy::execute_provision_step(&ssh_base_args)?;

    let bin_name = std::path::Path::new(&local_bin)
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "deployment binary path has no file name",
            )
        })?
        .to_string_lossy();
    deploy::execute_upload_step(&cfg, &local_bin, &ssh_base_args)?;
    deploy::execute_configure_step(&cfg, &bin_name, &ssh_base_args)?;

    println!(
        "{}",
        "🩺 [5/5] Running deployment health check..."
            .bold()
            .yellow()
    );
    let app_port = if cfg.port.is_empty() {
        "3000"
    } else {
        &cfg.port
    };
    let health_cmd = format!(
        "sleep 3 && curl -sf http://localhost:{app_port} > /dev/null && echo '✅ App is responding!' || echo '⚠️  App may still be starting...'"
    );
    let _ = deploy::run_ssh(&health_cmd, &ssh_base_args);

    deploy::print_deployment_success(&cfg);
    Ok(())
}
