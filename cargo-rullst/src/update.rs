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
#[path = "update/guided.rs"]
mod guided;
#[path = "update/project.rs"]
mod project;

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
        .subcommand(artifacts::stage_command())
        .subcommand(artifacts::install_command())
        .subcommand(project::command())
        .subcommand(guided::command())
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
    if let Some(matches) = matches.subcommand_matches("guided") {
        return guided::run(matches);
    }
    if let Some(matches) = matches.subcommand_matches("install") {
        return artifacts::installation(matches);
    }
    if let Some(matches) = matches.subcommand_matches("stage") {
        return artifacts::stage(matches);
    }
    if let Some(matches) = matches.subcommand_matches("project") {
        return project::run(matches).map_err(Into::into);
    }
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

    #[test]
    fn errors_debug_print_their_actionable_message() {
        assert_eq!(
            format!("{:?}", UpdateError::NestedRuntime),
            "cannot run synchronous discovery inside an existing async runtime"
        );
    }

    #[test]
    fn a_downgrade_check_is_refused_before_discovery() {
        let matches = command()
            .try_get_matches_from(["update", "check", "--to", "0.0.1"])
            .unwrap();
        let error = run(&matches).unwrap_err().to_string();
        assert_eq!(
            error,
            "invalid update selection: downgrades are not authorized by update discovery"
        );
    }

    /// Runs `update <args>` in a child test process with its own environment.
    #[test]
    fn update_child_fixture() {
        let Some(report) = std::env::var_os("RULLST_UPDATE_REPORT") else {
            return;
        };
        let args = std::env::var("RULLST_UPDATE_ARGS").unwrap();
        let matches = command()
            .try_get_matches_from(std::iter::once("update").chain(args.split(' ')))
            .unwrap();
        let text = match run(&matches) {
            Ok(()) => "ok".to_string(),
            Err(error) => format!("err {error}"),
        };
        std::fs::write(report, text).unwrap();
    }

    /// A private cache directory; with `catalog`, a fresh cached catalog.
    #[cfg(unix)]
    fn cache_home(catalog: Option<&str>) -> tempfile::TempDir {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        // The cache refuses a group- or world-writable ancestor.
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let xdg = temp.path().join("xdg");
        std::fs::DirBuilder::new().mode(0o700).create(&xdg).unwrap();
        if let Some(catalog) = catalog {
            let cache = xdg.join("rullst-update-v1");
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&cache)
                .unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(cache.join("catalog-v1"))
                .unwrap();
            std::io::Write::write_all(
                &mut file,
                format!("rullst-update-cache-v1\n{now}\n{catalog}").as_bytes(),
            )
            .unwrap();
        }
        temp
    }

    /// The result and standard output of `update <args>`; network requests go
    /// to a refusing local proxy, so nothing leaves the machine.
    #[cfg(unix)]
    fn update_in_child(home: &tempfile::TempDir, args: &str, offline: bool) -> (String, String) {
        let report = home.path().join("report");
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "update::tests::update_child_fixture",
                "--nocapture",
            ])
            .env("XDG_CACHE_HOME", home.path().join("xdg"))
            .env("RULLST_UPDATE_REPORT", &report)
            .env("RULLST_UPDATE_ARGS", args)
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("https_proxy", "http://127.0.0.1:9")
            .env("ALL_PROXY", "http://127.0.0.1:9")
            .env_remove("NO_PROXY")
            .env_remove("no_proxy")
            .env_remove("CARGO_NET_OFFLINE")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if offline {
            command.env("CARGO_NET_OFFLINE", "true");
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        (
            std::fs::read_to_string(report).unwrap(),
            String::from_utf8(output.stdout).unwrap(),
        )
    }

    #[cfg(unix)]
    #[test]
    fn offline_checks_use_only_a_fresh_private_cache() {
        let catalog = format!(
            "{{\"versions\":[{{\"crate\":\"cargo-rullst\",\"num\":\"{}\",\"yanked\":false,\"rust_version\":\"1.96.0\",\"checksum\":null}}]}}",
            env!("CARGO_PKG_VERSION")
        );
        let cached = cache_home(Some(&catalog));
        let (result, stdout) = update_in_child(&cached, "check --offline --json", false);
        assert_eq!(result, "ok");
        assert!(
            stdout.contains("\"schema_version\": \"rullst.update-discovery.v1\""),
            "{stdout}"
        );
        assert!(
            stdout.contains("\"metadata_source\": \"private-cache\""),
            "{stdout}"
        );

        let empty = cache_home(None);
        let offline = "err offline mode forbids registry discovery: ";
        for (args, variable) in [("check --offline", false), ("check", true)] {
            let (result, _) = update_in_child(&empty, args, variable);
            assert!(result.starts_with(offline), "{args}: {result}");
            assert!(!result.contains("refresh/no-cache"), "{args}: {result}");
        }
        assert_eq!(
            update_in_child(&empty, "check --no-cache", true).0,
            "err offline mode forbids registry discovery: refresh/no-cache requires an online request"
        );
        // Online, a missing cache falls through to the (refused) network request.
        let (result, _) = update_in_child(&empty, "check", false);
        assert!(
            result.starts_with("err release discovery failed"),
            "{result}"
        );
    }
}
