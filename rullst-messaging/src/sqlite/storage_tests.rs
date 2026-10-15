#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

const PRIMARY: &str = "rullst.messaging.encrypted.v1:storage";

fn namespace() -> Namespace {
    Namespace::try_new("storage-profile").unwrap()
}

fn binding(namespace: &Namespace) -> MessageBinding<'_> {
    MessageBinding::message(
        namespace,
        "events",
        1,
        "msg_0123456789abcdef0123456789abcdef",
        "ready",
        "application/json",
        1_800_000_000_000,
    )
}

fn encrypted() -> StorageProfile {
    let keyring = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap())
        .with_decryption_key(MessagingStorageKey::try_new("other", [72; 32]).unwrap())
        .unwrap();
    StorageProfile::encrypted(keyring)
}

#[test]
fn plaintext_profiles_accept_only_an_empty_probe() {
    let plaintext = StorageProfile::plaintext();
    assert_eq!(plaintext.open_probe(&namespace(), &[]), Ok(String::new()));
    assert_eq!(
        plaintext.open_probe(&namespace(), &[0]),
        Err(MessagingError::ConfigurationConflict)
    );
}

#[test]
fn markers_name_the_primary_key_and_require_a_known_key() {
    let profile = encrypted();
    assert!(profile.is_primary_marker(PRIMARY));
    assert!(!profile.is_primary_marker("rullst.messaging.encrypted.v1:other"));
    assert_eq!(profile.ensure_key_available(PRIMARY), Ok(()));
    assert_eq!(
        profile.ensure_key_available("rullst.messaging.encrypted.v1:missing"),
        Err(MessagingError::StorageKeyUnavailable)
    );
    assert_eq!(
        StorageProfile::plaintext().ensure_key_available(PRIMARY),
        Err(MessagingError::ConfigurationConflict)
    );
}

#[test]
fn decoding_needs_the_marker_of_the_sealing_key() {
    let profile = encrypted();
    let namespace = namespace();
    let (marker, sealed) = profile
        .encode_message(binding(&namespace), &MessageHeaders::new(), &[7; 16])
        .unwrap();
    assert_eq!(marker, PRIMARY);
    let (_, payload) = profile
        .decode_message(binding(&namespace), marker, sealed.clone(), 16)
        .unwrap();
    assert_eq!(payload, vec![7; 16]);
    assert_eq!(
        profile
            .decode_message(
                binding(&namespace),
                "rullst.messaging.encrypted.v1:other".to_owned(),
                sealed,
                16,
            )
            .err(),
        Some(MessagingError::StorageAuthenticationFailed)
    );
}

#[test]
fn a_four_byte_plaintext_is_read_as_empty_headers() {
    let profile = encrypted();
    let StorageProfile::Encrypted(keyring) = &profile else {
        panic!("encrypted profile");
    };
    let namespace = namespace();
    let sealed = seal(keyring.primary(), binding(&namespace), &[0; 4]).unwrap();
    assert_eq!(
        profile
            .decode_message(binding(&namespace), PRIMARY.to_owned(), sealed, 16)
            .err(),
        Some(MessagingError::CorruptStorage {
            context: "message header encoding"
        })
    );
}
