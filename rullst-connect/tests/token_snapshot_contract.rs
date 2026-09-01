use rullst_connect::{
    EncryptedTokenSnapshot, RefreshableTokenState, TokenSnapshotBinding, TokenSnapshotError,
    TokenSnapshotKey,
};
use secrecy::{ExposeSecret, SecretString};

#[test]
fn public_snapshot_contract_survives_storage_and_authenticates_ownership() {
    let state = RefreshableTokenState::try_new(
        "provider-user",
        SecretString::from("access-marker".to_string()),
        SecretString::from("refresh-marker".to_string()),
        1_000,
        3_600,
    )
    .expect("valid token state");
    let binding = TokenSnapshotBinding::try_new("github", "application-user")
        .expect("trusted application binding");
    let key = TokenSnapshotKey::try_new("primary", [23; 32]).expect("valid key");

    let sealed = EncryptedTokenSnapshot::seal(&state, &key, &binding).expect("state is encrypted");
    let stored_value = sealed.as_str().to_string();
    assert!(!stored_value.contains("access-marker"));
    assert!(!stored_value.contains("refresh-marker"));

    let restored = EncryptedTokenSnapshot::try_from_envelope(stored_value)
        .expect("persisted envelope remains valid")
        .open(&key, &binding)
        .expect("correct key and ownership binding authenticate");
    assert_eq!(restored.provider_user_id(), "provider-user");
    assert_eq!(restored.access_token().expose_secret(), "access-marker");
    assert_eq!(restored.refresh_token().expose_secret(), "refresh-marker");
    assert_eq!(restored.expires_at(), 4_600);
}

#[test]
fn copied_snapshot_cannot_be_opened_for_another_application_account() {
    let state = RefreshableTokenState::try_new(
        "provider-user",
        SecretString::from("access-marker".to_string()),
        SecretString::from("refresh-marker".to_string()),
        1_000,
        3_600,
    )
    .expect("valid token state");
    let owner = TokenSnapshotBinding::try_new("github", "owner").expect("valid owner");
    let attacker = TokenSnapshotBinding::try_new("github", "other").expect("valid owner");
    let key = TokenSnapshotKey::try_new("primary", [19; 32]).expect("valid key");
    let sealed = EncryptedTokenSnapshot::seal(&state, &key, &owner).expect("state is encrypted");

    assert!(matches!(
        sealed.open(&key, &attacker),
        Err(TokenSnapshotError::AuthenticationFailed)
    ));
}
