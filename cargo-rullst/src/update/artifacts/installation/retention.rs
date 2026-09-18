//! Remove only verified, terminal, non-current updater evidence. Never follow links.
use super::{
    ArtifactError, storage,
    transaction::{self, Intent, Phase},
};
use crate::update::cache;
use std::{fs, path::Path};

pub(super) fn prune(
    storage: &Path,
    root: &Path,
    current: Option<&Path>,
) -> Result<(), ArtifactError> {
    let mut count = 0;
    for entry in fs::read_dir(storage)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with("op-") {
            continue;
        }
        storage::validate_operation(&entry.file_name().to_string_lossy())?;
        count += 1;
        if count > 8 {
            return Err(ArtifactError::Invalid(
                "installation evidence exceeds retention bound",
            ));
        }
        let operation = entry.path();
        if Some(operation.as_path()) == current {
            continue;
        }
        cache::installation_root(&operation)?;
        if !entry.file_type()?.is_dir() || !operation.join("intent.json").try_exists()? {
            continue;
        }
        let intent = Intent::load(&operation, root)?;
        if !matches!(intent.phase, Phase::Complete | Phase::Recovered) {
            continue;
        }
        let mut files = Vec::new();
        let mut directories = Vec::new();
        // Validate the entire owned inventory before deleting any entry.
        for child in fs::read_dir(&operation)? {
            let child = child?;
            if child.file_name() == "intent.json" {
                continue;
            }
            let name = child.file_name().to_string_lossy().into_owned();
            if !["before", "after", "retired", "restore", "removed"].contains(&name.as_str()) {
                return Err(ArtifactError::Invalid(
                    "unknown retained installation evidence; manual review required",
                ));
            }
            cache::installation_root(&child.path())?;
            for file in fs::read_dir(child.path())? {
                let file = file?;
                let change = intent
                    .changes
                    .iter()
                    .find(|change| file.file_name() == change.name.as_str())
                    .ok_or(ArtifactError::Invalid(
                        "unknown file in retained installation evidence",
                    ))?;
                let expected = if matches!(name.as_str(), "after" | "removed") {
                    Some(change.after.clone())
                } else {
                    change.before.clone()
                };
                transaction::matching(&file.path(), &expected)?;
                files.push(file.path());
            }
            directories.push(child.path());
        }
        for file in files {
            fs::remove_file(file).map_err(|error| {
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    ArtifactError::Invalid("retained executable cannot be removed; close older CLI processes and retry; the current installation is unchanged")
                } else { error.into() }
            })?;
        }
        for directory in directories {
            fs::remove_dir(directory)?;
        }
        fs::remove_file(operation.join("intent.json"))?;
        fs::remove_dir(operation)?;
        storage::sync(storage)?;
    }
    Ok(())
}
