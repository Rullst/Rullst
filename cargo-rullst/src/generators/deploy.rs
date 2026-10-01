use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::blueprints::deploy::{
    CADDYFILE, DOCKER_COMPOSE_PROD, FLY_TOML, RAILWAY_JSON, RENDER_YAML,
};

/// A supported deployment target, parsed before anything is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    Fly,
    Railway,
    Render,
    Vps,
}

impl Platform {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "fly" | "fly.io" => Ok(Self::Fly),
            "railway" => Ok(Self::Railway),
            "render" => Ok(Self::Render),
            "vps" => Ok(Self::Vps),
            other => Err(format!(
                "Unknown platform '{other}'. Supported: fly, railway, render, vps"
            )),
        }
    }
}

pub fn run_deploy(platform_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🚀 Rullst Guided Cloud Deployment Wizard".bold().cyan()
    );

    // Reject an unknown platform before the Dockerfile scaffold mutates the project.
    let platform = match platform_arg {
        Some(p) => Platform::parse(p)?,
        None => {
            let theme = dialoguer::theme::ColorfulTheme::default();
            let options = &[
                "Fly.io (Global Edge Containers, automatic SSL & health probes)",
                "Railway (Zero-config deployment, PostgreSQL/Redis plugins)",
                "Render (Managed Cloud Services & Docker)",
                "VPS Production (Docker Compose + Caddy Reverse Proxy with Automatic SSL)",
            ];
            let selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("☁️ Select your Target Deployment Platform")
                .default(0)
                .items(&options[..])
                .interact()?;

            match selection {
                1 => Platform::Railway,
                2 => Platform::Render,
                3 => Platform::Vps,
                _ => Platform::Fly,
            }
        }
    };

    // Extract project name from Cargo.toml if available
    let project_name = get_project_name().unwrap_or_else(|| "rullst_app".to_string());

    // Ensure Dockerfile exists
    if !Path::new("Dockerfile").exists() {
        println!(
            "{}",
            "🐳 Dockerfile missing. Scaffolding optimized multi-stage build...".yellow()
        );
        crate::generators::project::generate_docker_files(
            Path::new("."),
            &project_name,
            None,
            None,
        )?;
    }

    match platform {
        Platform::Fly => deploy_fly(&project_name)?,
        Platform::Railway => deploy_railway(&project_name)?,
        Platform::Render => deploy_render(&project_name)?,
        Platform::Vps => deploy_vps(&project_name)?,
    }

    Ok(())
}

fn get_project_name() -> Option<String> {
    if let Ok(content) = fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            if line.trim().starts_with("name =") {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    return Some(parts[1].trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// Runs a provider CLI and reports whether it was available.
///
/// A missing executable stays advisory because the manifest was still
/// written; a provider command that ran and failed is returned as an error so
/// scripts and CI do not record a failed deployment as successful.
fn run_provider_cli(program: &str, args: &[&str]) -> Result<bool, Box<dyn std::error::Error>> {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => Ok(true),
        Ok(status) => Err(std::io::Error::other(format!(
            "`{program} {}` failed ({status}); the deployment did not complete",
            args.join(" ")
        ))
        .into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn deploy_fly(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let fly_file = "fly.toml";
    let content = FLY_TOML.replace("APP_NAME", project_name);

    if !Path::new(fly_file).exists() {
        fs::write(fly_file, content)?;
        println!("{}", format!("  ✅ Created {}", fly_file).green());
    } else {
        println!("{}", format!("  ℹ️ Existing {} retained.", fly_file).blue());
    }

    println!("{}", "\n🚀 Deploying to Fly.io...".bold().magenta());

    if run_provider_cli("flyctl", &["deploy"])? {
        println!(
            "{}",
            "🎉 Application successfully deployed to Fly.io!"
                .bold()
                .green()
        );
    } else {
        println!(
            "{}",
            "💡 Fly CLI ('flyctl') not found. Execute manually:".yellow()
        );
        println!("   {}", "fly launch".cyan());
        println!("   {}", "fly deploy".cyan());
    }

    Ok(())
}

fn deploy_railway(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let railway_file = "railway.json";
    let content = RAILWAY_JSON.replace("APP_NAME", project_name);

    if !Path::new(railway_file).exists() {
        fs::write(railway_file, content)?;
        println!("{}", format!("  ✅ Created {}", railway_file).green());
    } else {
        println!(
            "{}",
            format!("  ℹ️ Existing {} retained.", railway_file).blue()
        );
    }

    println!("{}", "\n🚀 Deploying to Railway...".bold().magenta());

    if run_provider_cli("railway", &["up"])? {
        println!(
            "{}",
            "🎉 Application successfully deployed to Railway!"
                .bold()
                .green()
        );
    } else {
        println!(
            "{}",
            "💡 Railway CLI ('railway') not found. Execute manually:".yellow()
        );
        println!("   {}", "railway login".cyan());
        println!("   {}", "railway up".cyan());
    }

    Ok(())
}

fn deploy_render(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let render_file = "render.yaml";
    let content = RENDER_YAML.replace("APP_NAME", project_name);

    if !Path::new(render_file).exists() {
        fs::write(render_file, content)?;
        println!("{}", format!("  ✅ Created {}", render_file).green());
    } else {
        println!(
            "{}",
            format!("  ℹ️ Existing {} retained.", render_file).blue()
        );
    }

    println!(
        "{}",
        "\n✨ Render Blueprint generated successfully!"
            .bold()
            .green()
    );
    println!("  Connect your GitHub repository to Render and select 'New Blueprint Instance'.");

    Ok(())
}

fn deploy_vps(_project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let compose_file = "docker-compose.prod.yml";
    let caddy_file = "Caddyfile";

    if !Path::new(compose_file).exists() {
        fs::write(compose_file, DOCKER_COMPOSE_PROD)?;
        println!("{}", format!("  ✅ Created {}", compose_file).green());
    }

    if !Path::new(caddy_file).exists() {
        fs::write(caddy_file, CADDYFILE)?;
        println!("{}", format!("  ✅ Created {}", caddy_file).green());
    }

    println!(
        "{}",
        "\n🔒 VPS Production Infrastructure Provisioned!"
            .bold()
            .green()
    );
    println!("  To launch on your VPS server:");
    println!("   DOMAIN=yourdomain.com docker compose -f docker-compose.prod.yml up -d --build");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn platforms_are_parsed_case_insensitively_and_unknown_ones_rejected() {
        assert_eq!(Platform::parse("Fly.io"), Ok(Platform::Fly));
        assert_eq!(Platform::parse("RAILWAY"), Ok(Platform::Railway));
        assert_eq!(Platform::parse("render"), Ok(Platform::Render));
        assert_eq!(Platform::parse("vps"), Ok(Platform::Vps));
        let error = Platform::parse("flyio").unwrap_err();
        assert!(error.contains("Unknown platform 'flyio'"));
    }
}
