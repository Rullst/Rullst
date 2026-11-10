//! The development `APP_KEY`, kept outside the source tree (v13).
//!
//! Without a configured key, outside staging and production, `get_app_key`
//! uses, in order:
//!
//! 1. a valid `.rullst_dev_key` in the current directory, the file earlier
//!    versions wrote there (read only; it is never created any more);
//! 2. the project's key in the per-user state directory:
//!    `$XDG_STATE_HOME/rullst/dev-keys/<project>.key`, else
//!    `$HOME/.local/state/rullst/dev-keys/<project>.key` (Unix and macOS), or
//!    `%LOCALAPPDATA%\rullst\dev-keys\<project>.key` (Windows), where
//!    `<project>` is the first 32 hex digits of the SHA-256 of the canonical
//!    current directory;
//! 3. a new random key. A test harness keeps it in memory for the process;
//!    any other process writes it to the file of step 2 (owner-only on Unix)
//!    so that sessions survive restarts, or keeps it in memory when no state
//!    directory is available.
//!
//! A test harness is an executable in a directory named `deps`: Cargo places
//! unit, integration-test and benchmark binaries in `target/<profile>/deps`
//! (also under cargo-nextest), while `cargo run` and installed binaries live
//! elsewhere. `cfg(test)` cannot decide this, because it is set only for the
//! crate under test, never for `rullst-auth` as a dependency. So `cargo test`
//! never writes a key, and a key in memory makes every test process use its
//! own: tests that need a stable key across processes set `APP_KEY`.

use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// The file earlier versions wrote into the current directory.
const LEGACY_FILE: &str = ".rullst_dev_key";

/// How a development key was obtained.
pub(super) enum Origin {
    /// Read from `.rullst_dev_key` or the state directory.
    Existing,
    /// Generated and written to this file.
    Persisted(PathBuf),
    /// Generated for this test-harness process only.
    TestHarness,
    /// Generated for this process only: no user state directory is configured.
    NoStateDirectory,
}

/// Resolves the development key; the caller serializes concurrent calls.
/// Exercised in isolated child processes by `tests/app_key_resolution.rs`.
#[cfg_attr(mutants, mutants::skip)]
pub(super) fn resolve() -> std::io::Result<(Vec<u8>, Origin)> {
    if let Some(key) = read_key(Path::new(LEGACY_FILE)) {
        return Ok((key, Origin::Existing));
    }
    let file = std::env::current_dir()
        .and_then(|directory| directory.canonicalize())
        .ok()
        .zip(state_directory(|name| std::env::var_os(name)))
        .map(|(project, directory)| key_file(&directory, &project));
    if let Some(key) = file.as_deref().and_then(read_key) {
        return Ok((key, Origin::Existing));
    }
    let mut key = vec![0u8; 32];
    rand::Rng::fill_bytes(&mut rand::rng(), &mut key);
    let exe = std::env::current_exe().ok();
    if is_test_harness(exe.as_deref()) {
        return Ok((key, Origin::TestHarness));
    }
    let Some(file) = file else {
        return Ok((key, Origin::NoStateDirectory));
    };
    let encoded = general_purpose::STANDARD.encode(&key);
    if let Some(parent) = file.parent() {
        create_private_directory(parent)?;
    }
    match persist(&file, &encoded, false) {
        Ok(()) => Ok((key, Origin::Persisted(file))),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another process published its key first: adopt it when complete,
            // otherwise replace the unusable file.
            if let Some(winner) = read_key(&file) {
                return Ok((winner, Origin::Existing));
            }
            persist(&file, &encoded, true)?;
            Ok((key, Origin::Persisted(file)))
        }
        Err(error) => Err(error),
    }
}

/// `<state>/rullst/dev-keys` from the environment, when absolute.
pub(super) fn state_directory(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |name: &str| {
        var(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let base = if cfg!(windows) {
        absolute("LOCALAPPDATA")?
    } else {
        absolute("XDG_STATE_HOME")
            .or_else(|| Some(absolute("HOME")?.join(".local").join("state")))?
    };
    Some(base.join("rullst").join("dev-keys"))
}

/// The key file of `project` in `directory`.
pub(super) fn key_file(directory: &Path, project: &Path) -> PathBuf {
    let digest = Sha256::digest(project.to_string_lossy().as_bytes());
    let name: String = digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    directory.join(format!("{name}.key"))
}

/// Whether `exe` sits in a Cargo `deps` directory (a test or bench binary).
pub(super) fn is_test_harness(exe: Option<&Path>) -> bool {
    exe.and_then(Path::parent)
        .and_then(Path::file_name)
        .is_some_and(|name| name == "deps")
}

fn read_key(path: &Path) -> Option<Vec<u8>> {
    let encoded = fs::read_to_string(path).ok()?;
    let key = general_purpose::STANDARD.decode(encoded.trim()).ok()?;
    (key.len() == 32).then_some(key)
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

/// Creates the key file, or with `replace` overwrites an unusable one.
fn persist(path: &Path, encoded_key: &str, replace: bool) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true);
    if replace {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    options.open(path)?.write_all(encoded_key.as_bytes())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "dev_key_tests.rs"]
mod tests;
