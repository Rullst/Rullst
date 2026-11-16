//! The file format of the project list (`projects.json`), and how Studio
//! reads a file that another Studio version wrote.
//!
//! Since 13.0.0-alpha.6 the format is forward compatible:
//!
//! - Every object keeps the fields this Studio does not know ([`Extra`]) and
//!   writes them back unchanged, so an older Studio that saves the list does
//!   not drop what a newer one stored. They are bounded: at most
//!   [`MAX_EXTRA_FIELDS`] per object, names of at most [`MAX_EXTRA_NAME_BYTES`]
//!   bytes and [`MAX_EXTRA_BYTES`] bytes of JSON per object.
//! - The fields Studio knows keep their strict types and bounds: names, base
//!   URLs, alert thresholds and notification settings are validated as when
//!   they are saved, and a file that fails is refused. The proxy setting is
//!   checked when it is used instead: one this Studio cannot use (a mode it
//!   does not know, a SOCKS URL) is kept as written and blocks every
//!   connection with a status that says why.
//! - `version` counts incompatible changes. Adding a field never raises it,
//!   and only a major release may. A reader refuses a file with a higher
//!   version; such a file names the oldest Studio that reads it in
//!   `min_studio_version`, which the refusal repeats.

use super::alerts::AlertRules;
use super::notify::NotificationSettings;
use super::preferences::StudioSettings;
use super::projects::{MAX_PROJECTS, ProjectError, validate_base_url, validate_name};
use super::proxy::ProxySettings;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The format this Studio reads and writes.
pub(crate) const FORMAT_VERSION: u32 = 1;
/// Largest project list read or written.
pub(crate) const MAX_FILE_BYTES: usize = 256 * 1024;
/// Most unknown fields kept in one object.
pub(crate) const MAX_EXTRA_FIELDS: usize = 32;
/// Longest name of an unknown field, in bytes.
pub(crate) const MAX_EXTRA_NAME_BYTES: usize = 64;
/// Most bytes of the unknown fields of one object, as compact JSON.
pub(crate) const MAX_EXTRA_BYTES: usize = 16 * 1024;
/// Longest `min_studio_version` that a refusal repeats.
const MAX_VERSION_CHARS: usize = 64;

/// The fields of one object that this Studio does not know, kept as read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Extra(Map<String, Value>);

impl Extra {
    /// Whether the fields fit the bounds of the module documentation.
    fn within_bounds(&self) -> bool {
        self.0.len() <= MAX_EXTRA_FIELDS
            && self.0.keys().all(|name| name.len() <= MAX_EXTRA_NAME_BYTES)
            && serde_json::to_vec(&self.0).is_ok_and(|json| json.len() <= MAX_EXTRA_BYTES)
    }
}

/// An object Studio knows (`known`) and the fields of a newer Studio next to
/// it (`extra`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Kept<T> {
    #[serde(flatten)]
    pub(crate) known: T,
    #[serde(flatten)]
    pub(crate) extra: Extra,
}

/// The whole file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredFile {
    pub(crate) version: u32,
    pub(crate) projects: Vec<StoredProject>,
    /// Absent while every setting has its default, so a Studio older than
    /// 13.0.0-alpha.6 keeps reading the file.
    #[serde(default, skip_serializing_if = "StoredSettings::is_default")]
    pub(crate) settings: StoredSettings,
    #[serde(flatten)]
    pub(crate) extra: Extra,
}

/// One project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredProject {
    pub(crate) name: String,
    pub(crate) base_url: String,
    /// The token sealed with [`super::crypto::SecretKey::seal`].
    pub(crate) token: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) watch: bool,
    #[serde(default)]
    pub(crate) rules: Kept<AlertRules>,
    /// The webhook URL sealed with [`super::crypto::SecretKey::seal`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) webhook: Option<String>,
    #[serde(flatten)]
    pub(crate) extra: Extra,
}

/// The Studio-wide settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredSettings {
    #[serde(default)]
    pub(crate) notifications: Kept<NotificationSettings>,
    #[serde(default)]
    pub(crate) proxy: Kept<ProxySettings>,
    #[serde(flatten)]
    pub(crate) extra: Extra,
}

impl StoredSettings {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The settings Studio uses.
    pub(crate) fn current(&self) -> StudioSettings {
        StudioSettings {
            notifications: self.notifications.known,
            proxy: self.proxy.known.clone(),
        }
    }
}

/// What a reader looks at before the rest of the file.
#[derive(Deserialize)]
struct Header {
    version: u32,
    #[serde(default)]
    min_studio_version: Option<Value>,
}

/// The major version of `min_studio_version` (`"14.0.0"` gives 14).
fn required_major(value: Option<&Value>) -> Option<u32> {
    let text = value?.as_str()?;
    if text.len() > MAX_VERSION_CHARS {
        return None;
    }
    text.split('.').next()?.parse().ok()
}

impl StoredFile {
    /// An empty list, for a missing file.
    pub(crate) fn empty() -> Self {
        Self {
            version: FORMAT_VERSION,
            projects: Vec::new(),
            settings: StoredSettings::default(),
            extra: Extra::default(),
        }
    }

    /// Reads and validates a file. A file of a later format is refused with
    /// [`ProjectError::NewerFormat`] before the rest of it is read.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, ProjectError> {
        let header: Header =
            serde_json::from_slice(bytes).map_err(|_| ProjectError::InvalidFile)?;
        if header.version > FORMAT_VERSION {
            return Err(ProjectError::NewerFormat {
                format: header.version,
                studio: required_major(header.min_studio_version.as_ref()),
            });
        }
        let file: Self = serde_json::from_slice(bytes).map_err(|_| ProjectError::InvalidFile)?;
        if file.version != FORMAT_VERSION || file.projects.len() > MAX_PROJECTS || !file.valid() {
            return Err(ProjectError::InvalidFile);
        }
        Ok(file)
    }

    /// Whether every known field is within its bounds and every object's
    /// unknown fields fit theirs.
    fn valid(&self) -> bool {
        let projects = self.projects.iter().all(|project| {
            validate_name(&project.name).is_ok()
                && validate_base_url(&project.base_url).is_ok()
                && project.rules.known.valid()
                && project.rules.extra.within_bounds()
                && project.extra.within_bounds()
        });
        let settings = &self.settings;
        projects
            && settings.notifications.known.valid()
            && settings.notifications.extra.within_bounds()
            && settings.proxy.extra.within_bounds()
            && settings.extra.within_bounds()
            && self.extra.within_bounds()
    }

    /// The file as written, refused when it would be too large to read back.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, ProjectError> {
        let body = serde_json::to_vec_pretty(self).map_err(|_| ProjectError::Storage)?;
        if body.len() > MAX_FILE_BYTES {
            return Err(ProjectError::Storage);
        }
        Ok(body)
    }
}
