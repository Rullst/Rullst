//! User-level AI credentials, kept outside every project and git work tree.
//!
//! Precedence: a provider's environment variable, then the user credentials
//! file (`$XDG_CONFIG_HOME/rullst/credentials.toml`, `~/.config/...` or
//! `%APPDATA%\rullst\credentials.toml`). The file is written atomically with
//! mode 0600 inside a 0700 directory on Unix and is never followed through a
//! symlink. Keys are never printed, logged or included in error messages.

use super::provider::{
    DEFAULT_OLLAMA_HOST, DETECTION_ORDER, Provider, is_mock_credential, valid_model, valid_secret,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const DIRECTORY: &str = "rullst";
const FILE: &str = "credentials.toml";
const MAX_FILE_BYTES: u64 = 16 * 1024;
const HEADER: &str = "# Rullst AI credentials, managed by `cargo rullst ai connect`.\n\
# Keep this file private; never copy it into a project or commit it.\n";

#[derive(Debug, thiserror::Error)]
pub(crate) enum CredentialError {
    #[error(
        "no user configuration directory: set XDG_CONFIG_HOME or HOME (APPDATA on Windows) to an absolute path"
    )]
    NoConfigDir,
    #[error(
        "refusing to use {0}: it resolves inside the current project; set XDG_CONFIG_HOME to a directory outside the project"
    )]
    InsideProject(PathBuf),
    #[error(
        "refusing to write {0}: it is inside a git work tree; set XDG_CONFIG_HOME to a directory outside any repository"
    )]
    InsideGitWorkTree(PathBuf),
    #[error("{0} is not a regular file; remove it and run `cargo rullst ai connect` again")]
    NotRegular(PathBuf),
    #[error("{0} is invalid; run `cargo rullst ai connect` again")]
    Invalid(PathBuf),
    #[error("credentials file I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// A secret that never appears in `Debug` output.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Secret(String);

impl Secret {
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// On-disk format. Unknown keys are rejected so a typo cannot be ignored.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredFile {
    version: u32,
    provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
}

/// Validated stored configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stored {
    pub provider: Provider,
    pub model: Option<String>,
    /// The API key, or the host for Ollama.
    pub secret: Option<Secret>,
}

/// A loaded file plus whether its permissions allow other users to read it.
#[derive(Debug)]
pub(super) struct Loaded {
    pub stored: Stored,
    pub insecure_permissions: bool,
}

