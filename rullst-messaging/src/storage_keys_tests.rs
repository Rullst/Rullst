#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[test]
fn debug_output_names_key_ids_and_hides_key_bytes() {
    let primary = MessagingStorageKey::try_new("primary", [3; 32]).unwrap();
    assert_eq!(
        format!("{primary:?}"),
        "MessagingStorageKey { key_id: \"primary\", key: \"[REDACTED]\" }"
    );
    let keyring = MessagingKeyring::new(primary)
        .with_decryption_key(MessagingStorageKey::try_new("previous", [4; 32]).unwrap())
        .unwrap();
    assert_eq!(
        format!("{keyring:?}"),
        "MessagingKeyring { primary_key_id: \"primary\", key_count: 2, keys: \"[REDACTED]\" }"
    );
}
