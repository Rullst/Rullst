//! The facts about the project the frontend kits depend on.

use std::path::Path;

/// The first `rullst` release with `rullst::assets::url`.
const ASSETS_SINCE: &str = "13.0.0-alpha.5";

/// A Rullst project: a `Cargo.toml` declaring `rullst` under
/// `[dependencies]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Project {
    /// The locked `rullst` (from `Cargo.lock`) has `rullst::assets::url`;
    /// true when nothing older is locked.
    pub supports_assets: bool,
    /// The locked `rullst` version, when one is locked.
    pub locked: Option<String>,
}

impl Project {
    pub(crate) fn read(root: &Path) -> Option<Self> {
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
        let manifest: toml::Table = toml::from_str(&manifest).ok()?;
        manifest.get("dependencies")?.as_table()?.get("rullst")?;
        let locked = std::fs::read_to_string(root.join("Cargo.lock"))
            .ok()
            .and_then(|lock| locked_rullst(&lock));
        Some(Self {
            supports_assets: locked.as_deref().is_none_or(has_assets),
            locked,
        })
    }
}

/// The version of the `rullst` package in a `Cargo.lock`.
pub(crate) fn locked_rullst(lock: &str) -> Option<String> {
    let lock: toml::Table = toml::from_str(lock).ok()?;
    lock.get("package")?
        .as_array()?
        .iter()
        .filter_map(toml::Value::as_table)
        .find(|package| package.get("name").and_then(toml::Value::as_str) == Some("rullst"))?
        .get("version")?
        .as_str()
        .map(str::to_owned)
}

/// Whether `version` is [`ASSETS_SINCE`] or later; an unparsable version
/// counts as recent.
pub(crate) fn has_assets(version: &str) -> bool {
    match (
        semver::Version::parse(version),
        semver::Version::parse(ASSETS_SINCE),
    ) {
        (Ok(version), Ok(since)) => version >= since,
        _ => true,
    }
}
