//! The project list of the standalone Studio: name, base URL and encrypted
//! monitor token per project, in an owner-only JSON file.

use super::alerts::AlertRules;
use super::crypto::{Secret, SecretKey};
use super::notify::NotificationSettings;
use super::paths::StudioPaths;
use super::preferences::StudioSettings;
use super::proxy::ProxySettings;
use super::secure_file;
use super::stored::{FORMAT_VERSION, Kept, MAX_FILE_BYTES, StoredFile, StoredProject};
use reqwest::Url;
use std::fmt;
use std::net::IpAddr;
use std::sync::Mutex;

/// Most projects the list holds.
pub const MAX_PROJECTS: usize = 32;
const MAX_NAME_CHARS: usize = 48;
const MAX_URL_CHARS: usize = 512;
const MAX_TOKEN_CHARS: usize = 512;
/// The version 1 monitoring document, relative to a project's base URL.
const MONITOR_PATH: &str = "_rullst/monitor/v1";
/// The version 2 document (traces and the security radar), relative to a
/// project's base URL.
const MONITOR_V2_PATH: &str = "_rullst/monitor/v2";

/// Why the project list could not be read or changed. Messages never contain
/// a token, a URL or file contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectError {
    /// The project list or its key could not be read or written.
    Storage,
    /// The project list file is malformed; fix or remove it.
    InvalidFile,
    /// The project list was written in a later, incompatible file format
    /// (since 13.0.0-alpha.6). A later 13.x Studio only adds fields, which
    /// this Studio keeps; only a major release changes the format.
    #[non_exhaustive]
    NewerFormat {
        /// The file's format version.
        format: u32,
        /// The major version of the oldest Studio that reads it, when the
        /// file names one.
        studio: Option<u32>,
    },
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
    /// A webhook URL is not `https://` with a host, without user information or
    /// fragment, or is longer than 2,048 characters.
    InvalidWebhook,
    /// An alert threshold is out of range.
    InvalidRules,
    /// A Studio setting is out of range, such as a quiet period outside 1 to
    /// 1,440 minutes.
    InvalidSettings,
    /// A proxy URL is not `http://` or `https://` with a host and without
    /// user information, path, query or fragment (SOCKS is not supported), or
    /// the exceptions exceed 4,096 characters.
    InvalidProxy,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NewerFormat { format, studio } => {
                let reader = match studio {
                    Some(major) => format!("which needs Rullst Studio {major} or later"),
                    None => "written by a newer Rullst Studio".to_string(),
                };
                return write!(
                    formatter,
                    "the project list uses file format {format}, {reader}; this Studio ({}) reads format {FORMAT_VERSION}",
                    env!("CARGO_PKG_VERSION")
                );
            }
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
            Self::InvalidWebhook => {
                "the webhook must be an https:// URL without user information or fragment"
            }
            Self::InvalidRules => {
                "thresholds must be positive (a 5xx rate up to 100%, p95 and p99 up to 600000 ms) and the cool-down 1 to 1440 minutes"
            }
            Self::InvalidSettings => "the quiet period must be 1 to 1440 minutes",
            Self::InvalidProxy => {
                "the proxy must be an http:// or https:// URL with a host and no user, path, query or fragment (SOCKS is not supported), and the exceptions at most 4096 characters"
            }
        };
        formatter.write_str(text)
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
    /// Whether the background poller reads this project and keeps history.
    pub watch: bool,
    /// Alert thresholds.
    pub rules: AlertRules,
    /// Whether a webhook is stored (it may still fail to decrypt).
    pub webhook_configured: bool,
    /// The decrypted webhook URL.
    pub webhook: Option<Secret>,
}

impl Project {
    /// The URL of the project's version 1 monitoring document.
    pub fn monitor_url(&self) -> Option<Url> {
        self.base_url.join(MONITOR_PATH).ok()
    }

    /// The URL of the project's version 2 monitoring document.
    pub fn monitor_v2_url(&self) -> Option<Url> {
        self.base_url.join(MONITOR_V2_PATH).ok()
    }
}

/// What to do with a project's webhook when saving its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookChange {
    /// Keep the stored webhook.
    Keep,
    /// Delete the stored webhook.
    Clear,
    /// Store this URL (validated, then sealed).
    Set(String),
}

/// The associated data binding a sealed token to its project.
pub(crate) fn token_context(name: &str, base_url: &str) -> Vec<u8> {
    format!("rullst-studio/token\n{name}\n{base_url}").into_bytes()
}

