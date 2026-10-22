//! The panic hook records the site of a panic inside `spawn_capturing`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[tokio::test]
async fn a_task_panic_records_its_site_and_a_backtrace_only_when_enabled() {
    let (handle, slot) = spawn_capturing(async {
        if std::hint::black_box(true) {
            panic!("error console capture fixture");
        }
    });
    let error = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("the task ends")
        .expect_err("the task panics");
    assert!(error.is_panic());

    let capture = slot.take();
    let (file, line) = capture.location.expect("panic site");
    assert!(file.ends_with("capture_tests.rs"), "{file}");
    assert_eq!(line, 10);
    // `RUST_BACKTRACE` decides once per process whether backtraces are taken.
    let enabled = Backtrace::capture().status() == BacktraceStatus::Captured;
    assert_eq!(capture.backtrace.is_some(), enabled);
    // The slot is emptied by `take`.
    assert!(slot.take().location.is_none());
}
