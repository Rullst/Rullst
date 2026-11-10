//! The Tailwind CSS standalone CLI: which release asset a host runs, the
//! per-user cache it is downloaded into, and the checksum verified before
//! every use. Nothing here executes a file whose SHA-256 differs from
//! [`tailwind_release`](super::tailwind_release).

use super::fetch::{FetchError, Fetcher};
use super::tailwind_release::{self, ASSETS, Asset, MAX_ASSET_BYTES, VERSION};
use super::{TAILWIND_INPUT, TAILWIND_OUTPUT};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A platform Tailwind CSS publishes a standalone CLI for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Platform {
    LinuxX64,
    LinuxArm64,
    LinuxX64Musl,
    LinuxArm64Musl,
    MacosX64,
    MacosArm64,
    WindowsX64,
}

impl Platform {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 7] = [
        Self::LinuxX64,
        Self::LinuxArm64,
        Self::LinuxX64Musl,
        Self::LinuxArm64Musl,
        Self::MacosX64,
        Self::MacosArm64,
        Self::WindowsX64,
    ];

    /// The platform this `cargo-rullst` binary was built for.
    pub(crate) fn host() -> Option<Self> {
        Self::for_target(
            std::env::consts::OS,
            std::env::consts::ARCH,
            cfg!(target_env = "musl"),
        )
    }

    /// The asset for an operating system and architecture as
    /// `std::env::consts` spells them. Windows on Arm runs the x64 build
    /// through the system's emulation; Tailwind publishes no Arm build.
    pub(crate) fn for_target(os: &str, arch: &str, musl: bool) -> Option<Self> {
        Some(match (os, arch, musl) {
            ("linux", "x86_64", false) => Self::LinuxX64,
            ("linux", "aarch64", false) => Self::LinuxArm64,
            ("linux", "x86_64", true) => Self::LinuxX64Musl,
            ("linux", "aarch64", true) => Self::LinuxArm64Musl,
            ("macos", "x86_64", _) => Self::MacosX64,
            ("macos", "aarch64", _) => Self::MacosArm64,
            ("windows", "x86_64" | "aarch64", _) => Self::WindowsX64,
            _ => return None,
        })
    }

    /// The pinned release asset of this platform.
    pub(crate) fn asset(self) -> Option<&'static Asset> {
        ASSETS.iter().find(|asset| asset.platform == self)
    }
}

/// Why the Tailwind CLI could not be prepared or run.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ToolError {
    #[error(
        "Tailwind CSS publishes no standalone CLI for {os}/{arch} (it has Linux and macOS x64/arm64 and Windows x64)"
    )]
    UnsupportedPlatform { os: String, arch: String },
    #[error(
        "no per-user cache directory is configured (set XDG_CACHE_HOME or HOME; LOCALAPPDATA on Windows)"
    )]
    NoCacheDirectory,
    #[error("could not {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("downloading the Tailwind CSS CLI failed: {0}")]
    Download(#[from] FetchError),
    #[error(
        "{path} does not have the pinned SHA-256 of tailwindcss v{VERSION} (expected {expected}, found {actual}); it was not run. Delete it to download it again"
    )]
    ChecksumMismatch {
        path: PathBuf,
        expected: &'static str,
        actual: String,
    },
    #[error("the Tailwind CSS CLI exited with {0}")]
    Failed(std::process::ExitStatus),
}

