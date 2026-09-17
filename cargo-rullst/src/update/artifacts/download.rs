//! A fresh bounded private download, authenticated before fetching executables.
use super::{ArtifactError, files, manifest::Manifest, native_target, provenance};
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

pub(super) fn command() -> Command {
    Command::new("stage")
        .about("Download and authenticate an exact native CLI release without installing it")
        .arg(Arg::new("to").long("to").required(true).value_name("EXACT_VERSION"))
        .arg(Arg::new("allow-major").long("allow-major").action(ArgAction::SetTrue))
        .arg(Arg::new("prerelease").long("prerelease").action(ArgAction::SetTrue))
        .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .after_help("Uses fresh registry metadata and GitHub attestations. No candidate executable is run; installation requires a separate operation and revalidation.")
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    if matches.get_flag("offline")
        || crate::ui::update_check::enabled_env_flag(
            std::env::var_os("CARGO_NET_OFFLINE").as_deref(),
        )
    {
        return Err(ArtifactError::Invalid("offline mode forbids authenticated downloads").into());
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(ArtifactError::Invalid(
            "cannot download synchronously inside an async runtime",
        )
        .into());
    }
    let installed = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let exact = matches
        .get_one::<String>("to")
        .ok_or(ArtifactError::Invalid("exact version required"))?;
    let policy = super::super::catalog::Selection::new(
        &installed,
        Some(exact),
        matches.get_flag("allow-major"),
        matches.get_flag("prerelease"),
    )?;
    let version = Version::parse(exact)?;
    let target = native_target()?;
    // Advisory cache contents never supply authority to download/install.
    let catalog = super::super::fetch_catalog()?;
    let selection = super::super::catalog::resolve(&catalog, &installed, &policy)?;
    let client = client()?;
    let workspace = super::super::cache::project_workspace()?;
    let artifact = stage_into(
        workspace.path(),
        &version,
        target,
        |name, limit, path| fetch(&client, &asset_url(&version, name), limit, path),
        provenance::verify,
    )?;
    let directory = workspace.retain();
    let report = serde_json::json!({"schema_version":"rullst.cli-staging.v1", "directory":directory,
        "artifact":artifact,"registry_selection":selection,"authority":{"artifact_verified":true,
        "registry_eligibility_checked":true,"candidate_executed":false,"cli_installation_authorized":false,
        "project_execution_authorized":false,"project_changes_authorized":false,"deployment_authorized":false}});
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Authenticated CLI {version} for {target} staged at {}",
            directory.display()
        );
        println!(
            "No candidate was executed or installed. Installation must revalidate this release and its files."
        );
    }
    Ok(())
}

fn stage_into(
    directory: &Path,
    version: &Version,
    target: &str,
    mut download: impl FnMut(&str, u64, &Path) -> Result<(), ArtifactError>,
    authenticate: impl FnOnce(&Path, &Manifest) -> Result<(), ArtifactError>,
) -> Result<Manifest, ArtifactError> {
    let name = format!("cli-manifest-{target}.json");
    let path = directory.join(&name);
    download(&name, 16 * 1024, &path)?;
    let body = files::read_bounded(&path, 16 * 1024)?;
    let manifest = Manifest::parse(&body, version, target)?;
    // This directory is newly created with the existing private cache boundary.
    // Same-user/root adversaries are outside that boundary; bytes are rechecked.
    authenticate(&path, &manifest)?;
    if files::read_bounded(&path, 16 * 1024)? != body {
        return Err(ArtifactError::Invalid(
            "manifest changed during authentication",
        ));
    }
    for record in &manifest.files {
        download(&record.name, record.bytes, &directory.join(&record.name))?;
    }
    manifest.verify_files(directory)?;
    Ok(manifest)
}

fn asset_url(version: &Version, name: &str) -> String {
    // All callers use a parsed canonical version and manifest-validated names.
    format!("https://github.com/Rullst/Rullst/releases/download/v{version}/{name}")
}

fn allowed_redirect(url: &reqwest::Url, previous: usize) -> bool {
    previous < 3
        && url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some("github.com" | "release-assets.githubusercontent.com")
        )
}

fn client() -> Result<reqwest::blocking::Client, ArtifactError> {
    reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("cargo-rullst/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if allowed_redirect(attempt.url(), attempt.previous().len()) {
                attempt.follow()
            } else {
                attempt.error("release asset redirect rejected")
            }
        }))
        .build()
        .map_err(|error| ArtifactError::Http(error.without_url()))
}

fn fetch(
    client: &reqwest::blocking::Client,
    url: &str,
    limit: u64,
    path: &Path,
) -> Result<(), ArtifactError> {
    let response = client
        .get(url)
        .send()
        .map_err(|error| ArtifactError::Http(error.without_url()))?;
    if response.status() != reqwest::StatusCode::OK
        || response
            .content_length()
            .is_some_and(|size| size == 0 || size > limit)
    {
        return Err(ArtifactError::Invalid(
            "release asset unavailable or exceeds its declared bound",
        ));
    }
    write_bounded(response, limit, path)
}

fn write_bounded(reader: impl Read, limit: u64, path: &Path) -> Result<(), ArtifactError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    let mut reader = reader.take(limit + 1);
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > limit {
            return Err(ArtifactError::Invalid(
                "release asset exceeds its byte bound",
            ));
        }
        file.write_all(&buffer[..count])?;
    }
    if bytes == 0 {
        return Err(ArtifactError::Invalid("empty release asset"));
    }
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
