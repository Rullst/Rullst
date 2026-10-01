//! Remembers the day the opening last animated, so only the first run of a
//! day plays it. The stamp lives in the user's cache directory, never in a
//! project. It is advisory: any failure to read or write it means "do not
//! animate", and nothing here can fail the CLI.

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const DIRECTORY: &str = "rullst-ui-v1";
const FILE: &str = "last-opening";
const STAMP_LIMIT: u64 = 64;

/// The cache base: `%LOCALAPPDATA%` on Windows, otherwise `$XDG_CACHE_HOME`
/// or `$HOME/.cache`. Relative values are ignored, as the XDG spec requires.
fn cache_base(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |value: OsString| {
        let path = PathBuf::from(value);
        path.is_absolute().then_some(path)
    };
    if cfg!(windows) {
        return var("LOCALAPPDATA").and_then(absolute);
    }
    var("XDG_CACHE_HOME")
        .filter(|value| !value.is_empty())
        .and_then(absolute)
        .or_else(|| {
            var("HOME")
                .and_then(absolute)
                .map(|home| home.join(".cache"))
        })
}

/// The stamp file path for the current user, if a cache base is configured.
pub(super) fn marker_path() -> Option<PathBuf> {
    cache_base(|name| std::env::var_os(name)).map(|base| base.join(DIRECTORY).join(FILE))
}

/// Today's local date as the stamp content.
pub(super) fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Claims today's animation: returns `true` only when the stamp did not
/// already hold `today` and was successfully replaced by it.
pub(super) fn claim_daily_animation(path: &Path, today: &str) -> bool {
    match read_stamp(path) {
        Ok(Some(stamp)) if stamp == today => false,
        Ok(_) => write_stamp(path, today).is_ok(),
        Err(_) => false,
    }
}

/// `Ok(None)` when absent; an unusual file type is reported as stale so a
/// fresh stamp replaces it (the rename replaces a link, never its target).
fn read_stamp(path: &Path) -> std::io::Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.len() > STAMP_LIMIT {
        return Ok(None);
    }
    let mut contents = String::new();
    open_for_read(path)?
        .take(STAMP_LIMIT)
        .read_to_string(&mut contents)?;
    Ok(Some(contents.trim().to_string()))
}

#[cfg(unix)]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    // Refuse a link swapped in after the check, and never block on a FIFO.
    fs::OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

fn write_stamp(path: &Path, today: &str) -> std::io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("the opening stamp has no parent directory"))?;
    create_private_directory(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(today.as_bytes())?;
    temporary.write_all(b"\n")?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(unix)]
fn create_private_directory(directory: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    if let Some(base) = directory.parent() {
        fs::create_dir_all(base)?;
    }
    match fs::DirBuilder::new().mode(0o700).create(directory) {
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => Err(error),
        _ => Ok(()),
    }
}

#[cfg(not(unix))]
fn create_private_directory(directory: &Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_run_of_a_day_claims_the_animation_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(DIRECTORY).join(FILE);

        assert!(claim_daily_animation(&path, "2026-10-01"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "2026-10-01\n");
        assert!(!claim_daily_animation(&path, "2026-10-01"));
        assert!(claim_daily_animation(&path, "2026-10-02"));
        assert!(!claim_daily_animation(&path, "2026-10-02"));
    }

    #[test]
    fn unusable_stamps_never_animate_and_never_fail() {
        let directory = tempfile::tempdir().unwrap();
        // The stamp's parent is a file, so it cannot be created.
        let blocker = directory.path().join("blocker");
        fs::write(&blocker, "x").unwrap();
        assert!(!claim_daily_animation(&blocker.join(FILE), "2026-10-01"));

        // A directory in place of the stamp cannot be replaced.
        let occupied = directory.path().join("occupied");
        fs::create_dir_all(occupied.join(FILE)).unwrap();
        assert!(!claim_daily_animation(&occupied.join(FILE), "2026-10-01"));

        // Oversized or corrupt stamps are stale and replaced.
        let stale = directory.path().join("stale");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join(FILE), "x".repeat(1024)).unwrap();
        assert!(claim_daily_animation(&stale.join(FILE), "2026-10-01"));
        assert_eq!(
            fs::read_to_string(stale.join(FILE)).unwrap(),
            "2026-10-01\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_stamp_is_replaced_without_touching_its_target() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target-file");
        fs::write(&target, "2026-10-01").unwrap();
        let marker = directory.path().join(DIRECTORY);
        fs::create_dir_all(&marker).unwrap();
        std::os::unix::fs::symlink(&target, marker.join(FILE)).unwrap();

        assert!(claim_daily_animation(&marker.join(FILE), "2026-10-01"));
        assert_eq!(fs::read_to_string(&target).unwrap(), "2026-10-01");
        assert!(
            !fs::symlink_metadata(marker.join(FILE))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn the_cache_base_follows_platform_conventions() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(*value))
            }
        };
        if cfg!(windows) {
            assert_eq!(
                cache_base(env(&[("LOCALAPPDATA", r"C:\Users\dev\AppData\Local")])),
                Some(PathBuf::from(r"C:\Users\dev\AppData\Local"))
            );
            assert_eq!(cache_base(env(&[("LOCALAPPDATA", "relative")])), None);
            return;
        }
        assert_eq!(
            cache_base(env(&[("XDG_CACHE_HOME", "/xdg"), ("HOME", "/home/dev")])),
            Some(PathBuf::from("/xdg"))
        );
        assert_eq!(
            cache_base(env(&[("XDG_CACHE_HOME", ""), ("HOME", "/home/dev")])),
            Some(PathBuf::from("/home/dev/.cache"))
        );
        assert_eq!(
            cache_base(env(&[
                ("XDG_CACHE_HOME", "relative"),
                ("HOME", "/home/dev")
            ])),
            Some(PathBuf::from("/home/dev/.cache"))
        );
        assert_eq!(cache_base(env(&[("HOME", "relative")])), None);
        assert_eq!(cache_base(env(&[])), None);
    }
}