/// The associated data binding a sealed webhook URL to its project.
pub(crate) fn webhook_context(name: &str, base_url: &str) -> Vec<u8> {
    format!("rullst-studio/webhook\n{name}\n{base_url}").into_bytes()
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
        let bytes = secure_file::read_private(&self.paths.projects, MAX_FILE_BYTES as u64)
            .map_err(|_| ProjectError::Storage)?;
        match bytes {
            Some(bytes) => StoredFile::parse(&bytes),
            None => Ok(StoredFile::empty()),
        }
    }

    fn write_file(&self, file: &StoredFile) -> Result<(), ProjectError> {
        let body = file.encode()?;
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
                let open = |sealed: &str, context: Vec<u8>| {
                    key.as_ref().and_then(|key| key.open(sealed, &context).ok())
                };
                let token = open(
                    &stored.token,
                    token_context(&stored.name, base_url.as_str()),
                );
                let webhook = stored.webhook.as_deref().and_then(|sealed| {
                    open(sealed, webhook_context(&stored.name, base_url.as_str()))
                });
                Ok(Project {
                    token,
                    watch: stored.watch,
                    rules: stored.rules.known,
                    webhook_configured: stored.webhook.is_some(),
                    webhook,
                    name: stored.name,
                    base_url,
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
            watch: false,
            rules: Kept::default(),
            webhook: None,
            extra: Default::default(),
        });
        self.write_file(&file)
    }

    /// Changes whether `name` is watched, its alert rules and its webhook.
    pub fn update_settings(
        &self,
        name: &str,
        watch: bool,
        rules: AlertRules,
        webhook: WebhookChange,
    ) -> Result<(), ProjectError> {
        if !rules.valid() {
            return Err(ProjectError::InvalidRules);
        }
        let webhook_url = match &webhook {
            WebhookChange::Set(url) => Some(super::webhook::validate_webhook_url(url)?),
            WebhookChange::Keep | WebhookChange::Clear => None,
        };
        let _guard = self.write.lock().unwrap_or_else(|error| error.into_inner());
        let mut file = self.read_file()?;
        let project = file
            .projects
            .iter_mut()
            .find(|project| project.name == name)
            .ok_or(ProjectError::NotFound)?;
        project.watch = watch;
        // Fields of a newer Studio stay next to the thresholds.
        project.rules.known = rules;
        match webhook {
            WebhookChange::Keep => {}
            WebhookChange::Clear => project.webhook = None,
            WebhookChange::Set(_) => {
                let key = SecretKey::load_or_create(&self.paths.key)
                    .map_err(|_| ProjectError::Storage)?;
                let url = webhook_url.ok_or(ProjectError::InvalidWebhook)?;
                project.webhook = Some(
                    key.seal(
                        url.as_str(),
                        &webhook_context(&project.name, &project.base_url),
                    )
                    .map_err(|_| ProjectError::Storage)?,
                );
            }
        }
        self.write_file(&file)
    }

    /// The Studio settings (defaults when none are stored).
    pub(crate) fn settings(&self) -> Result<StudioSettings, ProjectError> {
        Ok(self.read_file()?.settings.current())
    }

    /// Saves the desktop notification settings.
    pub(crate) fn update_notifications(
        &self,
        notifications: NotificationSettings,
    ) -> Result<(), ProjectError> {
        if !notifications.valid() {
            return Err(ProjectError::InvalidSettings);
        }
        let _guard = self.write.lock().unwrap_or_else(|error| error.into_inner());
        let mut file = self.read_file()?;
        file.settings.notifications.known = notifications;
        self.write_file(&file)
    }

    /// Saves the proxy settings.
    pub(crate) fn update_proxy(&self, proxy: ProxySettings) -> Result<(), ProjectError> {
        if !proxy.valid() {
            return Err(ProjectError::InvalidProxy);
        }
        let _guard = self.write.lock().unwrap_or_else(|error| error.into_inner());
        let mut file = self.read_file()?;
        file.settings.proxy.known = proxy;
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

    #[cfg(test)]
    pub(crate) fn paths(&self) -> &StudioPaths {
        &self.paths
    }

    /// Whether the project list or its key can be read by other users.
    pub fn readable_by_others(&self) -> bool {
        secure_file::readable_by_others(&self.paths.projects)
            || secure_file::readable_by_others(&self.paths.key)
    }
}
