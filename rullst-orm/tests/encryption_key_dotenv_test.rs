//! `#[orm(encrypted)]` fields read `RULLST_ENCRYPTION_KEY*` from `./.env`
//! when the process environment lacks them, without exporting the file.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::privacy::{decrypt_model_field, encrypt_model_field};

const KEYS: [&str; 3] = [
    "RULLST_ENCRYPTION_KEY",
    "RULLST_ENCRYPTION_KEY_ID",
    "RULLST_ENCRYPTION_KEYRING",
];

#[test]
fn encrypted_fields_use_the_key_from_dotenv() {
    let project = std::env::temp_dir().join(format!(
        "rullst-orm-dotenv-key-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join(".env"),
        "RULLST_ENCRYPTION_KEY=0123456789abcdef0123456789abcdef\n\
         RULLST_ENCRYPTION_KEY_ID=dotenv-2026\n",
    )
    .unwrap();
    // This is the only test in its process, so changing the working
    // directory and these variables races with no other test.
    for key in KEYS {
        unsafe { std::env::remove_var(key) };
    }
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project).unwrap();

    let encrypted = encrypt_model_field("secret", "accounts", "token").expect("key from .env");
    assert!(
        encrypted.starts_with("RULLST:v2:dotenv-2026:"),
        "the key id comes from .env: {encrypted}"
    );
    assert_eq!(
        decrypt_model_field(&encrypted, "accounts", "token").unwrap(),
        "secret"
    );
    for key in KEYS {
        assert!(std::env::var(key).is_err(), "{key} must not be exported");
    }

    // The process environment wins over the file.
    unsafe { std::env::set_var("RULLST_ENCRYPTION_KEY", "abcdef0123456789abcdef0123456789") };
    assert!(
        decrypt_model_field(&encrypted, "accounts", "token").is_err(),
        "the process key, not the .env key, is used"
    );
    unsafe { std::env::remove_var("RULLST_ENCRYPTION_KEY") };

    // Windows refuses to delete the process's working directory (os error 32).
    std::env::set_current_dir(original_dir).unwrap();
    std::fs::remove_dir_all(&project).unwrap();
}
