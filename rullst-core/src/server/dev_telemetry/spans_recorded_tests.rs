//! The process subscriber creates no ORM span until something that records
//! them is mounted. The flag is process-wide, so each check runs in a child
//! process that runs no other test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::telemetry::DEFAULT_LOG_FILTER;
use crate::telemetry::subscriber_tests::{Lines, orm_span, subscriber};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

const CHILD_ENV: &str = "RULLST_ORM_SPANS_RECORDED_CHILD";

/// In the test process: runs the test at `path` alone in a child process,
/// checks that it passed and returns `true`. In that child: returns `false`.
pub(crate) fn ran_in_child(path: &str) -> bool {
    if std::env::var_os(CHILD_ENV).is_some() {
        return false;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", path, "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stdout, &mut text);
        text
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("the child running {path} exceeded its deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let stdout = reader.join().unwrap();
    assert!(status.success(), "{stdout}");
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
    true
}

/// ORM spans of the process subscriber are disabled before `mount` and
/// created after it.
pub(crate) fn orm_spans_start_with(mount: impl FnOnce()) {
    let recorded = super::spans_recorded();
    let process = || subscriber(DEFAULT_LOG_FILTER, recorded, Lines::default());
    assert!(!recorded.load(Ordering::Relaxed));
    tracing::subscriber::with_default(process(), || {
        assert!(orm_span("Post", "posts").is_disabled());
    });
    mount();
    assert!(recorded.load(Ordering::Relaxed));
    tracing::subscriber::with_default(process(), || {
        assert!(!orm_span("Post", "posts").is_disabled());
    });
}

#[test]
fn mounting_the_development_recorder_turns_the_orm_spans_on() {
    if ran_in_child(
        "server::dev_telemetry::spans_recorded_tests::\
         mounting_the_development_recorder_turns_the_orm_spans_on",
    ) {
        return;
    }
    orm_spans_start_with(|| {
        super::recorder::enable_global();
    });
}
