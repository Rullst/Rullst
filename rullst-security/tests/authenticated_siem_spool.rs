use rullst_security::{AuthenticatedSiemSpool, LiveSecurityEvent, SiemIntegrityKey, SiemKeyRing};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-security-public-auth-siem-{}-{nonce}.spool",
        std::process::id()
    ))
}

fn integrity_key() -> SiemIntegrityKey {
    SiemIntegrityKey::try_new("public-v1", (0_u8..32).collect::<Vec<_>>())
        .expect("valid public test key")
}

#[test]
fn public_authenticated_spool_survives_restart_and_verifies_provenance() {
    let path = test_path();
    {
        let keys = SiemKeyRing::try_new(integrity_key(), []).expect("valid key ring");
        let spool = AuthenticatedSiemSpool::try_open(&path, keys).expect("open public spool");
        let receipt = spool
            .append_local(LiveSecurityEvent::local(
                "RBAC_ACCESS_DENIED",
                "owner mismatch",
                "192.0.2.80",
            ))
            .expect("append public event");
        assert_eq!(receipt.sequence(), 1);
    }

    let keys = SiemKeyRing::try_new(integrity_key(), []).expect("valid restart key ring");
    let reopened = AuthenticatedSiemSpool::try_open(&path, keys).expect("reopen public spool");
    let events = reopened.read_verified().expect("verify public event");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "RBAC_ACCESS_DENIED");
    assert!(events[0].verified_hmac);
    std::fs::remove_file(path).expect("remove public test spool");
}
