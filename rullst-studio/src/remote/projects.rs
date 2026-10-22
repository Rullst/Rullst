//! The project list of the standalone Studio: name, base URL and encrypted
//! monitor token per project, in an owner-only JSON file.

use super::crypto::{Secret, SecretKey};
use super::paths::StudioPaths;
use super::secure_file;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;
use std::sync::Mutex;

/// Most projects the list holds.
pub const MAX_PROJECTS: usize = 32;
const MAX_FILE_BYTES: u64 = 256 * 1024;
const MAX_NAME_CHARS: usize = 48;
const MAX_URL_CHARS: usize = 512;
const MAX_TOKEN_CHARS: usize = 512;
/// The phase 1 monitoring document, relative to a project's base URL.
const MONITOR_PATH: &str = "_rullst/monitor/v1";

/// Why the project list could not be read or changed. Messages never contain
/// a token, a URL or file contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectError {
    /// The project list or its key could not be read or written.
    Storage,
    /// The project list file is malformed; fix or remove it.
    InvalidFile,
    /// A name is not 1 to 48 letters, digits, `.`, `_` or `-`, starting with a
    /// letter or digit.
    InvalidName,
    /// A base URL is not `https://…`, or `http://` to a loopback address, without
    /// user information, query or fragment.
    InvalidUrl,
    /// A token is empty, too long or contains spaces or control characters.
    InvalidToken,
    /// A project with this name already exists.
    Duplicate,
    /// The list already holds [`MAX_PROJECTS`] projects.
    TooMany,
    /// No project has this name.
    NotFound,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Storage => "the project list or its key could not be read or written",
            Self::InvalidFile => "the project list file is malformed; fix or remove it",
            Self::InvalidName => {
                "use 1 to 48 letters, digits, '.', '_' or '-', starting with a letter or digit"
            }
            Self::InvalidUrl => {
                "use an https:// URL, or http:// to a loopback address (an SSH tunnel), without user, query or fragment"
            }
            Self::InvalidToken => "the token must be 1 to 512 visible characters",
            Self::Duplicate => "a project with this name already exists",
            Self::TooMany => "the project list is full (32 projects)",
            Self::NotFound => "no project has this name",
        })
    }
}

impl std::error::Error for ProjectError {}

/// One monitored application.
#[derive(Clone, Debug)]
pub struct Project {
    /// The project's name.
    pub name: String,
    /// The base URL; the document is read from `<base>_rullst/monitor/v1`.
    pub base_url: Url,
    /// The decrypted token, or `None` when it cannot be decrypted (missing or
    /// replaced key, edited name or URL).
    pub token: Option<Secret>,
}

