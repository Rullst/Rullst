// src/generators/foundry/mod.rs — Rullst Foundry: cloud deployment manifest & SSH pipeline.

mod config;
mod deploy;

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};

fn add_foundry_to_gitignore() -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = std::path::Path::new(".gitignore");
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path)?;
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

fn ensure_is_rullst_project() -> Result<(), IoError> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::NotFound,
            "Foundry commands must run at a Rullst project root containing a Cargo.toml Rullst dependency",
        ));
    }
    Ok(())
}

pub fn scaffold_foundry_config() -> Result<(), Box<dyn std::error::Error>> {
    ensure_is_rullst_project()?;

    let foundry_path = std::path::Path::new("Foundry.toml");
    if foundry_path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            "Foundry.toml already exists; review or move it before re-initializing",
        )
        .into());
    }

    println!(
        "{}",
        "🏭 Initializing Rullst Foundry deployment manifest (Foundry.toml)..."
            .cyan()
            .bold()
    );

    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let cargo_manifest = toml::from_str::<toml::Value>(&cargo_content).map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("Cargo.toml is not valid TOML: {error}"),
        )
    })?;
    let project_name = cargo_manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "Cargo.toml must contain a string package.name",
            )
        })?;

    let foundry_toml = config::generate_foundry_toml_template(project_name);
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
    ensure_is_rullst_project()?;

    let foundry_path = std::path::Path::new("Foundry.toml");
    if !foundry_path.exists() {
        return Err(IoError::new(
            ErrorKind::NotFound,
            "Foundry.toml not found; run `cargo rullst foundry:init` first",
        )
        .into());
    }

    let content = fs::read_to_string(foundry_path)?;
    let cfg = config::parse_foundry_config(&content)?;
    config::validate_foundry_config(&cfg)?;

    deploy::print_deployment_summary(&cfg);

    let ssh_base_args = deploy::get_ssh_base_args(&cfg);

    let local_bin = deploy::execute_build_step(&cfg)?;
    deploy::execute_provision_step(&cfg, &ssh_base_args)?;

    let bin_name = std::path::Path::new(&local_bin)
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "deployment binary path has no file name",
            )
        })?
        .to_string_lossy();
    deploy::execute_upload_step(&cfg, &local_bin)?;
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
        "attempt=0; while [ \"$attempt\" -lt 10 ]; do if curl -fsS --max-time 5 http://localhost:{app_port}/health > /dev/null; then exit 0; fi; attempt=$((attempt + 1)); sleep 2; done; exit 1"
    );
    if !deploy::run_ssh(&health_cmd, &ssh_base_args)? {
        return Err(std::io::Error::other(
            "remote /health probe did not become ready after 10 bounded attempts; deployment not declared successful",
        )
        .into());
    }

    deploy::print_deployment_success(&cfg);
    Ok(())
}
