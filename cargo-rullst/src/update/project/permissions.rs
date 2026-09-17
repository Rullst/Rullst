//! File access policy is part of the approved review, not inferred at commit.
use super::super::{ProjectError, snapshot};
use std::{fs, path::Path};

#[cfg(unix)]
#[path = "permissions_unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "permissions_windows.rs"]
mod platform;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::update::project) struct Permissions {
    readonly: bool,
    unix_mode: Option<u32>,
    unix_owner: Option<(u32, u32)>,
    windows_descriptor: Option<String>,
}

impl Permissions {
    pub fn capture(root: &Path, record: &snapshot::Record) -> Result<Self, ProjectError> {
        #[cfg(any(unix, windows))]
        {
            platform::capture(
                record
                    .sha256
                    .as_ref()
                    .map(|_| root.join(&record.path))
                    .as_deref(),
            )
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (root, record);
            Err(ProjectError::Invalid(
                "file permission preservation is unavailable on this platform",
            ))
        }
    }

    pub fn stage(&self, parent: &Path) -> Result<tempfile::NamedTempFile, ProjectError> {
        #[cfg(any(unix, windows))]
        {
            platform::stage(self, parent)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = parent;
            Err(ProjectError::Invalid(
                "file permission preservation is unavailable on this platform",
            ))
        }
    }
}
