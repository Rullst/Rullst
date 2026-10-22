use super::{RowSnapshot, UNKNOWN, fingerprint, fingerprint_json};
use crate::SecretString;
use crate::privacy::{with_redacted_secrets, with_secret_fingerprints};
use std::hash::{BuildHasher, Hasher};

#[test]
fn a_default_snapshot_is_untracked_and_marks_every_column_changed() {
    let snapshot = RowSnapshot::default();
    assert!(!snapshot.is_tracked());
    assert!(snapshot.__is_changed(0, 42));
    assert!(snapshot.__any_changed(&[42]));
    assert!(!snapshot.__any_changed(&[]));
}

#[test]
fn a_captured_snapshot_reports_exactly_the_changed_columns() {
    let snapshot = RowSnapshot::__from_fingerprints(&[11, 22, 33]);
    assert!(snapshot.is_tracked());
    assert!(!snapshot.__is_changed(0, 11));
    assert!(snapshot.__is_changed(1, 23));
    assert!(!snapshot.__is_changed(2, 33));
    // A column the snapshot does not know is always written.
    assert!(snapshot.__is_changed(3, 44));
    assert!(!snapshot.__any_changed(&[11, 22, 33]));
    assert!(snapshot.__any_changed(&[11, 22, 34]));
    assert!(snapshot.__any_changed(&[10, 22, 33]));
}

#[test]
fn an_unknown_fingerprint_on_either_side_never_matches() {
    let snapshot = RowSnapshot::__from_fingerprints(&[UNKNOWN, 7]);
    assert!(snapshot.__is_changed(0, UNKNOWN));
    assert!(snapshot.__is_changed(0, 7));
    assert!(snapshot.__is_changed(1, UNKNOWN));
    assert!(!snapshot.__is_changed(1, 7));
}

#[test]
fn snapshots_are_bookkeeping_that_compares_and_hashes_alike() {
    let tracked = RowSnapshot::__from_fingerprints(&[1, 2]);
    let untracked = RowSnapshot::default();
    assert_eq!(tracked, untracked);
    assert_eq!(tracked.cmp(&untracked), std::cmp::Ordering::Equal);
    assert_eq!(
        tracked.partial_cmp(&untracked),
        Some(std::cmp::Ordering::Equal)
    );

    // Hashing a snapshot writes nothing.
    let keys = std::collections::hash_map::RandomState::new();
    assert_eq!(keys.hash_one(&tracked), keys.build_hasher().finish());

    assert_eq!(format!("{tracked:?}"), "RowSnapshot { tracked: true }");
    assert_eq!(format!("{untracked:?}"), "RowSnapshot { tracked: false }");
    assert!(tracked.clone().is_tracked());
}

#[test]
fn fingerprints_are_stable_per_value_and_distinguish_values() {
    assert_eq!(fingerprint("draft"), fingerprint("draft"));
    assert_ne!(fingerprint("draft"), fingerprint("final"));
    assert_ne!(
        fingerprint(&None::<String>),
        fingerprint(&Some(String::new()))
    );
    assert_ne!(fingerprint(&1_i64), fingerprint(&2_i64));
    assert_ne!(fingerprint(&1_i64), UNKNOWN);
    let json = serde_json::json!({ "a": 1, "b": [true, null] });
    assert_eq!(fingerprint(&json), fingerprint(&json.clone()));
    assert_ne!(
        fingerprint(&json),
        fingerprint(&serde_json::json!({ "a": 1 }))
    );
}

struct Unserializable;

impl serde::Serialize for Unserializable {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("cannot serialize"))
    }
}

#[test]
fn a_value_that_cannot_be_serialized_has_the_unknown_fingerprint() {
    assert_eq!(fingerprint(&Unserializable), UNKNOWN);
}

#[test]
fn secret_strings_fingerprint_their_plaintext_only_inside_the_scope() {
    let first = SecretString::new("correct horse");
    let same = SecretString::new("correct horse");
    let other = SecretString::new("battery staple");
    assert_eq!(fingerprint(&first), fingerprint(&same));
    assert_ne!(fingerprint(&first), fingerprint(&other));
    assert_ne!(fingerprint(&first), UNKNOWN);

    // The fingerprint scope takes precedence over a redacted projection and
    // ends with its closure.
    let inside = with_redacted_secrets(|| {
        with_secret_fingerprints(|| serde_json::to_string(&first).expect("serialize"))
    });
    assert_eq!(inside, "\"correct horse\"");
    let after = with_redacted_secrets(|| serde_json::to_string(&first).expect("serialize"));
    assert_eq!(after, "\"***\"");
    let nested = with_secret_fingerprints(|| {
        with_secret_fingerprints(|| ());
        with_redacted_secrets(|| serde_json::to_string(&first).expect("serialize"))
    });
    assert_eq!(nested, "\"correct horse\"");
}

#[test]
fn json_fingerprints_ignore_object_key_order_at_every_level() {
    let stored: serde_json::Value =
        serde_json::from_str(r#"{"b":1,"a":{"y":[{"q":1,"p":2}],"x":true}}"#).expect("parse");
    let reordered = serde_json::json!({ "a": { "x": true, "y": [{ "p": 2, "q": 1 }] }, "b": 1 });
    assert_eq!(fingerprint_json(&stored), fingerprint_json(&reordered));
    let changed = serde_json::json!({ "a": { "x": true, "y": [{ "p": 3, "q": 1 }] }, "b": 1 });
    assert_ne!(fingerprint_json(&stored), fingerprint_json(&changed));
    // Array order is data.
    let swapped = serde_json::json!([2, 1]);
    assert_ne!(
        fingerprint_json(&serde_json::json!([1, 2])),
        fingerprint_json(&swapped)
    );
    assert_eq!(
        fingerprint_json(&crate::Json(reordered.clone())),
        fingerprint_json(&stored)
    );
    assert_eq!(fingerprint_json(&Unserializable), UNKNOWN);
    // A JSON column holding a secret fingerprints its plaintext, not a
    // randomized envelope.
    let secret = vec![SecretString::new("pin")];
    assert_eq!(
        fingerprint_json(&secret),
        fingerprint_json(&vec![SecretString::new("pin")])
    );
    assert_ne!(fingerprint_json(&secret), UNKNOWN);
}
