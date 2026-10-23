//! Free space on the filesystem that holds the project (or the working
//! directory): Rust builds need several GiB for `target/`.

use super::database::human_bytes;
use super::report::Check;
use std::path::Path;

const GIB: u64 = 1024 * 1024 * 1024;
/// Below this a build is likely to fail.
pub(crate) const FAIL_BELOW: u64 = GIB;
/// Below this a clean build may not fit.
pub(crate) const WARN_BELOW: u64 = 5 * GIB;
const FIX: &str = "Free space: run `cargo clean` in projects you are not using, or remove old toolchains (`rustup toolchain list`)";

/// Classifies `available` bytes on the filesystem of `shown`.
pub(crate) fn classify(available: u64, shown: &str) -> Check {
    const ID: &str = "disk.free";
    const TITLE: &str = "Free disk space";
    let detail = format!(
        "{} available on the filesystem of {shown}",
        human_bytes(available)
    );
    if available < FAIL_BELOW {
        Check::fail(ID, TITLE, detail, FIX)
    } else if available < WARN_BELOW {
        Check::warn(ID, TITLE, detail, FIX)
    } else {
        Check::pass(ID, TITLE, detail)
    }
}

#[cfg(unix)]
fn available_bytes(path: &Path) -> Option<u64> {
    let stats = rustix::fs::statvfs(path).ok()?;
    stats.f_bavail.checked_mul(stats.f_frsize)
}

#[cfg(not(unix))]
#[cfg_attr(mutants, mutants::skip)]
fn available_bytes(_path: &Path) -> Option<u64> {
    None
}

pub(crate) fn checks(path: &Path, shown: &str) -> Vec<Check> {
    vec![match available_bytes(path) {
        Some(available) => classify(available, shown),
        None => Check::info(
            "disk.free",
            "Free disk space",
            "not measured on this platform",
        ),
    }]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::generators::doctor::report::Status;

    #[test]
    fn one_and_five_gib_are_the_first_passing_amounts() {
        for (available, status) in [
            (FAIL_BELOW - 1, Status::Fail),
            (FAIL_BELOW, Status::Warn),
            (WARN_BELOW - 1, Status::Warn),
            (WARN_BELOW, Status::Pass),
        ] {
            assert_eq!(classify(available, ".").status, status, "{available}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn free_space_is_measured_on_unix() {
        let directory = tempfile::tempdir().unwrap();
        assert!(available_bytes(directory.path()).unwrap() > 1);
        let checks = checks(directory.path(), "here");
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].id, "disk.free");
        assert!(
            checks[0]
                .detail
                .ends_with(" available on the filesystem of here"),
            "{}",
            checks[0].detail
        );
    }
}
