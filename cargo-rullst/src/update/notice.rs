//! The decisions behind the daily update notice: whether the cached catalog
//! is still fresh (at most one crates.io request a day, `cache::MAX_AGE_SECONDS`)
//! and whether a newer release may be announced today (at most one notice a
//! day). The catalog lives in the private update cache; the day of the last
//! notice is a stamp next to the opening's in the user cache directory.

use super::cache;
use crate::ui::{opening_marker, update_check};
use semver::Version;

/// The stamp holding the day of the last notice.
const NOTICE_STAMP: &str = "last-update-notice";
/// The stamp holding the day of the last catalog request.
const CHECK_STAMP: &str = "last-update-check";

/// What this process does about the notice.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Start {
    /// A newer release to announce.
    pub announce: Option<Version>,
    /// Whether to fetch the catalog again (it is missing or a day old).
    pub refresh: bool,
}

/// Decides from a fresh cached catalog (`None` when missing or expired).
/// Without one, nothing is announced and the catalog is fetched again only
/// when `claim_check` records the first request of the day. With one, a newer
/// release is announced only when `claim_notice` records the first notice of
/// the day.
pub(crate) fn decide(
    catalog: Option<&[u8]>,
    current: &str,
    claim_notice: impl FnOnce() -> bool,
    claim_check: impl FnOnce() -> bool,
) -> Start {
    let Some(body) = catalog else {
        return Start {
            announce: None,
            refresh: claim_check(),
        };
    };
    let announce = update_check::select_update(body, current)
        .ok()
        .flatten()
        .filter(|_| claim_notice());
    Start {
        announce,
        refresh: false,
    }
}

/// The decision for this process, from the private cache and today's stamp.
pub(crate) fn start() -> Start {
    let cached = cache::load().ok();
    decide(
        cached.as_ref().map(|cached| cached.body.as_slice()),
        env!("CARGO_PKG_VERSION"),
        || claim_today(NOTICE_STAMP),
        || claim_today(CHECK_STAMP),
    )
}

/// Records today in `stamp`; `false` when it already held today or cannot
/// be written.
fn claim_today(stamp: &str) -> bool {
    opening_marker::stamp_path(stamp)
        .is_some_and(|path| opening_marker::claim_daily_animation(&path, &opening_marker::today()))
}

/// Fetches the catalog and keeps it in the private cache. Failures are
/// silent: the next day's first run tries again.
pub(crate) fn refresh() {
    if let Ok(Some(body)) = update_check::fetch_release_catalog() {
        let _ = cache::store(&body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn catalog(versions: &[&str]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"versions": versions.iter().map(|num| {
            serde_json::json!({"num": num, "yanked": false, "crate": "cargo-rullst"})
        }).collect::<Vec<_>>()}))
        .unwrap()
    }

    #[test]
    fn a_missing_or_expired_catalog_is_refreshed_once_a_day_without_a_notice() {
        let claimed = Cell::new(false);
        let start = decide(
            None,
            "13.0.0-alpha.3",
            || {
                claimed.set(true);
                true
            },
            || true,
        );
        assert_eq!(
            start,
            Start {
                announce: None,
                refresh: true
            }
        );
        assert!(!claimed.get(), "nothing to announce, so no notice stamp");
        assert_eq!(
            decide(None, "13.0.0-alpha.3", || true, || false),
            Start {
                announce: None,
                refresh: false
            }
        );
    }

    #[test]
    fn a_newer_release_is_announced_only_with_today_s_claim() {
        let body = catalog(&["13.0.0-alpha.3", "13.0.0-alpha.4", "14.0.0"]);
        let never = || -> bool { unreachable!("a fresh catalog needs no request") };
        let announced = decide(Some(&body), "13.0.0-alpha.3", || true, never);
        assert_eq!(
            announced,
            Start {
                announce: Some(Version::parse("13.0.0-alpha.4").unwrap()),
                refresh: false
            }
        );
        let already = decide(Some(&body), "13.0.0-alpha.3", || false, never);
        assert_eq!(
            already,
            Start {
                announce: None,
                refresh: false
            }
        );
        let claimed = Cell::new(false);
        let current = decide(
            Some(&body),
            "13.0.0-alpha.4",
            || {
                claimed.set(true);
                true
            },
            never,
        );
        assert_eq!(current.announce, None);
        assert!(
            !claimed.get(),
            "an up-to-date CLI never uses the day's notice"
        );
        assert_eq!(
            decide(Some(b"not json"), "13.0.0", || true, never).announce,
            None
        );
    }

    #[test]
    fn the_notice_names_the_exact_install_command() {
        let version = Version::parse("13.0.0-alpha.4").unwrap();
        assert_eq!(
            update_check::notice_text(&version),
            "A newer cargo-rullst is available: cargo install cargo-rullst --version =13.0.0-alpha.4 --locked"
        );
    }
}
