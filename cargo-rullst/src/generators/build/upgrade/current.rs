//! The Rullst version an application uses before the upgrade, which selects
//! the source rules a plan applies.

use super::manifest::ManifestUpgradePlan;
use semver::Version;
use std::path::Path;

/// The oldest Rullst version that a manifest requirement admits or
/// `Cargo.lock` holds; `None` when no requirement has a lower bound and
/// nothing is locked.
pub(super) fn application_version(
    root: &Path,
    plans: &[ManifestUpgradePlan],
) -> Result<Option<Version>, Box<dyn std::error::Error>> {
    let locked = super::isolated::locked_versions(root)?;
    Ok(plans
        .iter()
        .flat_map(|plan| plan.source_versions.iter().cloned())
        .chain(locked.into_iter().map(|(_, version)| version))
        .min_by(|left, right| left.cmp_precedence(right)))
}

#[cfg(test)]
#[path = "current_tests.rs"]
mod tests;
