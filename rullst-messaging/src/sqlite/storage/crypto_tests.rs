#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::Namespace;

#[test]
fn an_authentic_envelope_without_the_magic_prefix_is_refused() {
    let keyring = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap());
    let namespace = Namespace::try_new("crypto").unwrap();
    let binding = MessageBinding::message(
        &namespace,
        "events",
        1,
        "msg_0123456789abcdef0123456789abcdef",
        "ready",
        "application/json",
        1_800_000_000_000,
    );
    let sealed = seal(keyring.primary(), binding, b"payload").unwrap();
    let (key_id, plaintext) = open(&keyring, binding, &sealed).unwrap();
    assert_eq!(
        (key_id, plaintext.as_slice()),
        ("storage", b"payload".as_slice())
    );
    // The prefix is not authenticated data, so only the explicit check refuses it.
    let mut changed = sealed;
    changed[0] ^= 1;
    assert_eq!(
        open(&keyring, binding, &changed).err(),
        Some(MessagingError::StorageAuthenticationFailed)
    );
}
