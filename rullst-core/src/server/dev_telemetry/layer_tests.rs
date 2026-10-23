//! A process whose subscriber never installed the ORM query layer says so.
//! Installation is process-wide, so the check runs in a child process that
//! runs no other test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::process::{Command, Stdio};

const CHILD_ENV: &str = "RULLST_ORM_LAYER_UNINSTALLED_CHILD";
const NAME: &str =
    "server::dev_telemetry::layer_tests::a_process_without_the_query_layer_reports_it";

#[test]
fn a_process_without_the_query_layer_reports_it() {
    if std::env::var_os(CHILD_ENV).is_some() {
        // Nothing in this fresh process installed the layer, and the default
        // registry keeps every span.
        tracing::subscriber::with_default(tracing_subscriber::registry(), || {
            assert_eq!(orm_unavailable_reason(), Some("subscriber_not_installed"));
        });
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NAME, "--test-threads=1"])
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
            panic!("the uninstalled-layer child exceeded its deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let stdout = reader.join().unwrap();
    assert!(status.success(), "{stdout}");
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
}
