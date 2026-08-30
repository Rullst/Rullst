#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm};

const PRIMARY_KEY: &str = "0123456789abcdef0123456789abcdef";
const ROTATED_KEY: &str = "abcdef0123456789abcdef0123456789";

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "encrypted_records")]
struct EncryptedRecord {
    id: i32,
    #[orm(encrypted)]
    secret: String,
    #[orm(encrypted)]
    optional_secret: Option<String>,
    label: String,
}

fn configure_primary_key() {
    // This integration test is the only test in its process, so no other
    // thread reads these process-wide variables while they are changed.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", PRIMARY_KEY);
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "primary-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
}

fn configure_rotated_key() {
    // See the single-test process invariant in `configure_primary_key`.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", ROTATED_KEY);
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "rotated-2027");
        std::env::set_var(
            "RULLST_ENCRYPTION_KEYRING",
            format!(r#"{{"primary-2026":"{PRIMARY_KEY}"}}"#),
        );
    }
}

#[tokio::test]
async fn encrypted_fields_round_trip_rotate_and_reject_unsafe_queries() {
    configure_primary_key();
    Orm::init_with_options(
        "sqlite:file:encrypted_field_test.db?mode=memory&cache=shared",
        2,
        30,
    )
    .await
    .expect("ORM should initialize");
    let pool = Orm::pool().expect("ORM pool should exist");
    rullst_orm::_sqlx::query(
        "CREATE TABLE encrypted_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            secret TEXT NOT NULL,
            optional_secret TEXT,
            label TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .expect("encrypted_records should be created");

    let mut record = EncryptedRecord {
        id: 0,
        secret: "first secret".to_string(),
        optional_secret: Some("recovery phrase".to_string()),
        label: "account".to_string(),
    };
    record
        .save()
        .await
        .expect("encrypted insert should succeed");
    assert_eq!(record.secret, "first secret");

    let stored: (String, Option<String>) = rullst_orm::_sqlx::query_as(
        "SELECT secret, optional_secret FROM encrypted_records WHERE id = ?",
    )
    .bind(record.id)
    .fetch_one(pool)
    .await
    .expect("stored ciphertext should be readable");
    assert!(stored.0.starts_with("RULLST:v2:primary-2026:"));
    assert!(
        stored
            .1
            .as_deref()
            .is_some_and(|value| value.starts_with("RULLST:v2:primary-2026:"))
    );
    assert!(!stored.0.contains("first secret"));

    let loaded = EncryptedRecord::find(record.id)
        .await
        .expect("encrypted select should succeed")
        .expect("record should exist");
    assert_eq!(loaded.secret, "first secret");
    assert_eq!(loaded.optional_secret.as_deref(), Some("recovery phrase"));

    let rejected = EncryptedRecord::query()
        .where_eq("secret", "first secret")
        .get()
        .await
        .expect_err("randomized ciphertext cannot support equality filters");
    assert!(rejected.to_string().contains("blind-index"));

    let plucked = EncryptedRecord::query()
        .pluck_string("secret")
        .await
        .expect("non-null encrypted strings should decrypt when plucked");
    assert_eq!(plucked, vec!["first secret"]);

    configure_rotated_key();
    let mut loaded_with_old_key = EncryptedRecord::find(record.id)
        .await
        .expect("keyring should decrypt the old envelope")
        .expect("record should exist after rotation");
    assert_eq!(loaded_with_old_key.secret, "first secret");

    loaded_with_old_key.secret = "rotated secret".to_string();
    loaded_with_old_key
        .save()
        .await
        .expect("save should rewrite every encrypted field with the current key");
    let rotated: (String, Option<String>) = rullst_orm::_sqlx::query_as(
        "SELECT secret, optional_secret FROM encrypted_records WHERE id = ?",
    )
    .bind(record.id)
    .fetch_one(pool)
    .await
    .expect("rotated ciphertext should be readable");
    assert!(rotated.0.starts_with("RULLST:v2:rotated-2027:"));
    assert!(
        rotated
            .1
            .as_deref()
            .is_some_and(|value| value.starts_with("RULLST:v2:rotated-2027:"))
    );

    loaded_with_old_key
        .update_partial()
        .optional_secret(None)
        .save()
        .await
        .expect("nullable encrypted fields should support NULL");
    assert_eq!(
        EncryptedRecord::find(record.id)
            .await
            .expect("final select should succeed")
            .expect("record should exist")
            .optional_secret,
        None
    );

    rullst_orm::_sqlx::query("UPDATE encrypted_records SET optional_secret = secret WHERE id = ?")
        .bind(record.id)
        .execute(pool)
        .await
        .expect("test tampering should succeed");
    assert!(EncryptedRecord::find(record.id).await.is_err());
}
