#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn meta(empty_file_name: bool) -> FileMeta {
    FileMeta {
        display_name: Some("a.txt".to_string()),
        content_type: Some("text/plain".to_string()),
        empty_file_name,
    }
}

fn entries(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().count()
}

async fn stored(dir: &Path, policy: &MultipartPolicy, chunks: &[&[u8]]) -> UploadedFile {
    let mut sink = FileSink::new("doc".to_string(), meta(false), dir.to_path_buf());
    for chunk in chunks {
        sink.write(chunk, policy).await.unwrap();
    }
    sink.finish(policy).await.unwrap().expect("a file")
}

#[tokio::test]
async fn files_stay_in_memory_up_to_the_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .memory_threshold(8)
        .temp_dir(dir.path());
    let file = stored(dir.path(), &policy, &[b"1234", b"5678"]).await;
    assert_eq!(file.bytes(), Some(&b"12345678"[..]));
    assert_eq!(file.temp_path(), None);
    assert_eq!(file.len(), 8);
    assert!(!file.is_empty());
    assert_eq!(file.field_name(), "doc");
    assert_eq!(file.display_name(), Some("a.txt"));
    assert_eq!(file.declared_content_type(), Some("text/plain"));
    assert_eq!(file.kind(), Some(UploadKind::PlainText));
    assert_eq!(
        file.sha256_hex(),
        "ef797c8118f02dfb649607dd5d3f8c7623048c9c063d532cc95c5ed7a898a64f"
    );
    assert_eq!(file.read_to_vec().await.unwrap(), b"12345678");
    assert_eq!(entries(dir.path()), 0);
}

#[tokio::test]
async fn larger_files_stream_to_a_private_temporary_file_removed_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .memory_threshold(8)
        .temp_dir(dir.path());
    let file = stored(dir.path(), &policy, &[b"1234", b"56789"]).await;
    assert_eq!(file.bytes(), None);
    let path = file.temp_path().expect("a temporary file").to_path_buf();
    assert_eq!(path.parent(), Some(dir.path()));
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with(".rullst-upload-") && name.ends_with(".part"));
    assert_eq!(name.len(), ".rullst-upload-".len() + 32 + ".part".len());
    assert_eq!(std::fs::read(&path).unwrap(), b"123456789");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    assert_eq!(file.read_to_vec().await.unwrap(), b"123456789");
    assert!(format!("{file:?}").contains("in_memory: false"));
    drop(file);
    assert!(!path.exists());
    assert_eq!(entries(dir.path()), 0);
}

#[tokio::test]
async fn a_zero_threshold_writes_every_non_empty_file_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .memory_threshold(0)
        .temp_dir(dir.path());
    let file = stored(dir.path(), &policy, &[b"x"]).await;
    assert!(file.temp_path().is_some());
    let empty = stored(dir.path(), &policy, &[]).await;
    assert_eq!(empty.bytes(), Some(&b""[..]));
    assert!(empty.is_empty());
}

#[tokio::test]
async fn writes_enforce_the_file_limit_and_content_checks_before_storing() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .max_file_bytes(5)
        .memory_threshold(0)
        .temp_dir(dir.path());
    let mut sink = FileSink::new("doc".to_string(), meta(false), dir.path().to_path_buf());
    sink.write(b"12345", &policy).await.unwrap();
    assert_eq!(
        sink.write(b"6", &policy).await,
        Err(MultipartError::FileTooLarge)
    );
    assert_eq!(entries(dir.path()), 1);
    drop(sink);
    assert_eq!(entries(dir.path()), 0);

    let pdf_only = MultipartPolicy::default()
        .expect_signature(*b"%PDF-")
        .memory_threshold(0)
        .temp_dir(dir.path());
    let mut sink = FileSink::new("doc".to_string(), meta(false), dir.path().to_path_buf());
    assert_eq!(
        sink.write(b"\x89PNG", &pdf_only).await,
        Err(MultipartError::ContentDenied)
    );
    assert_eq!(entries(dir.path()), 0);
    // A file that ends before its signature could match is denied too.
    let mut sink = FileSink::new("doc".to_string(), meta(false), dir.path().to_path_buf());
    sink.write(b"%PD", &pdf_only).await.unwrap();
    assert!(matches!(
        sink.finish(&pdf_only).await,
        Err(MultipartError::ContentDenied)
    ));
    assert_eq!(entries(dir.path()), 0);
}

