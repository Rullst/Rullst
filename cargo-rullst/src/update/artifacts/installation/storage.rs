//! Destination-local private staging and coordination, independent of cache settings.
use super::{ArtifactError, files};
use crate::update::cache;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(super) struct Storage {
    pub path: PathBuf,
    root: PathBuf,
    _lease: cache::FileLease,
}

impl Storage {
    pub fn open(root: &Path) -> Result<Self, ArtifactError> {
        let parent = root
            .parent()
            .ok_or(ArtifactError::Invalid("installation parent missing"))?;
        // Parent aliases are already canonicalized. Windows final components
        // are bounded ASCII, so case aliases share one lock before creation too.
        #[cfg(windows)]
        let identity_root = parent.join(
            root.file_name()
                .ok_or(ArtifactError::Invalid("installation name missing"))?
                .to_string_lossy()
                .to_ascii_lowercase(),
        );
        #[cfg(not(windows))]
        let identity_root = root.to_owned();
        let identity = hex::encode(Sha256::digest(serde_json::to_vec(&identity_root)?));
        let path = cache::create_installation_directory(
            &parent.join(format!(".rullst-install-{identity}")),
        )?;
        let lease = cache::installation_lock(&path.join("operation.lock"))?;
        let marker = serde_json::to_vec(
            &serde_json::json!({"schema":"rullst.cli-installation-owner.v1","root":identity_root}),
        )?;
        let owner = path.join("owner.json");
        if owner.try_exists()? {
            cache::installation_file(&owner)?;
            if files::read_bounded(&owner, 16 * 1024)? != marker {
                return Err(ArtifactError::Invalid(
                    "installation storage belongs to another root",
                ));
            }
        } else {
            for entry in fs::read_dir(&path)? {
                if entry?.file_name() != "operation.lock" {
                    return Err(ArtifactError::Invalid("unknown installation storage"));
                }
            }
            write_new(&owner, &marker)?;
        }
        Ok(Self {
            path,
            root: root.to_owned(),
            _lease: lease,
        })
    }

    pub fn operation(&self) -> Result<PathBuf, ArtifactError> {
        super::retention::prune(&self.path, &self.root, self.current()?.as_deref())?;
        let mut operations = 0;
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if ["operation.lock", "owner.json", "current.json"]
                .iter()
                .any(|name| entry.file_name() == *name)
            {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            validate_operation(&name)?;
            cache::installation_root(&entry.path())?;
            operations += 1;
        }
        if operations >= 8 {
            return Err(ArtifactError::Invalid(
                "eight retained installation operations; archive older completed evidence before continuing",
            ));
        }
        let name = format!("op-{}", uuid::Uuid::new_v4());
        Ok(cache::create_installation_directory(&self.path.join(name))?)
    }

    pub fn current(&self) -> Result<Option<PathBuf>, ArtifactError> {
        let path = self.path.join("current.json");
        if !path.try_exists()? {
            return Ok(None);
        }
        cache::installation_file(&path)?;
        let name: String = serde_json::from_slice(&files::read_bounded(&path, 128)?)?;
        validate_operation(&name)?;
        let operation = cache::installation_root(&self.path.join(name))?;
        files::directory(&operation)?;
        Ok(Some(operation))
    }

    pub fn select(&self, operation: &Path) -> Result<(), ArtifactError> {
        if operation.parent() != Some(self.path.as_path()) {
            return Err(ArtifactError::Invalid(
                "operation leaves destination storage",
            ));
        }
        let name = operation
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(ArtifactError::Invalid("invalid operation path"))?;
        validate_operation(name)?;
        atomic_write(&self.path, "current.json", &serde_json::to_vec(name)?)
    }
}

pub(super) fn validate_operation(name: &str) -> Result<(), ArtifactError> {
    let id = name
        .strip_prefix("op-")
        .and_then(|id| uuid::Uuid::parse_str(id).ok());
    if !id.is_some_and(|id| name == format!("op-{id}") && id.get_version_num() == 4) {
        return Err(ArtifactError::Invalid(
            "invalid installation operation name",
        ));
    }
    Ok(())
}

pub(super) fn sync(directory: &Path) -> Result<(), ArtifactError> {
    #[cfg(unix)]
    fs::File::open(directory)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
    let mut file = cache::new_installation_file(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn atomic_write(
    directory: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<(), ArtifactError> {
    if bytes.is_empty() || bytes.len() > 128 * 1024 {
        return Err(ArtifactError::Invalid(
            "installation record exceeds 128 KiB",
        ));
    }
    let mut temporary = tempfile::Builder::new()
        .prefix("record-")
        .make_in(directory, |path| {
            cache::new_installation_file(path).map_err(std::io::Error::other)
        })?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(directory.join(name))
        .map_err(|error| error.error)?;
    sync(directory)
}

pub(super) fn copy(
    source: &Path,
    target: &Path,
    bytes: u64,
    digest: &str,
    executable: bool,
) -> Result<(), ArtifactError> {
    let original = files::open(source, super::super::manifest::MAX_BINARY)?;
    let parent = target
        .parent()
        .ok_or(ArtifactError::Invalid("staging parent missing"))?;
    let mut staged = tempfile::Builder::new()
        .prefix("copy-stage-")
        .make_in(parent, |path| {
            cache::new_installation_file(path).map_err(std::io::Error::other)
        })?;
    let mut reader = original.take(bytes + 1);
    let copied = std::io::copy(&mut reader, &mut staged)?;
    staged.as_file().sync_all()?;
    if copied != bytes || files::digest(staged.path(), bytes)? != digest {
        return Err(ArtifactError::Invalid(
            "installation copy differs from the authenticated bytes",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            staged.path(),
            fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
        )?;
    }
    #[cfg(not(unix))]
    let _ = executable;
    staged
        .persist_noclobber(target)
        .map_err(|error| error.error)?;
    sync(parent)
}
