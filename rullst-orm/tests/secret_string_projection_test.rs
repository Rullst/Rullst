//! `SecretString` model fields must never be serialized as plaintext by
//! generated projections, while the query-cache form still round-trips.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, SecretString};

const CPF: &str = "123.456.789-00";
const BACKUP_CODE: &str = "backup-4471-9920";

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "secret_customers", auditable, searchable)]
struct SecretCustomer {
    id: i32,
    name: String,
    cpf: SecretString,
    backup_code: Option<rullst_orm::privacy::SecretString>,
}

fn customer() -> SecretCustomer {
    SecretCustomer {
        id: 3,
        name: "Ana".to_string(),
        cpf: SecretString::new(CPF),
        backup_code: Some(SecretString::new(BACKUP_CODE)),
    }
}

#[test]
fn audit_event_and_search_projections_redact_secret_strings() {
    let payload = customer().to_json();
    assert!(!payload.contains(CPF) && !payload.contains(BACKUP_CODE));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&payload).unwrap(),
        serde_json::json!({"id": 3, "name": "Ana", "cpf": "***", "backup_code": "***"})
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&customer().__rullst_search_json()).unwrap(),
        serde_json::json!({"id": 3, "name": "Ana"})
    );
}

#[test]
fn secret_string_changes_are_detected_for_the_audit_trail() {
    let previous = customer();
    let mut current = customer();
    assert!(current.__rullst_redacted_changes(&previous).is_empty());
    current.cpf = SecretString::new("987.654.321-00");
    current.backup_code = None;
    assert_eq!(
        current.__rullst_redacted_changes(&previous),
        vec!["cpf", "backup_code"]
    );
}

#[test]
fn query_cache_json_holds_ciphertext_and_restores_the_real_value() {
    // This is the only test in this binary that reads the key variables.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef");
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "cache-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    let cached = SecretCustomer::to_cache_json_array(&[customer()]);
    assert!(!cached.contains(CPF) && !cached.contains(BACKUP_CODE));
    assert_eq!(cached.matches("RULLST:v2:cache-2026:").count(), 2);

    let restored = SecretCustomer::from_cache_json_array(&cached).expect("decrypt cache");
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].cpf.reveal_audited(), CPF);
    assert_eq!(
        restored[0]
            .backup_code
            .as_ref()
            .map(SecretString::reveal_audited),
        Some(BACKUP_CODE)
    );

    let single = customer().to_cache_json();
    assert!(!single.contains(CPF));
    assert_eq!(
        SecretCustomer::from_cache_json(&single)
            .expect("decrypt single cache entry")
            .cpf,
        SecretString::new(CPF)
    );
}
