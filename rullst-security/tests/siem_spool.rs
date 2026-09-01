use rullst_security::{DurableSiemSpool, LiveSecurityEvent};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_spool() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-security-public-siem-{}-{nonce}.spool",
        std::process::id()
    ))
}

#[test]
fn public_spool_survives_restart_and_exposes_bounded_receipts() {
    let path = temporary_spool();
    let receipt = DurableSiemSpool::try_open(&path)
        .and_then(|spool| {
            spool.append_local(LiveSecurityEvent::local(
                "RBAC_ACCESS_DENIED",
                "owner mismatch",
                "192.0.2.50",
            ))
        })
        .expect("public spool append should succeed");
    assert_eq!(receipt.sequence(), 1);

    let reopened = DurableSiemSpool::try_open(&path).expect("public spool should reopen");
    let events = reopened.read_local().expect("public spool should replay");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "RBAC_ACCESS_DENIED");
    assert!(!events[0].verified_hmac);
    assert_eq!(reopened.snapshot().expect("snapshot").records(), 1);
    std::fs::remove_file(path).expect("remove test spool");
}
