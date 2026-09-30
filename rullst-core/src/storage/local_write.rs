//! Atomic replacement of local storage objects.
//!
//! `LocalDriver::put` never truncates the destination. It writes a uniquely
//! named temporary file in the destination's (already validated) directory,
//! flushes it to disk and renames it over the destination. Readers and
//! concurrent writers therefore observe one complete version of an object.

use super::StorageError;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Prefix of in-flight temporary files. A random suffix makes each name
/// unique, and the fixed length keeps long object names within `NAME_MAX`.
const TEMPORARY_PREFIX: &str = ".rullst-put-";

/// Replaces `target` with `bytes`, or leaves any previous version untouched.
pub(super) async fn replace_file(target: PathBuf, bytes: Vec<u8>) -> Result<(), StorageError> {
    tokio::task::spawn_blocking(move || replace_file_blocking(&target, &bytes))
        .await
        .map_err(|_| StorageError::Io("local storage write task failed".to_string()))?
}

fn replace_file_blocking(target: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = target
        .parent()
        .ok_or_else(|| StorageError::PathTraversal("storage path has no parent".to_string()))?;
    let temporary = parent.join(format!(
        "{TEMPORARY_PREFIX}{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));

    let result = write_and_rename(&temporary, target, bytes);
    if result.is_err() {
        // Best effort: the rename did not happen, so this is never the object.
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error| StorageError::Io(error.to_string()))
}

fn write_and_rename(temporary: &Path, target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // `create_new` never follows or reuses an existing entry, including a symlink.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    // Atomic on POSIX; `std` replaces an existing destination on Windows.
    std::fs::rename(temporary, target)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    #[cfg(unix)]
    use super::super::LocalDriver;
    use super::super::{Storage, StorageError};
    use super::TEMPORARY_PREFIX;

    /// A unique directory under the system temporary directory, removed on drop.
    struct TempRoot(std::path::PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("rullst-local-put-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn leftover_temporary_files(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(TEMPORARY_PREFIX))
            .collect()
    }

    /// A reader that opened the previous version keeps reading all of it: the
    /// overwrite creates a new file instead of truncating the one being read.
    #[cfg(unix)]
    #[tokio::test]
    async fn overwrite_never_truncates_the_version_a_reader_holds() {
        use std::io::Read;

        let root = TempRoot::new();
        let storage = Storage::local(root.path().to_string_lossy());
        storage.put("avatars/42.png", b"AAAAAAAAAA").await.unwrap();

        let mut reader = std::fs::File::open(root.path().join("avatars/42.png")).unwrap();
        storage.put("avatars/42.png", b"BBBBB").await.unwrap();

        let mut held = Vec::new();
        reader.read_to_end(&mut held).unwrap();
        assert_eq!(held, b"AAAAAAAAAA");
        assert_eq!(storage.get("avatars/42.png").await.unwrap(), b"BBBBB");
        assert!(leftover_temporary_files(&root.path().join("avatars")).is_empty());
    }

    // Windows may refuse a replacement while another handle lacks delete
    // sharing; there a concurrent put can fail, but it still never interleaves.
    #[cfg(unix)]
    #[tokio::test]
    async fn concurrent_writers_and_readers_only_observe_complete_versions() {
        const WRITERS: u8 = 8;
        const ROUNDS: usize = 6;

        let root = TempRoot::new();
        let driver = LocalDriver::new(root.path());
        // Different lengths and bytes make any interleaving or truncation visible.
        let payloads: Vec<Vec<u8>> = (0..WRITERS)
            .map(|index| vec![b'A' + index; 256 * 1024 + usize::from(index) * 4099])
            .collect();
        driver.put("lessons/1.bin", &payloads[0]).await.unwrap();

        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reader = {
            let (driver, payloads, done) = (driver.clone(), payloads.clone(), done.clone());
            tokio::spawn(async move {
                let mut reads = 0_usize;
                while !done.load(std::sync::atomic::Ordering::Acquire) || reads == 0 {
                    let bytes = driver.get("lessons/1.bin").await.unwrap();
                    assert!(
                        payloads.contains(&bytes),
                        "a reader observed a partial or interleaved object"
                    );
                    reads += 1;
                    tokio::task::yield_now().await;
                }
            })
        };

        let writers: Vec<_> = payloads
            .iter()
            .cloned()
            .map(|payload| {
                let driver = driver.clone();
                tokio::spawn(async move {
                    for _ in 0..ROUNDS {
                        driver.put("lessons/1.bin", &payload).await.unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.await.unwrap();
        }
        done.store(true, std::sync::atomic::Ordering::Release);
        reader.await.unwrap();

        let last = driver.get("lessons/1.bin").await.unwrap();
        assert!(payloads.contains(&last));
        assert!(leftover_temporary_files(&root.path().join("lessons")).is_empty());
    }

    #[tokio::test]
    async fn failed_replacement_keeps_the_destination_and_removes_the_temporary_file() {
        let root = TempRoot::new();
        let storage = Storage::local(root.path().to_string_lossy());
        storage
            .put("reports/final/keep.txt", b"kept")
            .await
            .unwrap();

        // A directory cannot be replaced by a file, so the final rename fails.
        let error = storage
            .put("reports/final", b"replacement")
            .await
            .unwrap_err();
        assert!(matches!(error, StorageError::Io(_)));
        assert!(root.path().join("reports/final").is_dir());
        assert_eq!(
            storage.get("reports/final/keep.txt").await.unwrap(),
            b"kept"
        );
        assert!(leftover_temporary_files(&root.path().join("reports")).is_empty());
    }
}
