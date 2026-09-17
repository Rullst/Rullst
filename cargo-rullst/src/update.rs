//! Explicit advisory discovery for the staged 12.1 update experience.
//! Installation and application migration are separate consent boundaries.
use crate::ui::update_check::{self, DiscoveryError};
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use std::time::Duration;

#[path = "update/artifacts.rs"]
mod artifacts;
#[path = "update/cache.rs"]
mod cache;
#[path = "update/catalog.rs"]
mod catalog;

#[derive(thiserror::Error)]
enum UpdateError {
    #[error("offline mode forbids registry discovery: {0}")]
    Offline(String),
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
        .subcommand(artifacts::command())
        .subcommand(
            Command::new("check")
                .about("Show an exact CLI release and MSRV without changing the CLI or project")
                .arg(Arg::new("to").long("to").value_name("EXACT_VERSION")
                    .help("Inspect this exact release; the default is stable within the current major"))
                .arg(Arg::new("allow-major").long("allow-major").requires("to")
                    .action(ArgAction::SetTrue).help("Explicitly inspect another major; this does not supply its migration rules"))
                .arg(Arg::new("prerelease").long("prerelease").requires("to")
                    .action(ArgAction::SetTrue).help("Allow the exact prerelease selected with --to"))
                .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue)
                    .help("Use only a fresh private catalog cache; never access the network"))
                .arg(Arg::new("refresh").long("refresh").action(ArgAction::SetTrue)
                    .conflicts_with("offline").help("Bypass cached discovery and refresh from the registry"))
                .arg(Arg::new("no-cache").long("no-cache").action(ArgAction::SetTrue)
                    .conflicts_with("offline").help("Do not read or write persistent discovery metadata"))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)
                    .help("Emit the versioned rullst.update-discovery.v1 report")),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(matches) = matches.subcommand_matches("verify") {
        return artifacts::run(matches).map_err(Into::into);
    }
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
    let report = discover(matches, &installed, &policy)?;
    print_report(matches, &report)
}

fn discover(
    matches: &ArgMatches,
    installed: &Version,
    policy: &catalog::Selection,
) -> Result<catalog::Report, UpdateError> {
    let offline = matches.get_flag("offline")
        || update_check::enabled_env_flag(std::env::var_os("CARGO_NET_OFFLINE").as_deref());
    let use_cache = !matches.get_flag("no-cache");
    if use_cache && !matches.get_flag("refresh") {
        let cached = cache::load()
            .map_err(|error| error.to_string())
            .and_then(|cached| {
                // Revalidate every byte, identity and current selection policy. A
                // cached report/previous selection is never deserialized as authority.
                catalog::resolve(&cached.body, installed, policy)
                    .map(|mut report| {
                        report.metadata_source = "private-cache";
                        report.metadata_age_seconds = cached.age_seconds;
                        report
                    })
                    .map_err(|error| error.to_string())
            });
        match cached {
            Ok(report) => return Ok(report),
            Err(error) if offline => return Err(UpdateError::Offline(error)),
            Err(_) => {} // Online discovery can recover from a missing/invalid cache.
        }
    } else if offline {
        return Err(UpdateError::Offline(
            "refresh/no-cache requires an online request".into(),
        ));
    }
    let body = fetch_catalog()?;
    let report = catalog::resolve(&body, installed, policy)?;
    if use_cache && let Err(error) = cache::store(&body) {
        // Cache failure must not obscure a successfully validated network result.
        eprintln!(
            "Note: release discovery succeeded but its advisory cache was not saved: {error}"
        );
    }
    Ok(report)
}

fn fetch_catalog() -> Result<Vec<u8>, UpdateError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(UpdateError::NestedRuntime);
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime
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
        .ok_or(UpdateError::Unavailable)
}

fn print_report(matches: &ArgMatches, report: &catalog::Report) -> Result<(), UpdateError> {
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Installed CLI: {}", report.installed);
        println!(
            "Metadata: {} ({} seconds old)",
            report.metadata_source, report.metadata_age_seconds
        );
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
            "Discovery only: no CLI or project files changed. Cached metadata is advisory, not artifact verification."
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
    fn explicit_offline_mode_conflicts_with_online_only_cache_options() {
        // Explicit incompatible options are rejected before filesystem/network I/O.
        for flag in ["--refresh", "--no-cache"] {
            assert!(
                command()
                    .try_get_matches_from(["update", "check", "--offline", flag])
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn nested_runtime_is_rejected_before_network_discovery() {
        assert!(matches!(fetch_catalog(), Err(UpdateError::NestedRuntime)));
    }
}
