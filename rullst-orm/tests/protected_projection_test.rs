//! Generated audit, event, search and Redis hash projections must never carry
//! `#[orm(encrypted)]` or `#[orm(masked)]` plaintext. These checks need no
//! database, so they run under every feature combination.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::FromRow;

const DIAGNOSIS: &str = "F32.1 moderate depressive episode";
const RECOVERY_NOTE: &str = "call sister before discharge";
const CPF: &str = "123.456.789-00";
const INTERNAL_CODE: &str = "internal-7731";

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "protected_patients", auditable, searchable)]
struct Patient {
    id: i32,
    name: String,
    #[orm(encrypted)]
    diagnosis: String,
    #[orm(encrypted)]
    recovery_note: Option<String>,
    #[orm(masked)]
    cpf: String,
    #[orm(hidden)]
    internal_code: String,
}

fn patient() -> Patient {
    Patient {
        id: 7,
        name: "Ana".to_string(),
        diagnosis: DIAGNOSIS.to_string(),
        recovery_note: Some(RECOVERY_NOTE.to_string()),
        cpf: CPF.to_string(),
        internal_code: INTERNAL_CODE.to_string(),
    }
}

fn assert_no_plaintext(payload: &str) {
    for secret in [DIAGNOSIS, RECOVERY_NOTE, CPF, INTERNAL_CODE] {
        // Never format the values: they stand in for protected data.
        assert!(!payload.contains(secret), "a protected test value leaked");
    }
}

#[test]
fn audit_and_event_projection_redacts_encrypted_and_masked_values() {
    let payload: serde_json::Value =
        serde_json::from_str(&patient().to_json()).expect("to_json is valid JSON");
    assert_eq!(
        payload,
        serde_json::json!({
            "id": 7,
            "name": "Ana",
            "diagnosis": "***",
            "recovery_note": "***",
            "cpf": "***",
        })
    );
    assert_no_plaintext(&patient().to_json());
}

#[test]
fn search_document_omits_hidden_encrypted_and_masked_fields() {
    let document: serde_json::Value =
        serde_json::from_str(&patient().__rullst_search_json()).expect("search JSON");
    assert_eq!(document, serde_json::json!({"id": 7, "name": "Ana"}));
}

#[test]
fn redacted_changes_are_detected_without_exposing_values() {
    let previous = patient();
    assert!(patient().__rullst_redacted_changes(&previous).is_empty());

    let mut current = patient();
    current.name = "Ana Maria".to_string();
    assert!(current.__rullst_redacted_changes(&previous).is_empty());

    current.diagnosis = "Z00.0".to_string();
    current.recovery_note = None;
    current.cpf = "987.654.321-00".to_string();
    current.internal_code = "hidden fields are not audited".to_string();
    assert_eq!(
        current.__rullst_redacted_changes(&previous),
        vec!["diagnosis", "recovery_note", "cpf"]
    );
}

#[cfg(feature = "redis")]
#[test]
fn redis_hash_stores_encrypted_fields_as_authenticated_envelopes() {
    // This is the only test in this binary that reads the process-wide key
    // variables, so setting them here cannot race another reader.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef");
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "projection-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    let fields = patient()
        .__rullst_redis_hash_fields()
        .expect("serialize Redis hash");
    let hash: std::collections::HashMap<String, String> = fields
        .into_iter()
        .map(|(field, value)| (field.to_string(), value))
        .collect();
    for field in ["diagnosis", "recovery_note"] {
        let stored = &hash[field];
        assert!(
            stored.starts_with("\"RULLST:v2:projection-2026:"),
            "{field} must be stored as ciphertext: {stored}"
        );
        assert!(!stored.contains(DIAGNOSIS) && !stored.contains(RECOVERY_NOTE));
    }

    let restored = Patient::__rullst_from_redis_hash(&hash).expect("decrypt Redis hash");
    assert_eq!(restored.diagnosis, DIAGNOSIS);
    assert_eq!(restored.recovery_note.as_deref(), Some(RECOVERY_NOTE));
    assert_eq!(restored.cpf, CPF);

    let mut swapped = hash.clone();
    swapped.insert("recovery_note".to_string(), hash["diagnosis"].clone());
    assert!(
        Patient::__rullst_from_redis_hash(&swapped).is_err(),
        "an envelope bound to another column must not decrypt"
    );
}
