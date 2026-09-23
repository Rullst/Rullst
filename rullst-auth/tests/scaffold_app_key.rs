//! Published scaffold placeholders must never become usable session keys.
// TM-AUTH-02

#[test]
fn public_env_example_key_cannot_encrypt_or_decrypt_sessions() {
    // Exact public scaffold and documentation values, not secrets.
    for example in [
        b"REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY".as_slice(),
        b"REPLACE_WITH_A_STRONG_RANDOM_KEY".as_slice(),
        b"replace-with-at-least-32-random-bytes".as_slice(),
    ] {
        assert!(matches!(
            rullst_auth::validate_app_key(example),
            Err(rullst_auth::AuthError::MissingAppKey(_))
        ));
        assert!(matches!(
            rullst_auth::encrypt_session(42, example),
            Err(rullst_auth::AuthError::MissingAppKey(_))
        ));
        assert!(matches!(
            rullst_auth::decrypt_session("invalid-session", example),
            Err(rullst_auth::AuthError::MissingAppKey(_))
        ));
        #[cfg(feature = "jwt")]
        assert!(matches!(
            rullst_auth::JwtSigningKey::new("active", example),
            Err(rullst_auth::JwtError::WeakSigningKey)
        ));
    }
    assert!(rullst_auth::validate_app_key(b"  replace_with_your_32_char_random_key  ").is_err());
}
