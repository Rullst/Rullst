//! Owner-only files: created in a `0700` directory with mode `0600` on Unix,
//! written atomically, read with a size bound and never through a symlink.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// Creates `directory` (and its parents) and removes group and other access
/// from it on Unix.
pub(crate) fn create_private_dir(directory: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)?;
        let mode = fs::metadata(directory)?.permissions().mode();
        if mode & 0o077 != 0 {
            fs::set_permissions(directory, fs::Permissions::from_mode(mode & 0o700))?;
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(directory)
    }
}

/// Fails unless `path` is absent or a regular file (not a link).
fn ensure_regular_or_absent(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Atomically replaces `path` with `contents`, readable only by its owner.
pub(crate) fn write_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    ensure_regular_or_absent(path)?;
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent directory"))?;
    create_private_dir(directory)?;
    let suffix = random_suffix()?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("studio");
    let temporary = directory.join(format!(".{file_name}.{suffix}.tmp"));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Reads `path`, refusing links and anything larger than `max_bytes`.
/// `Ok(None)` when it does not exist.
pub(crate) fn read_private(path: &Path, max_bytes: u64) -> io::Result<Option<Vec<u8>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    if metadata.len() > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    let mut contents = Vec::new();
    fs::File::open(path)?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut contents)?;
    if contents.len() as u64 > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(Some(contents))
}

/// Whether group or other users may access `path` (always `false` off Unix).
pub(crate) fn readable_by_others(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o077 != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn random_suffix() -> io::Result<String> {
    let mut bytes = [0_u8; 8];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes)
        .map_err(|_| io::Error::other("operating-system randomness unavailable"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::super::server_test::TempDir;
    use super::*;

    #[test]
    fn temporary_names_take_a_random_hex_suffix() {
        let first = random_suffix().unwrap();
        assert_eq!(first.len(), 16);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert_ne!(first, random_suffix().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn a_path_below_a_file_is_an_error_not_an_absent_file() {
        let directory = TempDir::new("below-file");
        let file = directory.path().join("plain");
        fs::write(&file, b"x").unwrap();
        let below = file.join("projects.json");
        assert_eq!(
            read_private(&below, 64).unwrap_err().kind(),
            io::ErrorKind::NotADirectory
        );
        assert_eq!(
            write_private(&below, b"{}").unwrap_err().kind(),
            io::ErrorKind::NotADirectory
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_and_directories_are_never_written_through() {
        let directory = TempDir::new("write-links");
        let target = directory.path().join("target.json");
        fs::write(&target, b"kept").unwrap();
        let link = directory.path().join("projects.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(
            write_private(&link, b"{}").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&target).unwrap(), b"kept");

        let folder = directory.path().join("folder");
        fs::create_dir(&folder).unwrap();
        assert_eq!(
            write_private(&folder, b"{}").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_directory_loses_group_and_other_access() {
        use std::os::unix::fs::PermissionsExt;
        let directory = TempDir::new("private-dir");
        let open = directory.path().join("open");
        fs::create_dir(&open).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
        create_private_dir(&open).unwrap();
        assert_eq!(
            fs::metadata(&open).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
