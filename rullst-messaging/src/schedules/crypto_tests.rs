#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::MessagingStorageKey;

fn keys() -> MessagingKeyring {
    MessagingKeyring::new(MessagingStorageKey::try_new("k", [9; 32]).unwrap())
}

#[test]
fn the_largest_plaintext_round_trips() {
    let keys = keys();
    let largest = vec![7; 131_072];
    let sealed = seal(&keys, "school", "occurrence", "one", &largest).unwrap();
    assert_eq!(sealed.len(), 131_110);
    assert_eq!(
        open(&keys, "school", "occurrence", "one", &sealed)
            .unwrap()
            .as_slice(),
        largest.as_slice()
    );
}

#[test]
fn the_shortest_envelope_round_trips() {
    // A one-byte key ID and an empty plaintext: magic, length, ID, nonce and tag.
    let keys = keys();
    let sealed = seal(&keys, "school", "occurrence", "one", b"").unwrap();
    assert_eq!(sealed.len(), 38);
    assert!(
        open(&keys, "school", "occurrence", "one", &sealed)
            .unwrap()
            .is_empty()
    );
}
