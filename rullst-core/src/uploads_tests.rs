//! Unit tests of the in-memory upload admission contract.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn text_policy() -> UploadPolicy {
    UploadPolicy::try_new(1_024, [UploadKind::PlainText]).expect("test upload policy")
}

#[test]
// TM-ACADEMY-09
fn upload_policy_rejects_spoofing_active_content_and_unsafe_names() {
    let policy = text_policy();
    assert!(matches!(
        policy.admit(
            "school-1",
            "note.txt",
            "text/plain",
            b"<svg onload=alert(1)>"
        ),
        Err(UploadError::ActiveContentDenied)
    ));
    assert!(matches!(
        policy.admit("school-1", "../note.txt", "text/plain", b"safe"),
        Err(UploadError::InvalidFileName)
    ));
    assert!(matches!(
        policy.admit("school-1", "note.pdf", "application/pdf", b"plain text"),
        Err(UploadError::MediaTypeDenied)
    ));
    assert!(matches!(
        policy.admit("../school", "note.txt", "text/plain", b"safe"),
        Err(UploadError::InvalidTenant)
    ));
    for dot_only in [".", "..", "..."] {
        assert!(matches!(
            policy.admit(dot_only, "note.txt", "text/plain", b"safe"),
            Err(UploadError::InvalidTenant)
        ));
    }
}

#[test]
fn quarantine_requires_a_clean_digest_bound_scan() {
    let policy = text_policy();
    let bytes = b"bounded learner submission";
    let admitted = policy
        .admit("school-1", "answer.txt", "text/plain", bytes)
        .expect("admitted text");
    assert!(
        admitted
            .quarantine_key()
            .starts_with("quarantine/school-1/")
    );
    let verdict = OfflineMockScanner
        .scan(&admitted, bytes)
        .expect("offline clean scan");
    let released = admitted.release(verdict).expect("clean release");
    assert!(released.object_key.starts_with("accepted/school-1/"));
    assert_eq!(released.byte_len, bytes.len());

    let infected_bytes = b"EICAR deterministic fixture";
    let infected = policy
        .admit("school-1", "fixture.txt", "text/plain", infected_bytes)
        .expect("admitted infected fixture");
    let verdict = OfflineMockScanner
        .scan(&infected, infected_bytes)
        .expect("offline infected scan");
    assert!(matches!(
        infected.release(verdict),
        Err(UploadError::MalwareDetected)
    ));
}

#[test]
fn unavailable_or_tampered_scan_fails_closed() {
    let admitted = text_policy()
        .admit("school-1", "answer.txt", "text/plain", b"original")
        .expect("admitted text");
    assert!(matches!(
        OfflineMockScanner.scan(&admitted, b"changed"),
        Err(UploadError::InvalidSize | UploadError::ScanUnavailable)
    ));
    assert!(matches!(
        admitted.release(ScanVerdict::Unavailable {
            engine: "scanner".to_string(),
            reason: "offline".to_string(),
        }),
        Err(UploadError::ScanUnavailable)
    ));
}
