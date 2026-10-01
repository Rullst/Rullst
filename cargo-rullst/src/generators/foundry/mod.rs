// src/generators/foundry/mod.rs — Rullst Foundry: cloud deployment manifest & SSH pipeline.

mod config;
mod deploy;
mod service;

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind, Write};
use std::path::Path;

/// Whether `.gitignore` content ends up ignoring the root `Foundry.toml`.
///
/// Only exact pattern lines count, and the last matching line wins, so a
/// comment, a substring or a later `!Foundry.toml` negation is not mistaken
/// for protection.
fn gitignore_ignores_foundry_manifest(content: &str) -> bool {
    content
        .lines()
        .fold(false, |ignored, line| match line.trim_end() {
            "Foundry.toml" | "/Foundry.toml" => true,
            "!Foundry.toml" | "!/Foundry.toml" => false,
            _ => ignored,
        })
}

/// Ensures `.gitignore` (created when missing) ignores `Foundry.toml`.
fn add_foundry_to_gitignore(gitignore_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = match fs::read_to_string(gitignore_path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    if gitignore_ignores_foundry_manifest(&content) {
        return Ok(());
    }
    let mut new_content = content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str("# Rullst Foundry (contains server secrets)\nFoundry.toml\n");
    fs::write(gitignore_path, new_content)?;
    println!(
        "{}",
        "🔒 Automatically added Foundry.toml to .gitignore to protect your secrets.".green()
    );
    Ok(())
}

/// Creates `Foundry.toml` without replacing an existing entry; on Unix it is
/// readable only by its owner because operators add deployment secrets to it.
fn write_private_manifest(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    if let Err(error) = file.write_all(contents.as_bytes()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
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

    // Protect the manifest before it exists, so a failure never leaves it unignored.
    add_foundry_to_gitignore(Path::new(".gitignore"))?;
    let foundry_toml = config::generate_foundry_toml_template(project_name);
    write_private_manifest(foundry_path, &foundry_toml)?;

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
        "  2. Confirm {} is ignored ({}) and was never committed.",
        "Foundry.toml".cyan(),
        "git check-ignore Foundry.toml".yellow()
    );
    println!(
        "  3. Run {} to deploy to your cloud provider.\n",
        "cargo rullst foundry:deploy".magenta().bold()
    );
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
    let binary_sha256 = deploy::local_binary_sha256(&local_bin)?;
    deploy::execute_upload_step(&cfg, &local_bin)?;
    deploy::execute_configure_step(&cfg, &bin_name, &binary_sha256, &ssh_base_args)?;

    println!(
        "{}",
        "🩺 [5/5] Running deployment health check..."
            .bold()
            .yellow()
    );
    let app_port = cfg.app_port();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_effective_exact_patterns_count_as_ignoring_the_manifest() {
        assert!(gitignore_ignores_foundry_manifest(
            "target/\nFoundry.toml\n"
        ));
        assert!(gitignore_ignores_foundry_manifest("/Foundry.toml"));
        assert!(!gitignore_ignores_foundry_manifest(""));
        assert!(!gitignore_ignores_foundry_manifest(
            "# keep Foundry.toml private\n"
        ));
        assert!(!gitignore_ignores_foundry_manifest(
            "Foundry.toml\n!Foundry.toml\n"
        ));
        assert!(!gitignore_ignores_foundry_manifest(
            "Foundry.toml.example\n"
        ));
        assert!(gitignore_ignores_foundry_manifest(
            "!Foundry.toml\nFoundry.toml\n"
        ));
    }

    #[test]
    fn a_missing_or_negating_gitignore_is_fixed() {
        let directory = tempfile::tempdir().unwrap();
        let gitignore = directory.path().join(".gitignore");
        add_foundry_to_gitignore(&gitignore).unwrap();
        assert!(gitignore_ignores_foundry_manifest(
            &fs::read_to_string(&gitignore).unwrap()
        ));

        fs::write(&gitignore, "target/\n!Foundry.toml").unwrap();
        add_foundry_to_gitignore(&gitignore).unwrap();
        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.starts_with("target/\n!Foundry.toml\n"));
        assert!(gitignore_ignores_foundry_manifest(&content));
        add_foundry_to_gitignore(&gitignore).unwrap();
        assert_eq!(fs::read_to_string(&gitignore).unwrap(), content);
    }

    #[test]
    fn the_manifest_is_private_and_never_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let manifest = directory.path().join("Foundry.toml");
        write_private_manifest(&manifest, "[app]\n").unwrap();
        assert_eq!(
            write_private_manifest(&manifest, "other")
                .unwrap_err()
                .kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(&manifest).unwrap(), "[app]\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&manifest).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
