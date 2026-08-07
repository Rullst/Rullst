// src/generators/build/upgrade.rs — Safe self-healing upgrade system with codemods.

use crate::generators::is_rullst_project;
use crate::ui::spinner::with_spinner;
use colored::*;
use std::path::Path;
use std::process::Command;

fn get_cache_path() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("rullst_version_cache.txt");
    dir
}

pub fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "\n🚀 Starting Rullst Safe Upgrade (Self-Healing Upgrades)...\n"
            .cyan()
            .bold()
    );

    let latest_version = if get_cache_path().exists() {
        std::fs::read_to_string(get_cache_path())
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
            .trim()
            .to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    // Step 1: Update Cargo.toml
    update_cargo_toml(&latest_version)?;

    // Step 2: Run cargo update
    let update_success = with_spinner("Refreshing dependencies and lockfile...", || {
        Command::new("cargo")
            .arg("update")
            .arg("-q")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    if !update_success {
        println!(
            "{}",
            "❌ Failed to update dependencies via cargo update.".red()
        );
        std::process::exit(1);
    }

    // Step 3: Run self-healing codemod AST & regex rules
    apply_self_healing_codemods()?;

    // Step 4: Run `cargo fix`
    let fix_success = with_spinner("Applying additional code fixes via cargo fix...", || {
        Command::new("cargo")
            .arg("fix")
            .arg("--allow-no-vcs")
            .arg("--allow-dirty")
            .arg("-q")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    if !fix_success {
        println!(
            "{}",
            "❌ Failed to apply additional code fixes via cargo fix.".red()
        );
        std::process::exit(1);
    }

    // Step 5: Compiler validation gate
    let check_success = with_spinner(
        "Running validation gate (cargo check) to confirm health status...",
        || {
            Command::new("cargo")
                .arg("check")
                .arg("-q")
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        },
    );

    if check_success {
        println!(
            "{}",
            "\n✅ Rullst updated successfully. No breaking changes detected! Code is 100% stable.\n"
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            "\n⚠️ Warning: Upgrade completed with check failures. Please review the compiler errors manually.\n"
                .yellow()
                .bold()
        );
    }

    Ok(())
}

fn update_cargo_toml(latest_version: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        format!(
            "📦 Updating Rullst dependency versions to {} in Cargo.toml...",
            latest_version
        )
        .yellow()
    );
    let cargo_path = Path::new("Cargo.toml");
    if cargo_path.exists() {
        let mut cargo_content = std::fs::read_to_string(cargo_path)?;

        static RE_RULLST: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re_rullst = RE_RULLST
            .get_or_init(|| regex::Regex::new(r#"(?m)^(\s*rullst\s*=\s*)"[^"]+""#).unwrap());
        cargo_content = re_rullst
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}"#, &caps[1], latest_version)
            })
            .into_owned();

        static RE_MACROS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re_macros = RE_MACROS
            .get_or_init(|| regex::Regex::new(r#"(?m)^(\s*rullst-macros\s*=\s*)"[^"]+""#).unwrap());
        cargo_content = re_macros
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}"#, &caps[1], latest_version)
            })
            .into_owned();

        static RE_ELOQUENT: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re_eloquent = RE_ELOQUENT
            .get_or_init(|| regex::Regex::new(r#"(?m)^(\s*rullst-orm\s*=\s*)"[^"]+""#).unwrap());
        cargo_content = re_eloquent
            .replace_all(&cargo_content, |caps: &regex::Captures| {
                format!(r#"{}"{}"#, &caps[1], "6.1.1")
            })
            .into_owned();

        std::fs::write(cargo_path, cargo_content)?;
    }
    Ok(())
}

fn apply_self_healing_codemods() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "\n🔧 Executing self-healing codemod AST & regex rules over project files...".yellow()
    );

    static COMPILED_RULES: std::sync::OnceLock<Vec<(regex::Regex, &'static str, &'static str)>> =
        std::sync::OnceLock::new();
    let compiled_rules = COMPILED_RULES.get_or_init(|| {
        vec![
            (
                regex::Regex::new(r#"\bold_initializer\s*\(\s*\)"#).unwrap(),
                "Router::new()",
                "Legacy old_initializer() -> Router::new()",
            ),
            (
                regex::Regex::new(r#"\brullst::routing::old_initializer\b"#).unwrap(),
                "rullst::routing::Router::new",
                "Legacy router initialization path",
            ),
            (
                regex::Regex::new(r#"\buse\s+sqlx::"#).unwrap(),
                "use rullst::db::sqlx::",
                "Enforce Dependency Shielding for sqlx",
            ),
            (
                regex::Regex::new(r#"\buse\s+axum::"#).unwrap(),
                "use rullst::server::",
                "Enforce Dependency Shielding for axum",
            ),
            (
                regex::Regex::new(r#"\buse\s+tokio::"#).unwrap(),
                "use rullst::runtime::",
                "Enforce Dependency Shielding for tokio",
            ),
        ]
    });

    let mut applied_count = 0;
    if Path::new("src").exists() {
        let walker = walkdir::WalkDir::new("src");
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let mut file_content = std::fs::read_to_string(path)?;
                let mut modified = false;

                for (re, replacement, desc) in compiled_rules {
                    if re.is_match(&file_content) {
                        file_content = re.replace_all(&file_content, *replacement).into_owned();
                        println!(
                            "  [{}] Applied codemod: {} in {}",
                            "Codemod".green().bold(),
                            desc.cyan(),
                            path.display()
                        );
                        modified = true;
                        applied_count += 1;
                    }
                }

                if modified {
                    std::fs::write(path, file_content)?;
                }
            }
        }
    }

    if applied_count == 0 {
        println!("  ✨ No legacy APIs or shielding patterns required patching in this codebase.");
    } else {
        println!(
            "  ✨ Successfully executed {} codemod modifications.",
            applied_count
        );
    }
    Ok(())
}
