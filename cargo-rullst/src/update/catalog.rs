use semver::Version;
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub(super) enum SelectionError {
    #[error("invalid version: {0}")]
    Version(#[from] semver::Error),
    #[error("invalid registry catalog: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(&'static str),
}

pub(super) struct Selection {
    exact: Option<Version>,
    allow_prerelease: bool,
}

impl Selection {
    pub(super) fn new(
        installed: &Version,
        exact: Option<&str>,
        allow_major: bool,
        allow_prerelease: bool,
    ) -> Result<Self, SelectionError> {
        let exact = exact.map(parse_version).transpose()?;
        if let Some(target) = &exact {
            if target < installed {
                return Err(SelectionError::Invalid(
                    "downgrades are not authorized by update discovery",
                ));
            }
            if target.major != installed.major && !allow_major {
                return Err(SelectionError::Invalid(
                    "another major requires --allow-major and its own migration rules",
                ));
            }
            if !target.pre.is_empty() && !allow_prerelease {
                return Err(SelectionError::Invalid(
                    "a prerelease requires --prerelease",
                ));
            }
        } else if allow_major || allow_prerelease {
            return Err(SelectionError::Invalid(
                "major/prerelease inspection requires --to EXACT_VERSION",
            ));
        }
        Ok(Self {
            exact,
            allow_prerelease,
        })
    }
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
    rust_version: Option<String>,
    checksum: Option<String>,
}

#[derive(serde::Serialize)]
pub(super) struct Report {
    schema_version: &'static str,
    pub installed: String,
    pub status: &'static str,
    pub target: Option<Target>,
    pub platform: Platform,
    network_source: &'static str,
    pub metadata_source: &'static str,
    pub metadata_age_seconds: u64,
    cache_max_age_seconds: u64,
    authority: Authority,
}

#[derive(serde::Serialize)]
pub(super) struct Target {
    pub version: String,
    pub rust_version: Option<String>,
    registry_checksum: Option<String>,
    pub release_notes: String,
    pub requires_target_major_cli: bool,
}

#[derive(serde::Serialize)]
pub(super) struct Platform {
    pub os: &'static str,
    pub arch: &'static str,
}

#[derive(serde::Serialize)]
struct Authority {
    artifact_verified: bool,
    cli_installation_authorized: bool,
    project_execution_authorized: bool,
    project_changes_authorized: bool,
    deployment_authorized: bool,
}

pub(super) fn resolve(
    body: &[u8],
    installed: &Version,
    selection: &Selection,
) -> Result<Report, SelectionError> {
    if body.len() as u64 > crate::ui::update_check::CATALOG_LIMIT {
        return Err(SelectionError::Invalid("catalog exceeds 256 KiB"));
    }
    let catalog: Catalog = serde_json::from_slice(body)?;
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    for release in catalog.versions {
        if release.package != "cargo-rullst" {
            continue;
        }
        let Ok(version) = parse_version(&release.num) else {
            continue;
        };
        if !seen.insert(version.clone()) {
            return Err(SelectionError::Invalid(
                "conflicting or duplicate release entries",
            ));
        }
        if release.yanked || (!selection.allow_prerelease && !version.pre.is_empty()) {
            continue;
        }
        let eligible = match &selection.exact {
            Some(exact) => version == *exact,
            None => version.major == installed.major && version >= *installed,
        };
        if eligible {
            candidates.push((version, release));
        }
    }
    let candidate = candidates
        .into_iter()
        .max_by(|(left, _), (right, _)| left.cmp(right));
    if selection.exact.is_some() && candidate.is_none() {
        return Err(SelectionError::Invalid(
            "the exact release is unavailable, yanked or disallowed",
        ));
    }
    let status = match &candidate {
        Some((version, _)) if version == installed => "already-current",
        Some(_) => "update-available",
        None => "no-eligible-release",
    };
    let target = candidate
        .map(|(version, release)| -> Result<Target, SelectionError> {
            let rust_version = release
                .rust_version
                .as_deref()
                .map(parse_rust_version)
                .transpose()?;
            if release.checksum.as_deref().is_some_and(|checksum| {
                checksum.len() != 64 || !checksum.bytes().all(|c| c.is_ascii_hexdigit())
            }) {
                return Err(SelectionError::Invalid("malformed registry checksum"));
            }
            Ok(Target {
                version: version.to_string(),
                rust_version,
                registry_checksum: release
                    .checksum
                    .map(|checksum| checksum.to_ascii_lowercase()),
                release_notes: format!("https://github.com/Rullst/Rullst/releases/tag/v{version}"),
                requires_target_major_cli: version.major != installed.major,
            })
        })
        .transpose()?;
    Ok(Report {
        schema_version: "rullst.update-discovery.v1",
        installed: installed.to_string(),
        status,
        target,
        platform: Platform {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        network_source: crate::ui::update_check::CATALOG_URL,
        metadata_source: "registry",
        metadata_age_seconds: 0,
        cache_max_age_seconds: super::cache::MAX_AGE_SECONDS,
        authority: Authority {
            artifact_verified: false,
            cli_installation_authorized: false,
            project_execution_authorized: false,
            project_changes_authorized: false,
            deployment_authorized: false,
        },
    })
}

fn parse_version(text: &str) -> Result<Version, SelectionError> {
    if text.len() > 64 || text.chars().any(char::is_control) {
        return Err(SelectionError::Invalid("unsafe version string"));
    }
    let version = Version::parse(text)?;
    if !version.build.is_empty() {
        return Err(SelectionError::Invalid(
            "build metadata is not a supported release identity",
        ));
    }
    Ok(version)
}

fn parse_rust_version(text: &str) -> Result<String, SelectionError> {
    let normalized = if text.split('.').count() == 2 {
        format!("{text}.0")
    } else {
        text.to_owned()
    };
    let version = parse_version(&normalized)?;
    if version.major != 1 || !version.pre.is_empty() {
        return Err(SelectionError::Invalid(
            "unsupported Rust version requirement",
        ));
    }
    Ok(version.to_string())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
