//! Explicit read-only discovery for the staged 12.1 update experience.
//! Installation and application migration are separate consent boundaries.
use crate::ui::update_check::{self, DiscoveryError};
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use std::time::Duration;

#[path = "update/catalog.rs"]
mod catalog;

#[derive(thiserror::Error)]
enum UpdateError {
    #[error("offline mode forbids registry discovery; no persistent cache is trusted yet")]
    Offline,
    #[error("cannot run synchronous discovery inside an existing async runtime")]
    NestedRuntime,
    #[error("release discovery failed: {0}")]
    Discovery(#[from] DiscoveryError),
    #[error("release catalog request did not return HTTP 200")]
    Unavailable,
    #[error("invalid update selection: {0}")]
    Selection(#[from] catalog::SelectionError),
    #[error("update discovery I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not serialize discovery report: {0}")]
    Json(#[from] serde_json::Error),
}

// The executable returns Result from main, whose Termination implementation
// prints Debug. Preserve the actionable, bounded message at that boundary.
impl std::fmt::Debug for UpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

pub(crate) fn command() -> Command {
    Command::new("update")
        .about("Discover framework updates without silently installing or migrating")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Show an exact CLI release, MSRV and update boundaries; make no changes")
                .arg(Arg::new("to").long("to").value_name("EXACT_VERSION")
                    .help("Inspect this exact release; the default is stable within the current major"))
                .arg(Arg::new("allow-major").long("allow-major").requires("to")
                    .action(ArgAction::SetTrue).help("Explicitly inspect another major; this does not supply its migration rules"))
                .arg(Arg::new("prerelease").long("prerelease").requires("to")
                    .action(ArgAction::SetTrue).help("Allow the exact prerelease selected with --to"))
                .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue)
                    .help("Forbid network access; fail clearly when no trusted cache is available"))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)
                    .help("Emit the versioned rullst.update-discovery.v1 report")),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let Some(matches) = matches.subcommand_matches("check") else {
        return Err("unsupported update operation".into());
    };
    run_check(matches).map_err(Into::into)
}

fn run_check(matches: &ArgMatches) -> Result<(), UpdateError> {
    let installed =
        Version::parse(env!("CARGO_PKG_VERSION")).map_err(catalog::SelectionError::Version)?;
    let policy = catalog::Selection::new(
        &installed,
        matches.get_one::<String>("to").map(String::as_str),
        matches.get_flag("allow-major"),
        matches.get_flag("prerelease"),
    )?;
    if matches.get_flag("offline")
        || update_check::enabled_env_flag(std::env::var_os("CARGO_NET_OFFLINE").as_deref())
    {
        return Err(UpdateError::Offline);
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(UpdateError::NestedRuntime);
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let body = runtime
        .block_on(async {
            let client = update_check::discovery_client()
                .build()
                .map_err(DiscoveryError::Http)?;
            tokio::time::timeout(
                Duration::from_secs(4),
                update_check::fetch_catalog(&client, update_check::CATALOG_URL),
            )
            .await
            .map_err(|_| DiscoveryError::Timeout)?
        })?
        .ok_or(UpdateError::Unavailable)?;
    let report = catalog::resolve(&body, &installed, &policy)?;
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Installed CLI: {}", report.installed);
        if let Some(target) = &report.target {
            println!("Selected CLI: {} ({})", target.version, report.status);
            println!(
                "Required Rust: {}",
                target
                    .rust_version
                    .as_deref()
                    .unwrap_or("not declared; verify before installation")
            );
            println!("Release notes: {}", target.release_notes);
            println!(
                "Current platform: {} / {}",
                report.platform.os, report.platform.arch
            );
            if target.requires_target_major_cli {
                println!(
                    "Major migration: install the target-major CLI and review its supported rules first."
                );
            }
        } else {
            println!("No eligible stable release was found in this major.");
        }
        println!(
            "Discovery only: no CLI or project files changed. Metadata is not artifact verification."
        );
        println!(
            "Guided installation/preparation is still in development. Use the assisted-upgrade guide for the published workflow."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_flags_require_an_exact_target_and_do_not_accept_installation() {
        for args in [
            vec!["update", "check", "--allow-major"],
            vec!["update", "check", "--prerelease"],
            vec!["update", "check", "--install"],
            vec!["update", "apply"],
        ] {
            assert!(command().try_get_matches_from(args).is_err());
        }
        command()
            .try_get_matches_from([
                "update",
                "check",
                "--to",
                "13.0.0-rc.1",
                "--allow-major",
                "--prerelease",
                "--json",
            ])
            .unwrap();
    }

    #[test]
    fn offline_check_never_starts_a_runtime_or_network_request() {
        let matches = command()
            .try_get_matches_from(["update", "check", "--offline"])
            .unwrap();
        let error = run(&matches).unwrap_err().to_string();
        assert!(error.contains("offline mode forbids registry discovery"));
    }
}
