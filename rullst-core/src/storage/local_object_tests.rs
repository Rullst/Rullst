//! Local keys that are not objects behave like the cloud backends: not found.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{LocalDriver, Storage, StorageError};

#[tokio::test]
async fn directories_and_paths_below_files_are_not_objects() {
    let root = tempfile::tempdir().unwrap();
    let storage = Storage::local(root.path().to_string_lossy());
    let driver = LocalDriver::new(root.path());
    storage.put("avatars/1.png", b"png").await.unwrap();

    for key in ["avatars", "avatars/1.png/thumb", "avatars/1.png/a/b"] {
        assert!(!driver.exists(key).await.unwrap(), "{key}");
        assert!(
            matches!(storage.get(key).await, Err(StorageError::NotFound(_))),
            "{key}"
        );
        assert!(
            matches!(storage.metadata(key).await, Err(StorageError::NotFound(_))),
            "{key}"
        );
        assert!(
            matches!(storage.delete(key).await, Err(StorageError::NotFound(_))),
            "{key}"
        );
    }
    // The real object and its directory are untouched.
    assert!(driver.exists("avatars/1.png").await.unwrap());
    assert_eq!(storage.get("avatars/1.png").await.unwrap(), b"png");
    assert_eq!(
        storage.metadata("avatars/1.png").await.unwrap().size_bytes,
        3
    );
}