impl Project {
    /// The URL of the project's monitoring document.
    pub fn monitor_url(&self) -> Option<Url> {
        self.base_url.join(MONITOR_PATH).ok()
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct StoredFile {
    version: u32,
    projects: Vec<StoredProject>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct StoredProject {
    name: String,
    base_url: String,
    /// The token sealed with [`SecretKey::seal`].
    token: String,
}

/// The associated data binding a sealed token to its project.
pub(crate) fn token_context(name: &str, base_url: &str) -> Vec<u8> {
    format!("rullst-studio/token\n{name}\n{base_url}").into_bytes()
}

/// Validates a project name.
pub fn validate_name(name: &str) -> Result<(), ProjectError> {
    let valid = !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    valid.then_some(()).ok_or(ProjectError::InvalidName)
}

/// Validates and normalizes a base URL: `https`, or `http` to a loopback
/// host; no user information, query or fragment; the path ends with `/`.
pub fn validate_base_url(value: &str) -> Result<Url, ProjectError> {
    let value = value.trim();
    if value.len() > MAX_URL_CHARS {
        return Err(ProjectError::InvalidUrl);
    }
    let mut url = Url::parse(value).map_err(|_| ProjectError::InvalidUrl)?;
    let host = url.host_str().ok_or(ProjectError::InvalidUrl)?;
    let ip_host = host.trim_start_matches('[').trim_end_matches(']');
    let loopback = host.eq_ignore_ascii_case("localhost")
        || ip_host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.to_canonical().is_loopback());
    let scheme_allowed = match url.scheme() {
        "https" => true,
        "http" => loopback,
        _ => false,
    };
    if !scheme_allowed
        || url.cannot_be_a_base()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ProjectError::InvalidUrl);
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

fn validate_token(token: &str) -> Result<(), ProjectError> {
    let valid = !token.is_empty()
        && token.len() <= MAX_TOKEN_CHARS
        && token.bytes().all(|byte| byte.is_ascii_graphic());
    valid.then_some(()).ok_or(ProjectError::InvalidToken)
}

/// The project list on disk. Changes are serialized within the process.
pub struct ProjectStore {
    paths: StudioPaths,
    write: Mutex<()>,
}

impl ProjectStore {
    /// A store over `paths`.
    pub fn new(paths: StudioPaths) -> Self {
        Self {
            paths,
            write: Mutex::new(()),
        }
    }

    fn read_file(&self) -> Result<StoredFile, ProjectError> {
        let bytes = secure_file::read_private(&self.paths.projects, MAX_FILE_BYTES)
            .map_err(|_| ProjectError::Storage)?;
        let Some(bytes) = bytes else {
            return Ok(StoredFile {
                version: 1,
                projects: Vec::new(),
            });
        };
        let file: StoredFile =
            serde_json::from_slice(&bytes).map_err(|_| ProjectError::InvalidFile)?;
        let names_valid = file.projects.iter().all(|project| {
            validate_name(&project.name).is_ok() && validate_base_url(&project.base_url).is_ok()
        });
        if file.version != 1 || file.projects.len() > MAX_PROJECTS || !names_valid {
            return Err(ProjectError::InvalidFile);
        }
        Ok(file)
    }

    fn write_file(&self, file: &StoredFile) -> Result<(), ProjectError> {
        let body = serde_json::to_vec_pretty(file).map_err(|_| ProjectError::Storage)?;
        secure_file::write_private(&self.paths.projects, &body).map_err(|_| ProjectError::Storage)
    }

    /// Every project, with its token decrypted when possible.
    pub fn load(&self) -> Result<Vec<Project>, ProjectError> {
        let file = self.read_file()?;
        let key = SecretKey::load(&self.paths.key).map_err(|_| ProjectError::Storage)?;
        file.projects
            .into_iter()
            .map(|stored| {
                let base_url = validate_base_url(&stored.base_url)?;
                let token = key.as_ref().and_then(|key| {
                    key.open(
                        &stored.token,
                        &token_context(&stored.name, base_url.as_str()),
                    )
                    .ok()
                });
                Ok(Project {
                    name: stored.name,
                    base_url,
                    token,
                })
            })
            .collect()
    }

    /// Adds a project; the token is encrypted before it is written.
    pub fn add(&self, name: &str, base_url: &str, token: &str) -> Result<(), ProjectError> {
        validate_name(name)?;
        let base_url = validate_base_url(base_url)?;
        let token = token.trim();
        validate_token(token)?;
        let _guard = self.write.lock().unwrap_or_else(|error| error.into_inner());
        let mut file = self.read_file()?;
        if file.projects.iter().any(|project| project.name == name) {
            return Err(ProjectError::Duplicate);
        }
        if file.projects.len() >= MAX_PROJECTS {
            return Err(ProjectError::TooMany);
        }
        let key = SecretKey::load_or_create(&self.paths.key).map_err(|_| ProjectError::Storage)?;
        let sealed = key
            .seal(token, &token_context(name, base_url.as_str()))
            .map_err(|_| ProjectError::Storage)?;
        file.projects.push(StoredProject {
            name: name.to_string(),
            base_url: base_url.to_string(),
            token: sealed,
        });
        self.write_file(&file)
    }

    /// Removes the project called `name`.
    pub fn remove(&self, name: &str) -> Result<(), ProjectError> {
        let _guard = self.write.lock().unwrap_or_else(|error| error.into_inner());
        let mut file = self.read_file()?;
        let before = file.projects.len();
        file.projects.retain(|project| project.name != name);
        if file.projects.len() == before {
            return Err(ProjectError::NotFound);
        }
        self.write_file(&file)
    }

    /// Whether the project list or its key can be read by other users.
    pub fn readable_by_others(&self) -> bool {
        secure_file::readable_by_others(&self.paths.projects)
            || secure_file::readable_by_others(&self.paths.key)
    }
}
