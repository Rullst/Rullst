//! Decrypted `SecretString` values must stay out of audit rows, committed
//! events and Scout documents, and revision restore must keep the real value.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{
    Error, FromRow, ModelCommittedEvent, Orm, SearchEngine, SecretString, set_search_engine,
};
use std::sync::{Arc, Mutex};

const CPF: &str = "123.456.789-00";
const NEW_CPF: &str = "987.654.321-00";

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "secret_patients", auditable, searchable)]
struct SecretPatient {
    id: i32,
    name: String,
    cpf: SecretString,
}

type Captured = Arc<Mutex<Vec<String>>>;
/// `(event, old_values, new_values, restore_patch)` of one audit row.
type AuditRow = (String, Option<String>, Option<String>, Option<String>);

struct RecordingEngine(Captured);

#[rullst_orm::async_trait]
impl SearchEngine for RecordingEngine {
    async fn update(&self, _: &str, _: i32, payload: serde_json::Value) -> Result<(), Error> {
        self.0.lock().unwrap().push(payload.to_string());
        Ok(())
    }

    async fn delete(&self, _: &str, _: i32) -> Result<(), Error> {
        Ok(())
    }

    async fn search(&self, _: &str, _: &str) -> Result<Vec<i32>, Error> {
        Ok(Vec::new())
    }
}

struct RecordingObserver(Captured);

#[rullst_orm::async_trait]
impl SecretPatientObserver for RecordingObserver {
    async fn committed(&self, event: &ModelCommittedEvent) -> Result<(), Error> {
        self.0.lock().unwrap().push(event.payload.clone());
        Ok(())
    }
}

fn assert_no_plaintext(source: &str, payload: &str) {
    for secret in [CPF, NEW_CPF] {
        assert!(
            !payload.contains(secret),
            "{source} leaked {secret}: {payload}"
        );
    }
}

async fn latest_update(pool: &rullst_orm::RullstPool, id: i32) -> (i32, String) {
    rullst_orm::_sqlx::query_as(
        "SELECT id, new_values FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind("secret_patients")
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read latest update audit")
}

#[tokio::test]
async fn secret_strings_never_reach_audit_events_or_search() {
    // This is the only test in this binary, so no other thread reads the
    // process-wide key variables while they are set.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef");
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "secret-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    Orm::init_with_options(
        "sqlite:file:secret_string_audit_test.db?mode=memory&cache=shared",
        2,
        30,
    )
    .await
    .expect("initialize secret audit database");
    let pool = Orm::pool().expect("pool");
    rullst_orm::_sqlx::query(
        "CREATE TABLE secret_patients (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, cpf TEXT NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create secret patients");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");

    let documents = Captured::default();
    let events = Captured::default();
    set_search_engine(RecordingEngine(documents.clone())).expect("configure Scout");
    SecretPatient::observe(Arc::new(RecordingObserver(events.clone())));

    let context = AuditContext::system("secret-string-test").expect("audit context");
    let id = with_audit_context(context, async {
        let mut patient = SecretPatient {
            id: 0,
            name: "Ana".to_string(),
            cpf: SecretString::new(CPF),
        };
        patient.save().await.expect("create patient");

        patient.name = "Ana Maria".to_string();
        patient.save().await.expect("update name only");
        let name_revision = latest_update(pool, patient.id).await;
        assert!(name_revision.1.contains("Ana Maria"));

        patient.cpf = SecretString::new(NEW_CPF);
        patient.save().await.expect("update secret only");
        let secret_revision = latest_update(pool, patient.id).await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&secret_revision.1).unwrap(),
            serde_json::json!({"cpf": "***"}),
            "a secret-only change must still be audited, as a redacted value"
        );
        assert!(matches!(
            patient
                .restore_revision(secret_revision.0, "restore withheld value")
                .await,
            Err(Error::Validation(_))
        ));

        let restored = patient
            .restore_revision(name_revision.0, "restore visible field")
            .await
            .expect("restore through the encrypted cache form");
        assert_eq!(restored.name, "Ana");
        assert_eq!(restored.cpf.reveal_audited(), NEW_CPF);
        let reloaded = SecretPatient::find(restored.id)
            .await
            .expect("reload patient")
            .expect("patient exists");
        assert_eq!(reloaded.cpf.reveal_audited(), NEW_CPF);

        restored.delete().await.expect("delete patient");
        restored.id
    })
    .await;

    let rows: Vec<AuditRow> =
        rullst_orm::_sqlx::query_as(
            "SELECT event, old_values, new_values, restore_patch FROM rullst_audits WHERE model_type = ? AND model_id = ?",
        )
        .bind("secret_patients")
        .bind(id)
        .fetch_all(pool)
        .await
        .expect("read audit rows");
    assert!(rows.len() >= 5);
    for (event, old_values, new_values, restore_patch) in &rows {
        for payload in [old_values, new_values, restore_patch]
            .into_iter()
            .flatten()
        {
            assert_no_plaintext(&format!("audit {event}"), payload);
        }
    }
    for payload in events.lock().unwrap().iter() {
        assert_no_plaintext("committed event", payload);
        assert!(payload.contains("\"cpf\":\"***\""));
    }
    let documents = documents.lock().unwrap().clone();
    assert!(!documents.is_empty());
    for document in &documents {
        assert_no_plaintext("Scout document", document);
        assert!(!document.contains("cpf"));
    }
}
