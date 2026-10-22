//! Removal of the temporary files that a crashed process left behind.
//!
//! A temporary file is removed when its [`super::UploadedFile`] is dropped,
//! which never happens if the process is killed while it exists. Before the
//! first file of a process is written to a temporary directory, and again
//! once a day, [`sweep_stale`] removes from that directory the files that
//! only this module creates and that are older than [`STALE_AFTER`].

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

/// Start of a temporary file name; 32 lowercase hexadecimal digits and
/// [`TEMP_SUFFIX`] follow.
pub(super) const TEMP_PREFIX: &str = ".rullst-upload-";
/// End of a temporary file name.
pub(super) const TEMP_SUFFIX: &str = ".part";

/// Age from which a temporary file is a crash leftover, far beyond any
/// upload's idle timeout, and the wait between two sweeps of one directory.
const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// Bounds on the work of one sweep.
const LIMITS: SweepLimits = SweepLimits {
    max_entries: 10_000,
    max_removals: 1_000,
};

/// Directories whose last sweep is remembered; more distinct directories
/// make the memory start over, which at worst sweeps one again early.
const MAX_TRACKED_DIRS: usize = 256;

static LAST_SWEEPS: Mutex<BTreeMap<PathBuf, Instant>> = Mutex::new(BTreeMap::new());

#[derive(Debug, Clone, Copy)]
struct SweepLimits {
    /// Directory entries examined, matching or not.
    max_entries: usize,
    /// Files removed.
    max_removals: usize,
}

/// What one sweep did.
#[derive(Debug, Default, PartialEq, Eq)]
struct SweepOutcome {
    removed: usize,
    /// Stale files that could not be removed.
    failed: usize,
    /// Why the directory could not be listed.
    error: Option<ErrorKind>,
}

/// Sweeps `dir` unless this process did so in the last [`STALE_AFTER`]. The
/// sweep runs on the blocking thread pool; its failures are logged, never
/// returned, so the upload that triggered it continues.
pub(super) async fn sweep_stale(dir: &Path) {
    let claimed = claim(&mut last_sweeps(), dir, Instant::now());
    if !claimed {
        return;
    }
    let owned = dir.to_path_buf();
    let outcome = tokio::task::spawn_blocking(move || sweep_dir(&owned, SystemTime::now(), LIMITS))
        .await
        .unwrap_or_else(|_| join_failed());
    report(dir, &outcome);
}

/// The outcome reported when the blocking sweep task could not finish.
fn join_failed() -> SweepOutcome {
    SweepOutcome {
        error: Some(ErrorKind::Other),
        ..SweepOutcome::default()
    }
}

fn last_sweeps() -> std::sync::MutexGuard<'static, BTreeMap<PathBuf, Instant>> {
    LAST_SWEEPS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Records a sweep of `dir` at `now`; `false` when one ran less than
/// [`STALE_AFTER`] ago.
fn claim(last_sweeps: &mut BTreeMap<PathBuf, Instant>, dir: &Path, now: Instant) -> bool {
    if last_sweeps
        .get(dir)
        .is_some_and(|last| now.saturating_duration_since(*last) < STALE_AFTER)
    {
        return false;
    }
    if !last_sweeps.contains_key(dir) && last_sweeps.len() >= MAX_TRACKED_DIRS {
        last_sweeps.clear();
    }
    last_sweeps.insert(dir.to_path_buf(), now);
    true
}

/// Removes the stale temporary files of `dir`: regular files (never a
/// symbolic link or a directory) named exactly as [`TEMP_PREFIX`], 32
/// lowercase hexadecimal digits and [`TEMP_SUFFIX`], last modified at least
/// [`STALE_AFTER`] before `now`.
fn sweep_dir(dir: &Path, now: SystemTime, limits: SweepLimits) -> SweepOutcome {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            return SweepOutcome {
                error: Some(error.kind()),
                ..SweepOutcome::default()
            };
        }
    };
    let mut outcome = SweepOutcome::default();
    for entry in entries.take(limits.max_entries).flatten() {
        if outcome.removed >= limits.max_removals {
            break;
        }
        if !is_temp_name(&entry.file_name()) {
            continue;
        }
        let path = entry.path();
        // `symlink_metadata` describes a symbolic link itself.
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.file_type().is_file() || !is_stale(&metadata, now) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => outcome.removed += 1,
            // A concurrent sweep that removed it first is not a failure.
            Err(error) => outcome.failed += usize::from(error.kind() != ErrorKind::NotFound),
        }
    }
    outcome
}

fn is_temp_name(name: &OsStr) -> bool {
    name.to_str()
        .and_then(|name| name.strip_prefix(TEMP_PREFIX))
        .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
        .is_some_and(|id| {
            id.len() == 32
                && id
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
}

fn is_stale(metadata: &std::fs::Metadata, now: SystemTime) -> bool {
    metadata
        .modified()
        .ok()
        .and_then(|modified| now.duration_since(modified).ok())
        .is_some_and(|age| age >= STALE_AFTER)
}

fn report(dir: &Path, outcome: &SweepOutcome) {
    if outcome.error.is_some() || outcome.failed > 0 {
        tracing::warn!(
            target: "rullst::uploads",
            dir = %dir.display(),
            removed = outcome.removed,
            failed = outcome.failed,
            error = ?outcome.error,
            "could not remove every stale multipart temporary file"
        );
    } else if outcome.removed > 0 {
        tracing::info!(
            target: "rullst::uploads",
            dir = %dir.display(),
            removed = outcome.removed,
            "removed stale multipart temporary files"
        );
    }
}

#[cfg(test)]
#[path = "sweep_tests.rs"]
mod tests;
