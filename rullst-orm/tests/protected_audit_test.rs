//! Encrypted and masked values must stay out of audit rows, committed-event
//! payloads and Scout documents written by generated save/delete paths.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, FromRow, ModelCommittedEvent, Orm, SearchEngine, set_search_engine};
use std::sync::{Arc, Mutex};

const DIAGNOSIS: &str = "F32.1 moderate depressive episode";
const NEW_DIAGNOSIS: &str = "Z00.0 general examination";
const RECOVERY_NOTE: &str = "call sister before discharge";
const CPF: &str = "123.456.789-00";
const NEW_CPF: &str = "987.654.321-00";

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "audited_patients", auditable, searchable)]
struct AuditedPatient {
    id: i32,
    name: String,
    #[orm(encrypted)]
    diagnosis: String,
    #[orm(encrypted)]
    recovery_note: Option<String>,
    #[orm(masked)]
    cpf: String,
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
impl AuditedPatientObserver for RecordingObserver {
    async fn committed(&self, event: &ModelCommittedEvent) -> Result<(), Error> {
        self.0.lock().unwrap().push(event.payload.clone());
        Ok(())
    }
}

fn assert_no_plaintext(source: &str, payload: &str) {
    for secret in [DIAGNOSIS, NEW_DIAGNOSIS, RECOVERY_NOTE, CPF, NEW_CPF] {
        assert!(
            !payload.contains(secret),
            "{source} leaked {secret}: {payload}"
        );
    }
}

async fn latest_update(pool: &rullst_orm::RullstPool, id: i32) -> (i32, String, String) {
    rullst_orm::_sqlx::query_as(
        "SELECT id, old_values, new_values FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind("audited_patients")
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read latest update audit")
}

#[tokio::test]
async fn protected_fields_never_reach_audit_events_or_search() {
    // This is the only test in this binary, so no other thread reads the
    // process-wide key variables while they are set.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef");
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "audit-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    Orm::init_with_options(
        "sqlite:file:protected_audit_test.db?mode=memory&cache=shared",
        2,
        30,
    )
    .await
    .expect("initialize protected audit database");
    let pool = Orm::pool().expect("pool");
    rullst_orm::_sqlx::query(
        "CREATE TABLE audited_patients (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, diagnosis TEXT NOT NULL, recovery_note TEXT, cpf TEXT NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create patients");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");

    let documents = Captured::default();
    let events = Captured::default();
    set_search_engine(RecordingEngine(documents.clone())).expect("configure Scout");
    AuditedPatient::observe(Arc::new(RecordingObserver(events.clone())));

    let context = AuditContext::system("protected-audit-test").expect("audit context");
    let (id, name_revision, secret_revision) = with_audit_context(context, async {
        let mut patient = AuditedPatient {
            id: 0,
            name: "Ana".to_string(),
            diagnosis: DIAGNOSIS.to_string(),
            recovery_note: Some(RECOVERY_NOTE.to_string()),
            cpf: CPF.to_string(),
        };
        patient.save().await.expect("create patient");

        patient.name = "Ana Maria".to_string();
        patient.save().await.expect("update name only");
        let name_revision = latest_update(pool, patient.id).await;

        patient.diagnosis = NEW_DIAGNOSIS.to_string();
        patient.save().await.expect("update diagnosis only");
        let diagnosis_revision = latest_update(pool, patient.id).await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&diagnosis_revision.2).unwrap(),
            serde_json::json!({"diagnosis": "***"}),
            "an encrypted-only change must still be audited, as a redacted value"
        );

        patient.cpf = NEW_CPF.to_string();
        patient.save().await.expect("update masked field only");
        let cpf_revision = latest_update(pool, patient.id).await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&cpf_revision.2).unwrap(),
            serde_json::json!({"cpf": "***"})
        );

        let refused = patient
            .restore_revision(diagnosis_revision.0, "restore withheld value")
            .await;
        assert!(matches!(refused, Err(Error::Validation(_))));

        let restored = patient
            .restore_revision(name_revision.0, "restore visible field")
            .await
            .expect("a revision without redacted changes stays restorable");
        assert_eq!(restored.name, "Ana");
        assert_eq!(restored.diagnosis, NEW_DIAGNOSIS);
        assert_eq!(restored.cpf, NEW_CPF);

        restored.delete().await.expect("delete patient");
        (restored.id, name_revision, diagnosis_revision)
    })
    .await;
    assert!(name_revision.1.contains("Ana"));
    assert!(secret_revision.1.contains("***"));

    let rows: Vec<AuditRow> =
        rullst_orm::_sqlx::query_as(
            "SELECT event, old_values, new_values, restore_patch FROM rullst_audits WHERE model_type = ? AND model_id = ?",
        )
        .bind("audited_patients")
        .bind(id)
        .fetch_all(pool)
        .await
        .expect("read audit rows");
    assert!(rows.iter().any(|row| row.0 == "created"));
    assert!(rows.iter().any(|row| row.0 == "deleted"));
    for (event, old_values, new_values, restore_patch) in &rows {
        for payload in [old_values, new_values, restore_patch]
            .into_iter()
            .flatten()
        {
            assert_no_plaintext(&format!("audit {event}"), payload);
        }
    }

    let events = events.lock().unwrap().clone();
    assert!(events.len() >= 5, "committed events should be recorded");
    for payload in &events {
        assert_no_plaintext("committed event", payload);
        assert!(payload.contains("\"diagnosis\":\"***\""));
    }

    let documents = documents.lock().unwrap().clone();
    assert!(!documents.is_empty(), "Scout documents should be recorded");
    for document in &documents {
        assert_no_plaintext("Scout document", document);
        for field in ["diagnosis", "recovery_note", "cpf"] {
            assert!(!document.contains(field), "{field} indexed: {document}");
        }
    }
}
