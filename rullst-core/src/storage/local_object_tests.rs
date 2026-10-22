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

/// A link inside the storage root that resolves inside it is still refused:
/// keys never pass through a symbolic link, in the last segment or before it.
#[cfg(unix)]
#[tokio::test]
async fn keys_through_symbolic_links_inside_the_root_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let storage = Storage::local(root.path().to_string_lossy());
    storage.put("real/doc.txt", b"doc").await.unwrap();
    let canonical = root.path().canonicalize().unwrap();
    std::os::unix::fs::symlink(canonical.join("real/doc.txt"), canonical.join("alias.txt"))
        .unwrap();
    std::os::unix::fs::symlink(canonical.join("real"), canonical.join("linked")).unwrap();

    for key in ["alias.txt", "linked/doc.txt"] {
        assert!(
            matches!(storage.get(key).await, Err(StorageError::PathTraversal(_))),
            "{key}"
        );
        assert!(
            matches!(
                storage.put(key, b"new").await,
                Err(StorageError::PathTraversal(_))
            ),
            "{key}"
        );
    }
    assert_eq!(storage.get("real/doc.txt").await.unwrap(), b"doc");
    // A missing segment is not a link: a new key below it can be written.
    storage.put("fresh/dir/new.txt", b"new").await.unwrap();
    assert_eq!(storage.get("fresh/dir/new.txt").await.unwrap(), b"new");
}
