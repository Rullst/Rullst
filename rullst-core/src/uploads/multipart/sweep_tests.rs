#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::uploads::multipart::read_tests::{CONTENT_TYPE, file_part, form, within};
use crate::uploads::multipart::{Multipart, MultipartPolicy, UploadForm};
use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::header;
use std::sync::Arc;

const ID: &str = "0123456789abcdef0123456789abcdef";
const DAY: Duration = Duration::from_secs(86_400);
const LIMITS_FOR_TESTS: SweepLimits = SweepLimits {
    max_entries: 100,
    max_removals: 100,
};

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000)
}

fn temp_name(id: &str) -> String {
    format!(".rullst-upload-{id}.part")
}

/// Creates the regular file `name` in `dir`, last modified at `modified`.
fn touch(dir: &Path, name: &str, modified: SystemTime) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    file.set_modified(modified).unwrap();
    path
}

#[test]
fn only_the_names_this_module_creates_match() {
    assert!(is_temp_name(OsStr::new(&temp_name(ID))));
    for other in [
        ".rullst-upload-0123456789ABCDEF0123456789abcdef.part",
        ".rullst-upload-0123456789abcdef0123456789abcde.part",
        ".rullst-upload-0123456789abcdef0123456789abcdef0.part",
        ".rullst-upload-0123456789abcdef0123456789abcdeg.part",
        ".rullst-upload-0123456789abcdef0123456789abcdef.part.bak",
        "rullst-upload-0123456789abcdef0123456789abcdef.part",
        ".rullst-upload-0123456789abcdef0123456789abcdef",
        ".rullst-upload-.part",
        "report.pdf",
    ] {
        assert!(!is_temp_name(OsStr::new(other)), "{other}");
    }
}

#[test]
fn stale_temporary_files_are_removed_and_everything_else_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let stale = touch(dir.path(), &temp_name(ID), now() - DAY);
    let older = touch(
        dir.path(),
        &temp_name("ffffffffffffffffffffffffffffffff"),
        now() - DAY * 30,
    );
    let recent = touch(
        dir.path(),
        &temp_name("00000000000000000000000000000000"),
        now() - DAY + Duration::from_secs(1),
    );
    let future = touch(
        dir.path(),
        &temp_name("11111111111111111111111111111111"),
        now() + DAY,
    );
    let other = touch(dir.path(), "report.pdf", now() - DAY * 30);
    let upper = touch(
        dir.path(),
        ".rullst-upload-0123456789ABCDEF0123456789ABCDEF.part",
        now() - DAY * 30,
    );

    let outcome = sweep_dir(dir.path(), now(), LIMITS_FOR_TESTS);

    assert_eq!(
        outcome,
        SweepOutcome {
            removed: 2,
            failed: 0,
            error: None
        }
    );
    assert!(!stale.exists() && !older.exists());
    for kept in [&recent, &future, &other, &upper] {
        assert!(kept.exists(), "{}", kept.display());
    }
}

#[cfg(unix)]
#[test]
fn directories_and_symbolic_links_are_never_removed() {
    let dir = tempfile::tempdir().unwrap();
    let directory = dir.path().join(temp_name(ID));
    std::fs::create_dir(&directory).unwrap();
    std::fs::File::open(&directory)
        .unwrap()
        .set_modified(now() - DAY * 30)
        .unwrap();
    let target = touch(dir.path(), "kept.bin", now() - DAY * 30);
    let link = dir
        .path()
        .join(temp_name("abcdefabcdefabcdefabcdefabcdefab"));
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let outcome = sweep_dir(dir.path(), now(), LIMITS_FOR_TESTS);

    assert_eq!(outcome, SweepOutcome::default());
    assert!(directory.is_dir());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(target.exists());
}

#[test]
fn a_sweep_removes_and_examines_a_bounded_number_of_files() {
    let dir = tempfile::tempdir().unwrap();
    for id in [
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "cccccccccccccccccccccccccccccccc",
    ] {
        touch(dir.path(), &temp_name(id), now() - DAY);
    }
    let removals = SweepLimits {
        max_entries: 100,
        max_removals: 2,
    };
    assert_eq!(sweep_dir(dir.path(), now(), removals).removed, 2);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

    touch(dir.path(), &temp_name(ID), now() - DAY);
    let entries = SweepLimits {
        max_entries: 1,
        max_removals: 100,
    };
    assert_eq!(sweep_dir(dir.path(), now(), entries).removed, 1);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn an_unreadable_directory_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    assert_eq!(
        sweep_dir(&missing, now(), LIMITS_FOR_TESTS),
        SweepOutcome {
            removed: 0,
            failed: 0,
            error: Some(ErrorKind::NotFound)
        }
    );
}

