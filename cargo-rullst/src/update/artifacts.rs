//! Read-only candidate verification. A report never authorizes installation.
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use std::path::PathBuf;

#[path = "artifacts/files.rs"]
mod files;
#[path = "artifacts/manifest.rs"]
mod manifest;
#[path = "artifacts/provenance.rs"]
mod provenance;
#[cfg(test)]
#[path = "artifacts/tests.rs"]
mod tests;

#[derive(thiserror::Error)]
pub(super) enum ArtifactError {
    #[error("artifact verification rejected: {0}")]
    Invalid(&'static str),
    #[error("artifact verification I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid artifact manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid artifact selection: {0}")]
    Selection(#[from] super::catalog::SelectionError),
    #[error("private verification staging failed: {0}")]
    Cache(#[from] super::cache::CacheError),
}

impl std::fmt::Debug for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

pub(super) fn command() -> Command {
    Command::new("verify")
        .about("Authenticate downloaded native CLI files without executing or installing them")
        .arg(Arg::new("directory").long("directory").required(true)
            .value_parser(clap::value_parser!(PathBuf)).value_name("PATH"))
        .arg(Arg::new("to").long("to").required(true).value_name("EXACT_VERSION"))
        .arg(Arg::new("allow-major").long("allow-major").action(ArgAction::SetTrue))
        .arg(Arg::new("prerelease").long("prerelease").action(ArgAction::SetTrue))
        .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue)
            .help("Reject before I/O; online GitHub attestation verification is required"))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .after_help("Requires the caller-installed GitHub CLI with attestation verification support. Uses the network and a temporary private manifest. No candidate executable is run. A later installation must revalidate files and release eligibility.")
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), ArtifactError> {
    if matches.get_flag("offline")
        || crate::ui::update_check::enabled_env_flag(
            std::env::var_os("CARGO_NET_OFFLINE").as_deref(),
        )
    {
        return Err(ArtifactError::Invalid(
            "offline mode forbids attestation verification",
        ));
    }
    let text = matches
        .get_one::<String>("to")
        .ok_or(ArtifactError::Invalid("an exact version is required"))?;
    let installed = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(super::catalog::SelectionError::Version)?;
    super::catalog::Selection::new(
        &installed,
        Some(text),
        matches.get_flag("allow-major"),
        matches.get_flag("prerelease"),
    )?;
    let version = Version::parse(text).map_err(super::catalog::SelectionError::Version)?;
    let target = native_target()?;
    let directory = matches
        .get_one::<PathBuf>("directory")
        .ok_or(ArtifactError::Invalid("an artifact directory is required"))?;
    files::directory(directory)?;
    let body = files::read_bounded(
        &directory.join(format!("cli-manifest-{target}.json")),
        16 * 1024,
    )?;
    let manifest = manifest::Manifest::parse(&body, &version, target)?;
    // Authenticate precisely the bytes parsed here, not a subsequently changed
    // file in the caller-selected download directory. Cache permissions apply
    // to this disposable file; cached catalog contents play no part.
    let snapshot = super::cache::verification_manifest(&body)?;
    provenance::verify(snapshot.path(), &manifest)?;
    manifest.verify_files(directory)?;
    let report = Report::new(manifest);
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Verified CLI {} for {} at {}",
            report.artifact.version, report.artifact.target, report.artifact.source_commit
        );
        println!("Publisher: Rullst/Rullst; signer: .github/workflows/release.yml");
        println!(
            "Both binary digests match the authenticated manifest. No candidate binary was executed or installed."
        );
        println!(
            "This report covers the bytes just read; installation must revalidate them and current release eligibility."
        );
    }
    Ok(())
}

fn native_target() -> Result<&'static str, ArtifactError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") if cfg!(target_env = "gnu") => Ok("x86_64-unknown-linux-gnu"),
        ("windows", "x86_64") if cfg!(target_env = "msvc") => Ok("x86_64-pc-windows-msvc"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        _ => Err(ArtifactError::Invalid(
            "no reviewed native CLI artifact for this platform",
        )),
    }
}

#[derive(serde::Serialize)]
struct Report {
    schema_version: &'static str,
    artifact: manifest::Manifest,
    authority: Authority,
}

#[derive(serde::Serialize)]
struct Authority {
    artifact_verified: bool,
    candidate_executed: bool,
    registry_eligibility_checked: bool,
    cli_installation_authorized: bool,
    project_execution_authorized: bool,
    project_changes_authorized: bool,
    deployment_authorized: bool,
}

impl Report {
    fn new(artifact: manifest::Manifest) -> Self {
        Self {
            schema_version: "rullst.update-verification.v1",
            artifact,
            authority: Authority {
                artifact_verified: true,
                candidate_executed: false,
                registry_eligibility_checked: false,
                cli_installation_authorized: false,
                project_execution_authorized: false,
                project_changes_authorized: false,
                deployment_authorized: false,
            },
        }
    }
}
