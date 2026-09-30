//! Existing-object path checks for the local driver.

use super::StorageError;
use std::path::{Component, Path};

pub(super) async fn reject_symlink_components(
    canonical_base: &Path,
    relative_path: &Path,
) -> Result<(), StorageError> {
    let mut candidate = canonical_base.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(StorageError::PathTraversal(
                relative_path.to_string_lossy().into_owned(),
            ));
        };
        candidate.push(segment);
        match tokio::fs::symlink_metadata(&candidate).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(StorageError::PathTraversal(
                    relative_path.to_string_lossy().into_owned(),
                ));
            }
            Ok(_) => {}
            // A missing segment, or one below a regular file, cannot be a link.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                break;
            }
            Err(error) => return Err(StorageError::from(error)),
        }
    }
    Ok(())
}

/// Maps "a key segment is a regular file" (`ENOTDIR`) to `NotFound`, like a
/// missing key, instead of an I/O failure.
pub(super) fn missing_object(error: std::io::Error) -> StorageError {
    if error.kind() == std::io::ErrorKind::NotADirectory {
        StorageError::NotFound(error.to_string())
    } else {
        StorageError::from(error)
    }
}
