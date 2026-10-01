//! `redact_secrets` must count masked log records only as log redactions.
//!
//! This binary holds a single test because it asserts exact deltas of the
//! process-global `SecurityStore`, which parallel tests would disturb.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_security::dlp::mask_response_payload;
use rullst_security::log_redactor::redact_secrets;
use rullst_security::telemetry::SecurityStore;

fn dlp_events(store: &SecurityStore) -> usize {
    store
        .live_events
        .lock()
        .expect("telemetry lock")
        .iter()
        .filter(|event| event.event_type == "DLP_SECRET_LEAK_PREVENTED")
        .count()
}

#[test]
fn log_redaction_is_not_reported_as_a_blocked_http_response() {
    let store = SecurityStore::global();
    let before = store.snapshot();

    for _ in 0..60 {
        let logged = redact_secrets("connecting to postgres://app:pw@db/app");
        assert!(logged == "connecting to postgres://app:*****@db/app");
    }
    let after = store.snapshot();
    assert_eq!(after.dlp_secrets_masked, before.dlp_secrets_masked);
    assert_eq!(after.log_redactions, before.log_redactions + 60);
    assert_eq!(
        dlp_events(store),
        0,
        "log lines must not fill the live feed"
    );

    // Response DLP keeps its own counter and live event.
    let (_, masked) = mask_response_payload(b"key AKIAIOSFODNN7EXAMPLE");
    assert!(masked);
    assert_eq!(
        store.snapshot().dlp_secrets_masked,
        before.dlp_secrets_masked + 1
    );
    assert_eq!(dlp_events(store), 1);
}