fn io_error<'a>(action: &'static str, path: &'a Path) -> impl FnOnce(io::Error) -> ToolError + 'a {
    move |source| ToolError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

/// The per-user cache base: `XDG_CACHE_HOME` or `~/.cache` on Unix,
/// `LOCALAPPDATA` on Windows; relative values are ignored. `lookup` reads an
/// environment variable.
pub(crate) fn cache_base(
    windows: bool,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    let absolute = |name: &str| {
        lookup(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if windows {
        absolute("LOCALAPPDATA")
    } else {
        absolute("XDG_CACHE_HOME").or_else(|| absolute("HOME").map(|home| home.join(".cache")))
    }
}

/// Where the pinned CLI of `asset` lives below a cache base: a versioned
/// directory, so a bumped pin never reuses an older binary.
pub(crate) fn binary_path(base: &Path, asset: &Asset) -> PathBuf {
    base.join("rullst")
        .join("tailwindcss")
        .join(format!("v{VERSION}"))
        .join(asset.name)
}

/// The SHA-256 of everything `reader` yields, in lowercase hexadecimal.
fn sha256_hex(mut reader: impl Read) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 256 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Checks the file at `path` against the pinned checksum of `asset`.
pub(crate) fn verify(path: &Path, asset: &Asset) -> Result<(), ToolError> {
    let file = std::fs::File::open(path).map_err(io_error("open", path))?;
    let actual = sha256_hex(file).map_err(io_error("read", path))?;
    if actual == asset.sha256 {
        Ok(())
    } else {
        Err(ToolError::ChecksumMismatch {
            path: path.to_path_buf(),
            expected: asset.sha256,
            actual,
        })
    }
}

/// A verified CLI ready to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Verified {
    pub path: PathBuf,
    /// It was downloaded by this call.
    pub downloaded: bool,
}

/// Returns the verified CLI of `asset` below `base`, downloading it with
/// `fetcher` when it is not cached. A cached file that fails the checksum is
/// refused, never replaced silently; a download is written to a temporary
/// file, verified, made executable and only then moved into place.
pub(crate) fn ensure<F: Fetcher>(
    fetcher: &F,
    base: &Path,
    asset: &Asset,
) -> Result<Verified, ToolError> {
    let path = binary_path(base, asset);
    if path.is_file() {
        verify(&path, asset)?;
        return Ok(Verified {
            path,
            downloaded: false,
        });
    }
    let directory = path.parent().unwrap_or(base);
    create_private_directory(directory)?;
    let mut staged = tempfile::Builder::new()
        .prefix(".download-")
        .tempfile_in(directory)
        .map_err(io_error("create a file in", directory))?;
    fetcher.fetch(
        &tailwind_release::download_url(asset),
        MAX_ASSET_BYTES,
        staged.as_file_mut(),
    )?;
    let actual = {
        let file = staged.reopen().map_err(io_error("read", staged.path()))?;
        sha256_hex(file).map_err(io_error("read", staged.path()))?
    };
    if actual != asset.sha256 {
        // Dropping `staged` deletes the unverified bytes.
        return Err(ToolError::ChecksumMismatch {
            path: staged.path().to_path_buf(),
            expected: asset.sha256,
            actual,
        });
    }
    make_executable(staged.path())?;
    staged.persist(&path).map_err(|error| ToolError::Io {
        action: "move the download to",
        path: path.clone(),
        source: error.error,
    })?;
    // Verified again at its final path, as on every later use.
    verify(&path, asset)?;
    Ok(Verified {
        path,
        downloaded: true,
    })
}

#[cfg(unix)]
fn create_private_directory(directory: &Path) -> Result<(), ToolError> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(io_error("create", directory))
}

#[cfg(not(unix))]
fn create_private_directory(directory: &Path) -> Result<(), ToolError> {
    std::fs::create_dir_all(directory).map_err(io_error("create", directory))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), ToolError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(io_error("make executable", path))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), ToolError> {
    Ok(())
}

/// Why the host has no asset.
pub(crate) fn unsupported_host() -> ToolError {
    ToolError::UnsupportedPlatform {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
    }
}

/// The host's asset, or why there is none.
pub(crate) fn host_asset() -> Result<&'static Asset, ToolError> {
    Platform::host()
        .and_then(Platform::asset)
        .ok_or_else(unsupported_host)
}

/// The user's cache base (see [`cache_base`]).
pub(crate) fn user_cache_base() -> Option<PathBuf> {
    cache_base(cfg!(windows), |name| std::env::var_os(name))
}

/// The verified host CLI in the user's cache, downloaded when missing.
pub(crate) fn ensure_host<F: Fetcher>(fetcher: &F) -> Result<Verified, ToolError> {
    let asset = host_asset()?;
    let base = user_cache_base().ok_or(ToolError::NoCacheDirectory)?;
    ensure(fetcher, &base, asset)
}

/// `tailwindcss --input assets/css/app.css --output static/app.css` in
/// `root`, with `--minify` for release builds.
pub(crate) fn build_command(binary: &Path, root: &Path, minify: bool) -> Command {
    let mut command = Command::new(binary);
    command
        .current_dir(root)
        .args(["--input", TAILWIND_INPUT, "--output", TAILWIND_OUTPUT]);
    if minify {
        command.arg("--minify");
    }
    command
}

/// [`build_command`] with `--watch`: rebuilds on changes and exits when its
/// standard input closes, so the dev supervisor keeps that pipe open.
pub(crate) fn watch_command(binary: &Path, root: &Path) -> Command {
    let mut command = build_command(binary, root, false);
    command.arg("--watch").stdin(Stdio::piped());
    command
}

/// Starts `command`, retrying while the kernel still reports the just
/// written executable as busy (`ETXTBSY`: a process forked elsewhere in this
/// process can briefly hold the file's write descriptor).
pub(crate) fn spawn(command: &mut Command) -> io::Result<std::process::Child> {
    let mut retries = 0;
    loop {
        match command.spawn() {
            Err(error) if error.kind() == io::ErrorKind::ExecutableFileBusy && retries < 5 => {
                std::thread::sleep(std::time::Duration::from_millis(10 << retries));
                retries += 1;
            }
            result => return result,
        }
    }
}

/// Runs one build and waits for it.
pub(crate) fn run_build(binary: &Path, root: &Path, minify: bool) -> Result<(), ToolError> {
    let mut command = build_command(binary, root, minify);
    command.stdin(Stdio::null());
    let status = spawn(&mut command)
        .and_then(|mut child| child.wait())
        .map_err(io_error("run", binary))?;
    if status.success() {
        Ok(())
    } else {
        Err(ToolError::Failed(status))
    }
}