#[cfg(unix)]
#[test]
fn a_stale_file_that_cannot_be_removed_is_counted() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    let stale = touch(&locked, &temp_name(ID), now() - DAY);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();
    // A privileged user ignores directory permissions: nothing to observe.
    let privileged = std::fs::File::create(locked.join("probe")).is_ok();
    let outcome = sweep_dir(&locked, now(), LIMITS_FOR_TESTS);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
    if !privileged {
        assert_eq!(
            outcome,
            SweepOutcome {
                removed: 0,
                failed: 1,
                error: None
            }
        );
        assert!(stale.exists());
    }
}

#[test]
fn a_directory_is_swept_at_most_once_a_day() {
    let mut last_sweeps = BTreeMap::new();
    let start = Instant::now();
    let uploads = Path::new("uploads");
    assert!(claim(&mut last_sweeps, uploads, start));
    assert!(claim(&mut last_sweeps, Path::new("other"), start));
    let almost = start + DAY - Duration::from_secs(1);
    assert!(!claim(&mut last_sweeps, uploads, almost));
    assert!(claim(&mut last_sweeps, uploads, start + DAY));
    assert!(!claim(&mut last_sweeps, Path::new("other"), almost));

    // The memory is bounded: the 257th directory starts it over.
    let mut full: BTreeMap<PathBuf, Instant> = (0..256)
        .map(|index| (PathBuf::from(format!("dir-{index}")), start))
        .collect();
    assert!(claim(&mut full, Path::new("dir-0"), start + DAY));
    assert_eq!(full.len(), 256);
    assert!(claim(&mut full, uploads, start));
    assert_eq!(full.len(), 1);
}

/// The `rullst::uploads` lines that `report` writes for `outcome`.
fn reported(outcome: &SweepOutcome) -> String {
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || report(Path::new("uploads"), outcome));
    let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();
    logs.lines()
        .filter(|line| line.contains("rullst::uploads"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn failures_are_logged_as_warnings_and_removals_as_information() {
    let unreadable = reported(&SweepOutcome {
        error: Some(ErrorKind::PermissionDenied),
        ..SweepOutcome::default()
    });
    assert!(
        unreadable.contains("WARN") && !unreadable.contains("INFO"),
        "{unreadable}"
    );
    assert!(
        unreadable.contains("error=Some(PermissionDenied)"),
        "{unreadable}"
    );
    let failed = reported(&SweepOutcome {
        removed: 3,
        failed: 1,
        error: None,
    });
    assert!(
        failed.contains("WARN") && !failed.contains("INFO"),
        "{failed}"
    );
    assert!(
        failed.contains("dir=uploads removed=3 failed=1"),
        "{failed}"
    );
    let removed = reported(&SweepOutcome {
        removed: 2,
        ..SweepOutcome::default()
    });
    assert!(
        removed.contains("INFO") && !removed.contains("WARN"),
        "{removed}"
    );
    assert!(removed.contains("dir=uploads removed=2"), "{removed}");
    assert_eq!(reported(&SweepOutcome::default()), "");
}

async fn spill_into(dir: &Path) -> UploadForm {
    let request = axum::http::Request::post("/upload")
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .body(Body::from(form(&[file_part("doc", "a.bin", b"spilled")])))
        .unwrap();
    let policy = MultipartPolicy::default().memory_threshold(0).temp_dir(dir);
    within(async {
        let multipart = Multipart::from_request(request, &()).await.unwrap();
        multipart.read(&policy).await.unwrap()
    })
    .await
}

#[tokio::test]
async fn the_first_upload_written_to_a_directory_sweeps_it_once() {
    let dir = tempfile::tempdir().unwrap();
    let real_now = SystemTime::now();
    let stale = touch(dir.path(), &temp_name(ID), real_now - DAY * 2);
    let recent = touch(
        dir.path(),
        &temp_name("00000000000000000000000000000000"),
        real_now,
    );

    let upload = spill_into(dir.path()).await;
    let spilled = upload.file("doc").unwrap().temp_path().unwrap().to_owned();
    assert!(spilled.exists());
    assert!(!stale.exists());
    assert!(recent.exists());
    drop(upload);
    assert!(!spilled.exists());

    // The directory was swept today: a new leftover waits for tomorrow.
    let later = touch(
        dir.path(),
        &temp_name("22222222222222222222222222222222"),
        real_now - DAY * 2,
    );
    drop(spill_into(dir.path()).await);
    assert!(later.exists());
}