#[tokio::test]
async fn an_empty_file_input_is_skipped_but_an_empty_named_file_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default();
    let sink = FileSink::new("doc".to_string(), meta(true), dir.path().to_path_buf());
    assert!(!sink.counts_immediately());
    assert!(sink.is_empty());
    assert!(sink.finish(&policy).await.unwrap().is_none());
    let mut sink = FileSink::new("doc".to_string(), meta(true), dir.path().to_path_buf());
    sink.write(b"x", &policy).await.unwrap();
    assert!(!sink.is_empty());
    assert!(sink.finish(&policy).await.unwrap().is_some());
    let sink = FileSink::new("doc".to_string(), meta(false), dir.path().to_path_buf());
    assert!(sink.counts_immediately());
    assert!(sink.finish(&policy).await.unwrap().is_some());
}

#[tokio::test]
async fn a_spill_failure_is_a_storage_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let policy = MultipartPolicy::default().memory_threshold(0);
    let mut sink = FileSink::new("doc".to_string(), meta(false), missing);
    assert_eq!(
        sink.write(b"x", &policy).await,
        Err(MultipartError::Storage(ErrorKind::NotFound))
    );
}

#[tokio::test]
async fn persisting_creates_a_new_private_file_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    for threshold in [1024, 0] {
        let policy = MultipartPolicy::default()
            .memory_threshold(threshold)
            .temp_dir(dir.path());
        let target = dir.path().join(format!("kept-{threshold}.txt"));
        let file = stored(dir.path(), &policy, &[b"persist me"]).await;
        file.persist_to(&target).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"persist me");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "{threshold}");
        }
        // Only the target is left: the temporary file is gone.
        let names = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.starts_with(".rullst-upload-"))
            .count();
        assert_eq!(names, 0, "{threshold}");

        let again = stored(dir.path(), &policy, &[b"other"]).await;
        assert_eq!(
            again.persist_to(&target).await,
            Err(MultipartError::PersistTargetExists)
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"persist me");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn persisting_refuses_an_existing_symbolic_link() {
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim");
    std::fs::write(&victim, b"keep").unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&victim, &link).unwrap();
    for threshold in [1024, 0] {
        let policy = MultipartPolicy::default()
            .memory_threshold(threshold)
            .temp_dir(dir.path());
        let file = stored(dir.path(), &policy, &[b"x"]).await;
        assert_eq!(
            file.persist_to(&link).await,
            Err(MultipartError::PersistTargetExists)
        );
    }
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
}

#[tokio::test]
async fn persist_targets_are_validated() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default();
    std::fs::write(dir.path().join("plain-file"), b"").unwrap();
    for target in [
        PathBuf::new(),
        PathBuf::from(".."),
        dir.path().join("a").join("..").join("b"),
        dir.path().join("missing").join("file"),
        dir.path().join("plain-file").join("file"),
        PathBuf::from("/"),
    ] {
        let file = stored(dir.path(), &policy, &[b"x"]).await;
        assert_eq!(
            file.persist_to(&target).await,
            Err(MultipartError::InvalidPersistTarget),
            "{target:?}"
        );
    }
    // A bare file name lands in the current directory, which exists.
    assert!(validate_target(Path::new("bare-name.bin")).await.is_ok());
}

#[test]
fn only_an_existing_target_stops_a_failed_link_and_other_failures_copy() {
    assert_eq!(link_outcome(Ok(())), Linked::Done);
    assert_eq!(
        link_outcome(Err(std::io::Error::from(ErrorKind::AlreadyExists))),
        Linked::TargetExists
    );
    for kind in [
        ErrorKind::CrossesDevices,
        ErrorKind::Unsupported,
        ErrorKind::PermissionDenied,
        ErrorKind::Other,
    ] {
        assert_eq!(
            link_outcome(Err(std::io::Error::from(kind))),
            Linked::CopyInstead,
            "{kind:?}"
        );
    }
}

#[tokio::test]
async fn a_failed_copy_removes_the_partial_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("copy");
    assert_eq!(
        copy_new(&dir.path().join("no-source"), &target).await,
        Err(MultipartError::Storage(ErrorKind::NotFound))
    );
    assert!(!target.exists());
    let source = dir.path().join("source");
    std::fs::write(&source, b"copied").unwrap();
    copy_new(&source, &target).await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"copied");
    assert_eq!(
        copy_new(&source, &target).await,
        Err(MultipartError::PersistTargetExists)
    );
    assert_eq!(
        write_new(&target, b"x").await,
        Err(MultipartError::PersistTargetExists)
    );
    assert_eq!(
        create_target(&dir.path().join("missing").join("file"))
            .await
            .map(|_| ()),
        Err(MultipartError::Storage(ErrorKind::NotFound))
    );
}
