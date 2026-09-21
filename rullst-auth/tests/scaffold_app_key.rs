//! Published scaffold placeholders must never become usable session keys.
// TM-AUTH-02

#[test]
fn public_env_example_key_cannot_encrypt_or_decrypt_sessions() {
    // Exact public .env.example value emitted by the CLI, not a secret.
    let example = b"REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY";
    assert!(rullst_auth::validate_app_key(example).is_err());
    assert!(rullst_auth::encrypt_session(42, example).is_err());
    assert!(rullst_auth::decrypt_session("invalid-session", example).is_err());
    assert!(rullst_auth::validate_app_key(b"  replace_with_your_32_char_random_key  ").is_err());
}
