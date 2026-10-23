//! Envelope, policy and label bounds of encrypted document snapshots.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::*;
use crate::polyglot::{CollectionName, DocumentId, DocumentRepository, MockDocumentStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Note {
    sequence: u64,
    label: String,
}

const KEY_ID: &str = "backup-2026-09";

fn key() -> DocumentRecoveryKey {
    DocumentRecoveryKey::try_new(KEY_ID, [9; 32]).expect("valid key")
}

fn policy(max_snapshot_bytes: usize) -> DocumentRecoveryPolicy {
    DocumentRecoveryPolicy::try_new(10, 10, max_snapshot_bytes).expect("valid policy")
}

async fn sealed(
    label_len: usize,
    policy: DocumentRecoveryPolicy,
) -> Result<String, DocumentRecoveryError> {
    let source = MockDocumentStore::<Note>::new();
    let collection = CollectionName::new("notes").expect("collection");
    let note = Note {
        sequence: 1,
        label: "x".repeat(label_len),
    };
    source
        .create(&collection, &DocumentId::new("note-1").expect("id"), &note)
        .await
        .expect("seed");
    let binding = DocumentRecoveryBinding::try_new("academy", collection).expect("binding");
    export_document_snapshot(&source, &binding, &key(), policy)
        .await
        .map(|snapshot| snapshot.as_str().to_owned())
}

fn ciphertext_len(envelope: &str) -> usize {
    let encoded = envelope.rsplit(':').next().expect("ciphertext");
    URL_SAFE_NO_PAD.decode(encoded).expect("base64url").len()
}

fn envelope(ciphertext: &str) -> String {
    let nonce = URL_SAFE_NO_PAD.encode([0_u8; 12]);
    format!("RULLST-ORM-SNAPSHOT:v1:{KEY_ID}:{nonce}:{ciphertext}")
}

fn check(envelope: &str, policy: DocumentRecoveryPolicy) -> Result<(), DocumentRecoveryError> {
    EncryptedDocumentSnapshot::try_from_envelope(envelope, policy).map(|_| ())
}

#[tokio::test]
async fn a_snapshot_of_exactly_the_policy_size_seals_validates_and_opens() {
    let probe = sealed(10, DocumentRecoveryPolicy::default())
        .await
        .expect("probe");
    // The ciphertext is the plaintext plus a 16-byte tag.
    let plaintext = ciphertext_len(&probe) - 16;
    let exact = sealed(10 + 1_024 - plaintext, policy(1_024))
        .await
        .expect("a snapshot of exactly 1 KiB seals");
    assert_eq!(ciphertext_len(&exact), 1_024 + 16);
    check(&exact, policy(1_024)).expect("and validates");
    let collection = CollectionName::new("notes").expect("collection");
    let binding = DocumentRecoveryBinding::try_new("academy", collection).expect("binding");
    let destination = MockDocumentStore::<Note>::new();
    let snapshot = EncryptedDocumentSnapshot::try_from_envelope(exact, policy(1_024)).unwrap();
    let report =
        restore_document_snapshot(&destination, &snapshot, &binding, &key(), policy(1_024))
            .await
            .expect("and opens");
    assert_eq!(report.inserted(), 1);
    assert!(matches!(
        sealed(10 + 1_025 - plaintext, policy(1_024)).await,
        Err(DocumentRecoveryError::CapacityExceeded)
    ));
}

#[tokio::test]
async fn envelopes_must_keep_their_prefix_shape_and_key_label() {
    let valid = sealed(10, policy(1_024)).await.expect("sealed");
    check(&valid, policy(1_024)).expect("valid envelope");
    let forged_prefix = valid.replacen("RULLST-ORM-SNAPSHOT", "RULLST-ORM-SNAPSHOX", 1);
    let extra_part = format!("{valid}:extra");
    let bad_label = valid.replacen(KEY_ID, "backup/2026", 1);
    let tag = URL_SAFE_NO_PAD.encode([0_u8; 16]);
    let empty_nonce = format!("RULLST-ORM-SNAPSHOT:v1:{KEY_ID}::{tag}");
    let empty_ciphertext = envelope("");
    for envelope in [
        forged_prefix,
        extra_part,
        bad_label,
        empty_nonce,
        empty_ciphertext,
    ] {
        assert!(
            matches!(
                check(&envelope, policy(1_024)),
                Err(DocumentRecoveryError::InvalidEnvelope)
            ),
            "{envelope}"
        );
    }
}

#[test]
fn ciphertext_lengths_and_encodings_are_checked_exactly() {
    let policy = policy(1_024);
    // A tag alone (16 bytes) is the shortest ciphertext.
    check(&envelope(&URL_SAFE_NO_PAD.encode([0_u8; 16])), policy).expect("16 bytes");
    assert!(check(&envelope(&URL_SAFE_NO_PAD.encode([0_u8; 15])), policy).is_err());
    // 1024 + 16 bytes is the longest; one more byte is rejected.
    check(
        &envelope(&URL_SAFE_NO_PAD.encode(vec![0_u8; 1_040])),
        policy,
    )
    .expect("1040 bytes");
    assert!(
        check(
            &envelope(&URL_SAFE_NO_PAD.encode(vec![0_u8; 1_041])),
            policy
        )
        .is_err()
    );
    // Complete quanta and every final base64url digit kind are accepted.
    let quantum = URL_SAFE_NO_PAD.encode([0_u8; 18]);
    assert_eq!(quantum.len() % 4, 0);
    check(&envelope(&quantum), policy).expect("complete quanta");
    for last in ['-', '_'] {
        let ciphertext = format!("{}{last}", &quantum[..quantum.len() - 1]);
        check(&envelope(&ciphertext), policy).expect("dash and underscore digits");
    }
    let partial = URL_SAFE_NO_PAD.encode([0_u8; 17]);
    assert_eq!(partial.len() % 4, 3);
    for last in ['0', '4', '8'] {
        let ciphertext = format!("{}{last}", &partial[..partial.len() - 1]);
        check(&envelope(&ciphertext), policy).expect("decimal digits");
    }
}

#[test]
fn the_envelope_size_bound_comes_before_version_checks() {
    let policy = policy(1_024);
    // encoded(1024 + 16) + 256 = 1644 bytes.
    let head = "RULLST-ORM-SNAPSHOT:v2:backup:AAAA:";
    let at_bound = format!("{head}{}", "A".repeat(1_644 - head.len()));
    assert!(matches!(
        check(&at_bound, policy),
        Err(DocumentRecoveryError::UnsupportedVersion)
    ));
    let over = format!("{at_bound}A");
    assert!(matches!(
        check(&over, policy),
        Err(DocumentRecoveryError::InvalidEnvelope)
    ));
}

#[test]
fn policies_keys_and_labels_are_bounded() {
    assert!(DocumentRecoveryPolicy::try_new(500, 100_000, 64 * 1024 * 1024).is_ok());
    for (page, documents, bytes) in [
        (501, 1, 1_024),
        (1, 100_001, 1_024),
        (1, 1, 64 * 1024 * 1024 + 1),
        (1, 1, 1_023),
    ] {
        assert!(
            matches!(
                DocumentRecoveryPolicy::try_new(page, documents, bytes),
                Err(DocumentRecoveryError::InvalidPolicy)
            ),
            "{page} {documents} {bytes}"
        );
    }
    assert!(DocumentRecoveryKey::try_new("k".repeat(64), [1; 32]).is_ok());
    for invalid in [String::new(), "k".repeat(65), "bad/key".to_string()] {
        assert!(
            matches!(
                DocumentRecoveryKey::try_new(invalid.clone(), [1; 32]),
                Err(DocumentRecoveryError::InvalidKey)
            ),
            "{invalid}"
        );
    }
    assert_eq!(
        format!("{:?}", key()),
        "DocumentRecoveryKey { key_id: \"backup-2026-09\", key: \"[REDACTED]\" }"
    );
}
