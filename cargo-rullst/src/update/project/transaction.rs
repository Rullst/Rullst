//! Stage all source replacements before committing any directory entry.
use super::{ProjectError, snapshot};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[path = "permissions.rs"]
mod permissions;
pub(super) use permissions::Permissions;

pub(super) struct Replacement {
    pub current: snapshot::Record,
    pub desired: snapshot::Record,
    pub permissions: Permissions,
}

impl Replacement {
    fn unchanged(&self, root: &Path) -> Result<(), ProjectError> {
        matches(root, &self.current)?;
        if Permissions::capture(root, &self.current)? != self.permissions {
            return Err(ProjectError::Invalid(
                "source access policy changed after review",
            ));
        }
        Ok(())
    }
}

pub(super) struct Staged {
    replacements: Vec<(Replacement, Option<tempfile::NamedTempFile>)>,
    pub owned_paths: BTreeSet<String>,
}

impl Staged {
    pub fn create(
        root: &Path,
        content_root: &Path,
        replacements: Vec<Replacement>,
    ) -> Result<Self, ProjectError> {
        let mut staged = Self {
            replacements: Vec::new(),
            owned_paths: BTreeSet::new(),
        };
        let mut total = 0u64;
        for replacement in replacements {
            let name = &replacement.current.path;
            snapshot::validate_path(name)?;
            if name != &replacement.desired.path
                || (replacement.desired.sha256.is_none() && name != "Cargo.lock")
            {
                return Err(ProjectError::Invalid("invalid source replacement scope"));
            }
            replacement.unchanged(root)?;
            let desired = snapshot::read(content_root, name)?;
            if snapshot::record(name, desired.as_deref()) != replacement.desired {
                return Err(ProjectError::Invalid(
                    "replacement bytes changed before staging",
                ));
            }
            total = total
                .checked_add(replacement.desired.bytes)
                .ok_or(ProjectError::Invalid("replacement size overflow"))?;
            if total > snapshot::MAX_TOTAL {
                return Err(ProjectError::Invalid("source replacements exceed 512 MiB"));
            }
            let temporary = if let Some(body) = desired {
                let target = root.join(name);
                let parent = target
                    .parent()
                    .ok_or(ProjectError::Invalid("replacement has no parent"))?;
                let mut file = replacement.permissions.stage(parent)?;
                file.write_all(&body)?;
                file.as_file().sync_all()?;
                let relative = file
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| ProjectError::Invalid("staging file leaves source root"))?
                    .to_str()
                    .ok_or(ProjectError::Invalid("staging path must be UTF-8"))?
                    .replace('\\', "/");
                staged.owned_paths.insert(relative);
                Some(file)
            } else {
                None
            };
            staged.replacements.push((replacement, temporary));
        }
        Ok(staged)
    }

    pub fn commit(self, root: &Path) -> Result<usize, (usize, ProjectError)> {
        self.commit_with(root, |_, temporary, path, absent| {
            replace(temporary, path, absent)
        })
    }

    fn commit_with(
        self,
        root: &Path,
        mut writer: impl FnMut(
            usize,
            Option<tempfile::NamedTempFile>,
            &Path,
            bool,
        ) -> Result<(), ProjectError>,
    ) -> Result<usize, (usize, ProjectError)> {
        let count = self.replacements.len();
        for (replacement, _) in &self.replacements {
            replacement.unchanged(root).map_err(|error| (0, error))?;
        }
        for (index, (replacement, temporary)) in self.replacements.into_iter().enumerate() {
            replacement
                .unchanged(root)
                .map_err(|error| (index, error))?;
            let target = root.join(&replacement.current.path);
            writer(
                index,
                temporary,
                &target,
                replacement.current.sha256.is_none(),
            )
            .map_err(|error| (index, error))?;
            #[cfg(unix)]
            if let Some(parent) = target.parent() {
                fs::File::open(parent)
                    .and_then(|file| file.sync_all())
                    .map_err(|error| (index + 1, ProjectError::Io(error)))?;
            }
        }
        Ok(count)
    }
}

fn replace(
    temporary: Option<tempfile::NamedTempFile>,
    path: &Path,
    absent: bool,
) -> Result<(), ProjectError> {
    if let Some(temporary) = temporary {
        if absent {
            temporary.persist_noclobber(path)
        } else {
            temporary.persist(path)
        }
        .map_err(|error| ProjectError::Io(error.error))?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub(super) fn matches(root: &Path, expected: &snapshot::Record) -> Result<(), ProjectError> {
    let bytes = snapshot::read(root, &expected.path)?;
    if snapshot::record(&expected.path, bytes.as_deref()) != *expected {
        return Err(ProjectError::Invalid(
            "source changed before file replacement; no conflicting file was overwritten",
        ));
    }
    Ok(())
}

pub(super) fn write_record(directory: &Path, name: &str, body: &[u8]) -> Result<(), ProjectError> {
    if body.len() > 8 * 1024 * 1024 {
        return Err(ProjectError::Invalid("application intent exceeds 8 MiB"));
    }
    let mut staged = tempfile::NamedTempFile::new_in(directory)?;
    staged.write_all(body)?;
    staged.as_file().sync_all()?;
    staged
        .persist(directory.join(name))
        .map_err(|error| ProjectError::Io(error.error))?;
    #[cfg(unix)]
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub schema_version: String,
    pub phase: String,
    pub source: PathBuf,
    pub prepared_directory: PathBuf,
    pub verified_directory: PathBuf,
    pub receipt_sha256: String,
    pub review_sha256: String,
    pub changes: Vec<super::review::Change>,
    pub permissions: Vec<Permissions>,
}

impl Intent {
    pub fn store(&self, directory: &Path) -> Result<(), ProjectError> {
        write_record(
            directory,
            "application.json",
            &serde_json::to_vec_pretty(self)?,
        )
    }
}

#[cfg(test)]
#[path = "transaction_tests.rs"]
mod tests;
