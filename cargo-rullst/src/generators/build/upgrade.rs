// src/generators/build/upgrade.rs — Validated project dependency upgrade pipeline.

use crate::generators::is_rullst_project;
use crate::ui::spinner::with_spinner;
use colored::*;
use semver::Version;
use std::path::Path;
use std::process::Command;

const RULLST_DEPENDENCIES: &[&str] = &[
    "rullst",
    "rullst-ai",
    "rullst-auth",
    "rullst-capital",
    "rullst-connect",
    "rullst-core",
    "rullst-iot",
    "rullst-macros",
    "rullst-mail",
    "rullst-nexus",
    "rullst-orm",
    "rullst-orm-macros",
    "rullst-security",
    "rullst-studio",
];

#[derive(Debug, thiserror::Error)]
enum UpgradeError {
    #[error("this command must be executed at the root of a Rullst project")]
    NotRullstProject,
    #[error("the installed cargo-rullst version is invalid: {0}")]
    InvalidInstalledVersion(String),
    #[error("Cargo.toml does not contain a standard versioned Rullst dependency")]
    NoManagedDependencies,
    #[error("{command} failed; the upgrade stopped and its diagnostics must be reviewed")]
    CommandFailed { command: &'static str },
}

fn get_cache_path() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("rullst_version_cache.txt");
    dir
}

pub fn run_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(UpgradeError::NotRullstProject.into());
    }

    println!(
        "{}",
        "\n🚀 Starting the validated Rullst project upgrade...\n"
            .cyan()
            .bold()
    );

    let latest_version = target_version()?;

    // Step 1: Update Cargo.toml
    update_cargo_toml(&latest_version.to_string())?;

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
        return Err(UpgradeError::CommandFailed {
            command: "cargo update",
        }
        .into());
    }

    // Step 3: Apply compiler-provided migrations only. Rullst deliberately does
    // not rewrite valid Axum, SQLx, or Tokio imports: those are public escape
    // hatches and global text replacement can silently break application code.
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
        return Err(UpgradeError::CommandFailed {
            command: "cargo fix",
        }
        .into());
    }

    // Step 4: Compiler validation gate
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

    if !check_success {
        return Err(UpgradeError::CommandFailed {
            command: "cargo check",
        }
        .into());
    }

    println!(
        "{}",
        "\n✅ Dependencies were updated and the project passed cargo check. Run the project's full test suite before committing.\n"
            .green()
            .bold()
    );

    Ok(())
}

fn target_version() -> Result<Version, UpgradeError> {
    let installed_text = env!("CARGO_PKG_VERSION");
    let installed = Version::parse(installed_text)
        .map_err(|_| UpgradeError::InvalidInstalledVersion(installed_text.to_string()))?;

    let cached = std::fs::read_to_string(get_cache_path())
        .ok()
        .and_then(|value| Version::parse(value.trim()).ok());

    Ok(cached
        .filter(|candidate| candidate > &installed)
        .unwrap_or(installed))
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
        let cargo_content = std::fs::read_to_string(cargo_path)?;
        let (updated, matched, changed) = update_manifest_content(&cargo_content, latest_version)?;
        if matched == 0 {
            return Err(UpgradeError::NoManagedDependencies.into());
        }
        if changed > 0 {
            std::fs::write(cargo_path, updated)?;
        }
        println!(
            "  {} matched, {} changed; path-only and renamed dependencies were left untouched.",
            matched, changed
        );
    }
    Ok(())
}

fn update_manifest_content(
    cargo_content: &str,
    latest_version: &str,
) -> Result<(String, usize, usize), regex::Error> {
    let names = RULLST_DEPENDENCIES.join("|");
    let inline = regex::Regex::new(&format!(
        r#"(?m)^([ \t]*(?:{names})[ \t]*=[ \t]*\{{[^\r\n}}]*?\bversion[ \t]*=[ \t]*)"([^"]*)""#
    ))?;
    let quoted = regex::Regex::new(&format!(
        r#"(?m)^([ \t]*(?:{names})[ \t]*=[ \t]*)"([^"]*)""#
    ))?;

    let inline_matches = inline.find_iter(cargo_content).count();
    let inline_changed = inline
        .captures_iter(cargo_content)
        .filter(|captures| {
            captures
                .get(2)
                .is_some_and(|value| value.as_str() != latest_version)
        })
        .count();
    let after_inline = inline
        .replace_all(cargo_content, format!(r#"${{1}}"{latest_version}""#))
        .into_owned();
    let quoted_matches = quoted.find_iter(&after_inline).count();
    let quoted_changed = quoted
        .captures_iter(&after_inline)
        .filter(|captures| {
            captures
                .get(2)
                .is_some_and(|value| value.as_str() != latest_version)
        })
        .count();
    let updated = quoted
        .replace_all(&after_inline, format!(r#"${{1}}"{latest_version}""#))
        .into_owned();

    Ok((
        updated,
        inline_matches + quoted_matches,
        inline_changed + quoted_changed,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn updates_all_standard_rullst_dependency_forms_without_rewriting_escape_hatches() {
        let manifest = r#"[dependencies]
rullst = "5.1"
rullst-core = { version = "6", default-features = false }
rullst-ai = { path = "../rullst-ai" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
"#;

        let (updated, matched, changed) =
            update_manifest_content(manifest, "12.0.0-rc.1").expect("valid regex");
        assert_eq!(matched, 2);
        assert_eq!(changed, 2);
        assert!(updated.contains("rullst = \"12.0.0-rc.1\""));
        assert!(updated.contains("rullst-core = { version = \"12.0.0-rc.1\""));
        assert!(updated.contains("rullst-ai = { path = \"../rullst-ai\" }"));
        assert!(updated.contains("axum = \"0.8\""));
        assert!(updated.contains("tokio = { version = \"1\""));
    }

    #[test]
    fn already_current_dependencies_are_matched_without_a_rewrite() {
        let manifest = "[dependencies]\nrullst = \"12.0.0\"\n";
        let (updated, matched, changed) =
            update_manifest_content(manifest, "12.0.0").expect("valid regex");
        assert_eq!(updated, manifest);
        assert_eq!(matched, 1);
        assert_eq!(changed, 0);
    }
}
