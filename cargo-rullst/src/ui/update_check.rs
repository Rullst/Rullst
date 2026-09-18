// src/ui/update_check.rs — Background version check and update banner.

use colored::*;
use semver::Version;
use std::io::{IsTerminal, Read};
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

pub(crate) const CATALOG_LIMIT: u64 = 256 * 1024;
pub(crate) const CATALOG_URL: &str = "https://crates.io/api/v1/crates/cargo-rullst";
// Notices are advisory, not installation authority. Do not trust the legacy
// shared temporary file. Discovery keeps only a process-local validated result;
// a persistent, cross-platform private cache needs its own reviewed contract.
static AVAILABLE_UPDATE: OnceLock<Version> = OnceLock::new();
static DISCOVERY_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub(crate) enum DiscoveryError {
    #[error("release metadata exceeds the discovery limit")]
    TooLarge,
    #[error("could not read release metadata: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid release metadata: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid installed CLI version: {0}")]
    Version(#[from] semver::Error),
    #[error("release metadata request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("release discovery exceeded its total time limit")]
    Timeout,
}

#[derive(serde::Deserialize)]
struct Catalog {
    versions: Vec<Release>,
}

#[derive(serde::Deserialize)]
struct Release {
    #[serde(rename = "crate")]
    package: String,
    num: String,
    yanked: bool,
}

pub(crate) fn enabled_env_flag(value: Option<&std::ffi::OsStr>) -> bool {
    value
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

fn update_check_disabled() -> bool {
    enabled_env_flag(std::env::var_os("RULLST_DISABLE_UPDATE_CHECK").as_deref())
        || enabled_env_flag(std::env::var_os("CARGO_NET_OFFLINE").as_deref())
        || enabled_env_flag(std::env::var_os("CI").as_deref())
}

fn eligible_update(current: &Version, latest: &Version) -> bool {
    latest > current
        && latest.major == current.major
        && latest.pre.is_empty()
        && latest.build.is_empty()
}

fn select_update(reader: impl Read, current: &str) -> Result<Option<Version>, DiscoveryError> {
    let mut body = Vec::new();
    reader.take(CATALOG_LIMIT + 1).read_to_end(&mut body)?;
    if body.len() as u64 > CATALOG_LIMIT {
        return Err(DiscoveryError::TooLarge);
    }
    let current = Version::parse(current)?;
    let catalog: Catalog = serde_json::from_slice(&body)?;
    Ok(catalog
        .versions
        .into_iter()
        .filter(|release| {
            release.package == "cargo-rullst" && !release.yanked && release.num.len() <= 64
        })
        .filter_map(|release| Version::parse(&release.num).ok())
        .filter(|version| eligible_update(&current, version))
        .max())
}

/// Read a completed advisory check without network or filesystem access.
/// The result is only a newer non-yanked stable version in this CLI's major.
pub fn check_update_available() -> Option<String> {
    if update_check_disabled() {
        return None;
    }
    AVAILABLE_UPDATE.get().map(ToString::to_string)
}

pub(crate) fn discovery_client() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .connect_timeout(std::time::Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .user_agent(concat!(
            "cargo-rullst-update-check/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/Rullst/Rullst)"
        ))
}

async fn fetch_update(
    client: &reqwest::Client,
    url: &str,
    current: &str,
) -> Result<Option<Version>, DiscoveryError> {
    match fetch_catalog(client, url).await? {
        Some(bytes) => select_update(bytes.as_slice(), current),
        None => Ok(None),
    }
}

pub(crate) async fn fetch_catalog(
    client: &reqwest::Client,
    url: &str,
) -> Result<Option<Vec<u8>>, DiscoveryError> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    // error_for_status does not reject redirection or empty success responses.
    if response.status() != reqwest::StatusCode::OK {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > CATALOG_LIMIT as usize - bytes.len() {
            return Err(DiscoveryError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Some(bytes))
}

async fn bounded_discovery(
    client: &reqwest::Client,
    url: &str,
    current: &str,
    budget: std::time::Duration,
) -> Result<Option<Version>, DiscoveryError> {
    // One total deadline also covers a slow trickle of body chunks. A blocking
    // Read timeout alone can restart for each read and is not a total deadline.
    tokio::time::timeout(budget, fetch_update(client, url, current))
        .await
        .map_err(|_| DiscoveryError::Timeout)?
}

fn discover_update() -> Result<Option<Version>, DiscoveryError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let client = discovery_client().build()?;
        bounded_discovery(
            &client,
            CATALOG_URL,
            env!("CARGO_PKG_VERSION"),
            std::time::Duration::from_secs(4),
        )
        .await
    })
}

/// Start at most one bounded advisory request in an interactive process.
/// Failure is silent; this function never installs or changes project files.
pub fn trigger_background_update_check() {
    if update_check_disabled()
        || !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || DISCOVERY_STARTED.swap(true, Ordering::AcqRel)
    {
        return;
    }

    let started = std::thread::Builder::new()
        .name("rullst-update-notice".into())
        .spawn(|| {
            if let Ok(Some(version)) = discover_update() {
                let _ = AVAILABLE_UPDATE.set(version);
            }
        });
    if started.is_err() {
        DISCOVERY_STARTED.store(false, Ordering::Release);
    }
}

/// Print a syntactically validated, same-major stable-version notice.
pub fn print_update_banner(latest_version: &str) {
    let current_version = env!("CARGO_PKG_VERSION");
    let (Ok(current), Ok(latest)) = (
        Version::parse(current_version),
        Version::parse(latest_version),
    ) else {
        return;
    };
    if !eligible_update(&current, &latest) {
        return;
    }
    println!();
    println!(
        "{}",
        "┌────────────────────────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    println!(
        "{}  🚀 {} {:<19} {}",
        "│".cyan().bold(),
        "New cargo-rullst available:".bold().yellow(),
        format!("{} → {}", current_version, latest_version)
            .green()
            .bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  Install that exact CLI, then run                     {}",
        "│".cyan().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  {} to migrate the project transactionally. {}",
        "│".cyan().bold(),
        "'cargo rullst upgrade'".magenta().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    println!();
}

#[cfg(test)]
#[path = "update_check_tests.rs"]
mod tests;
