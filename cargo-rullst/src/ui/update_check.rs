// src/ui/update_check.rs — The daily "a newer cargo-rullst is available" notice.
//
// Advisory only: the CLI never downloads, verifies or replaces its own
// binary. In an interactive terminal (not CI, `RULLST_DISABLE_UPDATE_CHECK`
// or `CARGO_NET_OFFLINE`), the crates.io catalog is fetched in the background
// at most once a day and kept in the private user cache; a newer release found
// there is announced at most once a day with the exact install command.

use semver::Version;
use std::io::{IsTerminal, Read};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(crate) const CATALOG_LIMIT: u64 = 256 * 1024;
pub(crate) const CATALOG_URL: &str = "https://crates.io/api/v1/crates/cargo-rullst";
/// The release this process announces, taken by the first caller.
static PENDING: Mutex<Option<Version>> = Mutex::new(None);
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

/// A newer release of the same major worth announcing: a stable release, or
/// any newer release when the installed CLI is itself a prerelease (the user
/// chose that train). Never build metadata or another major.
pub(crate) fn eligible_update(current: &Version, latest: &Version) -> bool {
    latest > current
        && latest.major == current.major
        && (latest.pre.is_empty() || !current.pre.is_empty())
        && latest.build.is_empty()
}

/// The command the notice prints for `version`.
pub(crate) fn install_command(version: &Version) -> String {
    format!("cargo install cargo-rullst --version ={version} --locked")
}

/// The one-line notice for `version`.
pub(crate) fn notice_text(version: &Version) -> String {
    format!(
        "A newer cargo-rullst is available: {}",
        install_command(version)
    )
}

pub(crate) fn select_update(
    reader: impl Read,
    current: &str,
) -> Result<Option<Version>, DiscoveryError> {
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

/// The release to announce in this process, at most once: the first call
/// takes it. `None` when nothing newer is known, the notice was already shown
/// today or the check is silenced.
pub fn check_update_available() -> Option<String> {
    if update_check_disabled() {
        return None;
    }
    PENDING
        .lock()
        .ok()
        .and_then(|mut pending| pending.take())
        .map(|version| version.to_string())
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

/// The catalog body within one total deadline, which also covers a slow
/// trickle of body chunks (a per-read timeout would restart on each chunk).
async fn bounded_fetch(
    client: &reqwest::Client,
    url: &str,
    budget: std::time::Duration,
) -> Result<Option<Vec<u8>>, DiscoveryError> {
    tokio::time::timeout(budget, fetch_catalog(client, url))
        .await
        .map_err(|_| DiscoveryError::Timeout)?
}

#[cfg(test)]
async fn bounded_discovery(
    client: &reqwest::Client,
    url: &str,
    current: &str,
    budget: std::time::Duration,
) -> Result<Option<Version>, DiscoveryError> {
    match bounded_fetch(client, url, budget).await? {
        Some(bytes) => select_update(bytes.as_slice(), current),
        None => Ok(None),
    }
}

/// One bounded crates.io request for the `cargo-rullst` catalog.
pub(crate) fn fetch_release_catalog() -> Result<Option<Vec<u8>>, DiscoveryError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let client = discovery_client().build()?;
        bounded_fetch(&client, CATALOG_URL, std::time::Duration::from_secs(4)).await
    })
}

/// Prepares this process's notice from the cached catalog and, at most once
/// a day, refreshes that catalog in a background thread. Only interactive
/// terminals outside CI take part; nothing here can fail or delay a command,
/// and nothing is installed.
pub fn trigger_background_update_check() {
    if update_check_disabled()
        || !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
        || DISCOVERY_STARTED.swap(true, Ordering::AcqRel)
    {
        return;
    }
    let outcome = crate::update::notice::start();
    if let (Some(version), Ok(mut pending)) = (outcome.announce, PENDING.lock()) {
        *pending = Some(version);
    }
    if outcome.refresh {
        let started = std::thread::Builder::new()
            .name("rullst-update-notice".into())
            .spawn(crate::update::notice::refresh);
        if started.is_err() {
            DISCOVERY_STARTED.store(false, Ordering::Release);
        }
    }
}

/// Prints the notice for `latest_version` when it is a valid release this
/// CLI should announce.
pub fn print_update_banner(latest_version: &str) {
    let (Ok(current), Ok(latest)) = (
        Version::parse(env!("CARGO_PKG_VERSION")),
        Version::parse(latest_version),
    ) else {
        return;
    };
    if eligible_update(&current, &latest) {
        println!("\n  {}\n", notice_text(&latest));
    }
}

/// After a command: prints the pending notice, if any, on standard error.
pub(crate) fn print_pending_notice() {
    if let Some(version) = check_update_available().and_then(|text| Version::parse(&text).ok()) {
        eprintln!("{}", notice_text(&version));
    }
}

#[cfg(test)]
#[path = "update_check_tests.rs"]
mod tests;