/// `%APPDATA%` on Windows, otherwise `$XDG_CONFIG_HOME` or `$HOME/.config`.
/// Relative values are ignored, as the XDG specification requires.
pub(super) fn config_base(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |value: OsString| {
        let path = PathBuf::from(value);
        path.is_absolute().then_some(path)
    };
    if cfg!(windows) {
        return var("APPDATA").and_then(absolute);
    }
    var("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .and_then(absolute)
        .or_else(|| {
            var("HOME")
                .and_then(absolute)
                .map(|home| home.join(".config"))
        })
}

pub(super) fn credentials_path() -> Result<PathBuf, CredentialError> {
    config_base(|name| std::env::var_os(name))
        .map(|base| base.join(DIRECTORY).join(FILE))
        .ok_or(CredentialError::NoConfigDir)
}

/// The canonical form of the deepest existing ancestor of `path`, with the
/// missing remainder appended. Links in the existing part are resolved, so a
/// configuration directory linked into a project is detected.
fn resolved(path: &Path) -> Result<PathBuf, CredentialError> {
    let mut existing = path.to_path_buf();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = existing.file_name().map(OsString::from) else {
                    return Err(error.into());
                };
                missing.push(name);
                if !existing.pop() {
                    return Err(error.into());
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut canonical = fs::canonicalize(&existing)?;
    canonical.extend(missing.iter().rev());
    Ok(canonical)
}

/// Refuses a credentials location inside `project_root`; for writes, also
/// inside any git work tree (a `.git` entry in any ancestor).
pub(super) fn check_location(
    path: &Path,
    project_root: Option<&Path>,
    write: bool,
) -> Result<(), CredentialError> {
    let target = resolved(path)?;
    if let Some(root) = project_root {
        let root = fs::canonicalize(root)?;
        if target.starts_with(&root) {
            return Err(CredentialError::InsideProject(path.to_path_buf()));
        }
    }
    if write
        && target
            .ancestors()
            .skip(1)
            .any(|ancestor| fs::symlink_metadata(ancestor.join(".git")).is_ok())
    {
        return Err(CredentialError::InsideGitWorkTree(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(unix)]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    // Refuse a link swapped in after the metadata check; never block on a FIFO.
    fs::OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_read(path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

#[cfg(unix)]
fn insecure(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o077 != 0
}

#[cfg(not(unix))]
fn insecure(_metadata: &fs::Metadata) -> bool {
    false
}

/// Reads and validates the credentials file; `Ok(None)` when it is absent.
pub(super) fn load(path: &Path) -> Result<Option<Loaded>, CredentialError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Err(CredentialError::NotRegular(path.to_path_buf()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(CredentialError::Invalid(path.to_path_buf()));
    }
    let mut contents = String::new();
    open_for_read(path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|_| CredentialError::Invalid(path.to_path_buf()))?;
    // Parse errors can quote the offending line, so they are never surfaced.
    let invalid = || CredentialError::Invalid(path.to_path_buf());
    let file: StoredFile = toml::from_str(&contents).map_err(|_| invalid())?;
    let provider = Provider::parse(&file.provider).ok_or_else(invalid)?;
    if file.version != 1
        || file
            .model
            .as_deref()
            .is_some_and(|model| !valid_model(model))
        || file
            .api_key
            .as_deref()
            .is_some_and(|key| !valid_secret(key))
        || file.host.as_deref().is_some_and(|host| !valid_secret(host))
        || (provider.uses_api_key() && file.host.is_some())
        || (!provider.uses_api_key() && file.api_key.is_some())
    {
        return Err(invalid());
    }
    let secret = file.api_key.or(file.host).map(Secret::new);
    Ok(Some(Loaded {
        stored: Stored {
            provider,
            model: file.model,
            secret,
        },
        insecure_permissions: insecure(&metadata),
    }))
}

#[cfg(unix)]
fn create_private_directory(directory: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)?;
    // An existing directory keeps its owner but loses group/other access.
    let mode = fs::metadata(directory)?.permissions().mode();
    if mode & 0o077 != 0 {
        fs::set_permissions(directory, fs::Permissions::from_mode(mode & 0o700))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn create_private_directory(directory: &Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)
}

/// Atomically writes `stored` with owner-only permissions.
pub(super) fn save(path: &Path, stored: &Stored) -> Result<(), CredentialError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(CredentialError::NotRegular(path.to_path_buf()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let directory = path
        .parent()
        .ok_or_else(|| CredentialError::Invalid(path.to_path_buf()))?;
    create_private_directory(directory)?;
    let (api_key, host) = match &stored.secret {
        Some(secret) if stored.provider.uses_api_key() => (Some(secret.expose().to_string()), None),
        Some(secret) => (None, Some(secret.expose().to_string())),
        None => (None, None),
    };
    let file = StoredFile {
        version: 1,
        provider: stored.provider.id().to_string(),
        model: stored.model.clone(),
        api_key,
        host,
    };
    let body = toml::to_string(&file).map_err(|_| CredentialError::Invalid(path.to_path_buf()))?;
    // `tempfile` creates the file with mode 0600 on Unix before any byte is written.
    let mut temporary = tempfile::Builder::new()
        .prefix(".credentials-")
        .tempfile_in(directory)?;
    temporary.write_all(HEADER.as_bytes())?;
    temporary.write_all(body.as_bytes())?;
    temporary.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o600))?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

/// Removes the credentials file (a link itself, never its target).
pub(super) fn remove(path: &Path) -> Result<bool, CredentialError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            fs::remove_file(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Where the secret for the active provider came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum KeySource {
    Env(&'static str),
    File,
    /// The default local Ollama host.
    Default,
    Missing,
}

/// The provider configuration a session will use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Resolved {
    /// `None` when nothing is configured: the offline mock answers.
    pub provider: Option<Provider>,
    pub model: String,
    pub model_is_default: bool,
    pub secret: Secret,
    pub source: KeySource,
    pub mock: bool,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid model name; use letters, digits and . - _ : / only")]
pub(crate) struct InvalidModel;

/// Applies the documented precedence. `env` returns trimmed non-empty values.
pub(super) fn resolve(
    flag_provider: Option<Provider>,
    flag_model: Option<&str>,
    stored: Option<&Stored>,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Resolved, InvalidModel> {
    let env = |name: &str| env(name).filter(|value| !value.trim().is_empty());
    if flag_model.is_some_and(|model| !valid_model(model.trim())) {
        return Err(InvalidModel);
    }
    let provider = flag_provider
        .or_else(|| stored.map(|stored| stored.provider))
        .or_else(|| {
            DETECTION_ORDER
                .into_iter()
                .find(|provider| env(provider.env_var()).is_some())
        });
    let Some(provider) = provider else {
        return Ok(Resolved {
            provider: None,
            model: "offline-mock".to_string(),
            model_is_default: true,
            secret: Secret::new(""),
            source: KeySource::Missing,
            mock: true,
        });
    };
    let same = stored.filter(|stored| stored.provider == provider);
    let (secret, source) = if let Some(value) = env(provider.env_var()) {
        (value.trim().to_string(), KeySource::Env(provider.env_var()))
    } else if let Some(secret) = same.and_then(|stored| stored.secret.as_ref()) {
        (secret.expose().to_string(), KeySource::File)
    } else if provider == Provider::Ollama {
        (DEFAULT_OLLAMA_HOST.to_string(), KeySource::Default)
    } else {
        (String::new(), KeySource::Missing)
    };
    let configured = flag_model
        .map(str::to_string)
        .or_else(|| env("RULLST_AI_MODEL"))
        .or_else(|| same.and_then(|stored| stored.model.clone()));
    let model_is_default = configured.is_none();
    let model = configured
        .map(|model| model.trim().to_string())
        .unwrap_or_else(|| provider.default_model().to_string());
    if !valid_model(&model) {
        return Err(InvalidModel);
    }
    Ok(Resolved {
        provider: Some(provider),
        model,
        model_is_default,
        mock: is_mock_credential(&secret),
        secret: Secret::new(secret),
        source,
    })
}

#[cfg(test)]
#[path = "tests/credentials.rs"]
mod tests;
