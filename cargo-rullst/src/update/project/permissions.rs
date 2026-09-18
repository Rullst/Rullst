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

    pub fn capture_many<'a>(
        root: &Path,
        records: impl Iterator<Item = &'a snapshot::Record>,
    ) -> Result<Vec<Self>, ProjectError> {
        bounded(records.map(|record| Self::capture(root, record)))
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

fn bounded(
    policies: impl Iterator<Item = Result<Permissions, ProjectError>>,
) -> Result<Vec<Permissions>, ProjectError> {
    let mut total = 0usize;
    let mut result = Vec::new();
    for policy in policies {
        let policy = policy?;
        total = total
            .checked_add(serde_json::to_vec(&policy)?.len())
            .ok_or(ProjectError::Invalid("file access policy size overflow"))?;
        if total > 1024 * 1024 {
            return Err(ProjectError::Invalid(
                "reviewed file access policies exceed 1 MiB",
            ));
        }
        result.push(policy);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_permission_budget_stops_before_collecting_all_descriptors() {
        let inspected = std::cell::Cell::new(0);
        let policies = (0..100_000).map(|_| {
            inspected.set(inspected.get() + 1);
            Ok(Permissions {
                readonly: false,
                unix_mode: None,
                unix_owner: None,
                windows_descriptor: Some("x".repeat(32 * 1024)),
            })
        });
        assert!(bounded(policies).is_err());
        assert!(inspected.get() < 33);
    }
}
