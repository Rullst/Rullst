use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::*;
use crate::polyglot::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage, DocumentRepository,
    MockDocumentStore, PolyglotError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    sequence: u64,
    label: String,
}

fn event(sequence: u64) -> Event {
    Event {
        sequence,
        label: format!("secret-event-{sequence}"),
    }
}

fn binding(collection: &CollectionName) -> DocumentRecoveryBinding {
    DocumentRecoveryBinding::try_new("academy.production", collection.clone())
        .expect("valid binding")
}

fn recovery_key(byte: u8) -> DocumentRecoveryKey {
    DocumentRecoveryKey::try_new("backup-2026-09", [byte; 32]).expect("valid key")
}

#[tokio::test]
async fn encrypted_snapshot_restores_and_replays_exact_subset() {
    let source = MockDocumentStore::<Event>::new();
    let destination = MockDocumentStore::<Event>::new();
    let collection = CollectionName::new("events").expect("valid collection");
    let first = DocumentId::new("event-01").expect("valid ID");
    let second = DocumentId::new("event-02").expect("valid ID");
    source
        .create(&collection, &second, &event(2))
        .await
        .expect("seed second");
    source
        .create(&collection, &first, &event(1))
        .await
        .expect("seed first");
    destination
        .create(&collection, &first, &event(1))
        .await
        .expect("seed resumable prefix");

    let binding = binding(&collection);
    let key = recovery_key(7);
    let policy = DocumentRecoveryPolicy::default();
    let snapshot = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("export snapshot");
    let independently_sealed = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("repeat export");
    assert_ne!(snapshot, independently_sealed);
    assert!(!snapshot.as_str().contains("secret-event"));
    assert_eq!(
        format!("{snapshot:?}"),
        "EncryptedDocumentSnapshot([REDACTED])"
    );
    assert!(!format!("{key:?}").contains("[7, 7"));

    let first_report = restore_document_snapshot(&destination, &snapshot, &binding, &key, policy)
        .await
        .expect("resume restore");
    assert_eq!(first_report.inserted(), 1);
    assert_eq!(first_report.replayed(), 1);
    assert_eq!(first_report.verified(), 2);

    let replay = restore_document_snapshot(&destination, &snapshot, &binding, &key, policy)
        .await
        .expect("idempotent replay");
    assert_eq!(replay.inserted(), 0);
    assert_eq!(replay.replayed(), 2);
    assert_eq!(replay.verified(), 2);
}

#[tokio::test]
// TM-ORM-01
async fn tampering_wrong_key_and_cross_scope_copy_fail_closed() {
    let source = MockDocumentStore::<Event>::new();
    let collection = CollectionName::new("events").expect("valid collection");
    source
        .create(
            &collection,
            &DocumentId::new("event-01").expect("valid ID"),
            &event(1),
        )
        .await
        .expect("seed source");
    let binding = binding(&collection);
    let key = recovery_key(9);
    let policy = DocumentRecoveryPolicy::default();
    let snapshot = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("export snapshot");

    let mut tampered = snapshot.as_str().as_bytes().to_vec();
    let index = tampered.len().saturating_sub(8);
    tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(tampered).expect("ASCII envelope");
    let tampered = EncryptedDocumentSnapshot::try_from_envelope(tampered, policy)
        .expect("structurally valid envelope");
    let destination = MockDocumentStore::<Event>::new();
    assert!(matches!(
        restore_document_snapshot(&destination, &tampered, &binding, &key, policy).await,
        Err(DocumentRecoveryError::AuthenticationFailed)
    ));

    let wrong_key = recovery_key(10);
    assert!(matches!(
        restore_document_snapshot(&destination, &snapshot, &binding, &wrong_key, policy).await,
        Err(DocumentRecoveryError::AuthenticationFailed)
    ));
    let other_binding = DocumentRecoveryBinding::try_new("other.production", collection)
        .expect("valid other binding");
    assert!(matches!(
        restore_document_snapshot(&destination, &snapshot, &other_binding, &key, policy).await,
        Err(DocumentRecoveryError::AuthenticationFailed)
    ));
}

#[tokio::test]
async fn malformed_envelopes_and_wrong_rotation_id_fail_before_repository_access() {
    let policy = DocumentRecoveryPolicy::default();
    assert!(matches!(
        EncryptedDocumentSnapshot::try_from_envelope("not-an-envelope", policy),
        Err(DocumentRecoveryError::InvalidEnvelope)
    ));

    let source = MockDocumentStore::<Event>::new();
    let destination = MockDocumentStore::<Event>::new();
    let collection = CollectionName::new("events").expect("valid collection");
    let binding = binding(&collection);
    let key = recovery_key(14);
    let snapshot = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("empty snapshot");
    let future = snapshot.as_str().replacen(":v1:", ":v2:", 1);
    assert!(matches!(
        EncryptedDocumentSnapshot::try_from_envelope(future, policy),
        Err(DocumentRecoveryError::UnsupportedVersion)
    ));
    let malformed_ciphertext = format!("{}=", snapshot.as_str());
    assert!(matches!(
        EncryptedDocumentSnapshot::try_from_envelope(malformed_ciphertext, policy),
        Err(DocumentRecoveryError::InvalidEnvelope)
    ));

    let rotated =
        DocumentRecoveryKey::try_new("backup-2026-10", [14; 32]).expect("valid rotated key");
    assert!(matches!(
        restore_document_snapshot(&destination, &snapshot, &binding, &rotated, policy).await,
        Err(DocumentRecoveryError::KeyIdMismatch)
    ));
}

#[tokio::test]
// TM-ORM-02
async fn conflicting_or_extra_destination_content_is_never_overwritten() {
    let source = MockDocumentStore::<Event>::new();
    let collection = CollectionName::new("events").expect("valid collection");
    let id = DocumentId::new("event-01").expect("valid ID");
    source
        .create(&collection, &id, &event(1))
        .await
        .expect("seed source");
    let binding = binding(&collection);
    let key = recovery_key(11);
    let policy = DocumentRecoveryPolicy::default();
    let snapshot = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("export snapshot");

    let conflicting = MockDocumentStore::<Event>::new();
    conflicting
        .create(&collection, &id, &event(99))
        .await
        .expect("seed conflict");
    assert!(matches!(
        restore_document_snapshot(&conflicting, &snapshot, &binding, &key, policy).await,
        Err(DocumentRecoveryError::DestinationConflict)
    ));
    assert_eq!(
        conflicting
            .find(&collection, &id)
            .await
            .expect("find conflict"),
        Some(event(99))
    );

    let extra = MockDocumentStore::<Event>::new();
    extra
        .create(
            &collection,
            &DocumentId::new("extra").expect("valid extra ID"),
            &event(3),
        )
        .await
        .expect("seed extra");
    assert!(matches!(
        restore_document_snapshot(&extra, &snapshot, &binding, &key, policy).await,
        Err(DocumentRecoveryError::DestinationConflict)
    ));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReservedIdModel {
    id: String,
}

#[tokio::test]
async fn policy_bounds_and_non_portable_models_fail_before_sealing() {
    assert!(DocumentRecoveryPolicy::try_new(0, 1, 1_024).is_err());
    assert!(DocumentRecoveryPolicy::try_new(1, 0, 1_024).is_err());
    assert!(DocumentRecoveryPolicy::try_new(1, 1, 1_023).is_err());

    let collection = CollectionName::new("events").expect("valid collection");
    let binding = binding(&collection);
    let key = recovery_key(12);
    let source = MockDocumentStore::<ReservedIdModel>::new();
    source
        .create(
            &collection,
            &DocumentId::new("event-01").expect("valid ID"),
            &ReservedIdModel {
                id: "embedded".to_owned(),
            },
        )
        .await
        .expect("mock accepts backend-specific shape");
    assert!(matches!(
        export_document_snapshot(&source, &binding, &key, DocumentRecoveryPolicy::default()).await,
        Err(DocumentRecoveryError::NonPortableModel)
    ));

    let bounded = MockDocumentStore::<Event>::new();
    bounded
        .create(
            &collection,
            &DocumentId::new("one").expect("valid ID"),
            &event(1),
        )
        .await
        .expect("seed first");
    bounded
        .create(
            &collection,
            &DocumentId::new("two").expect("valid ID"),
            &event(2),
        )
        .await
        .expect("seed second");
    let one_document = DocumentRecoveryPolicy::try_new(1, 1, 1_024).expect("valid strict policy");
    assert!(matches!(
        export_document_snapshot(&bounded, &binding, &key, one_document).await,
        Err(DocumentRecoveryError::CapacityExceeded)
    ));
}

struct ChangingInventory {
    observations: AtomicUsize,
}

struct UnorderedInventory;

#[async_trait]
impl DocumentRepository<Event> for UnorderedInventory {
    async fn create(
        &self,
        _: &CollectionName,
        _: &DocumentId,
        _: &Event,
    ) -> Result<(), PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn find(
        &self,
        _: &CollectionName,
        _: &DocumentId,
    ) -> Result<Option<Event>, PolyglotError> {
        unreachable!("export does not find individual records")
    }

    async fn replace(
        &self,
        _: &CollectionName,
        _: &DocumentId,
        _: &Event,
    ) -> Result<(), PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn delete(&self, _: &CollectionName, _: &DocumentId) -> Result<bool, PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn list(&self, _: &CollectionName, _: DocumentPage) -> Result<Vec<Event>, PolyglotError> {
        unreachable!("export uses identifier-preserving inventory")
    }
}

#[async_trait]
impl DocumentInventory<Event> for UnorderedInventory {
    async fn list_entries(
        &self,
        _: &CollectionName,
        _: DocumentPage,
    ) -> Result<Vec<DocumentEntry<Event>>, PolyglotError> {
        Ok(vec![
            DocumentEntry::new(DocumentId::new("event-02").expect("valid ID"), event(2)),
            DocumentEntry::new(DocumentId::new("event-01").expect("valid ID"), event(1)),
        ])
    }
}

#[async_trait]
impl DocumentRepository<Event> for ChangingInventory {
    async fn create(
        &self,
        _: &CollectionName,
        _: &DocumentId,
        _: &Event,
    ) -> Result<(), PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn find(
        &self,
        _: &CollectionName,
        _: &DocumentId,
    ) -> Result<Option<Event>, PolyglotError> {
        unreachable!("export does not find individual records")
    }

    async fn replace(
        &self,
        _: &CollectionName,
        _: &DocumentId,
        _: &Event,
    ) -> Result<(), PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn delete(&self, _: &CollectionName, _: &DocumentId) -> Result<bool, PolyglotError> {
        unreachable!("export does not mutate")
    }

    async fn list(&self, _: &CollectionName, _: DocumentPage) -> Result<Vec<Event>, PolyglotError> {
        unreachable!("export uses identifier-preserving inventory")
    }
}

#[async_trait]
impl DocumentInventory<Event> for ChangingInventory {
    async fn list_entries(
        &self,
        _: &CollectionName,
        _: DocumentPage,
    ) -> Result<Vec<DocumentEntry<Event>>, PolyglotError> {
        let observation = self.observations.fetch_add(1, Ordering::SeqCst);
        Ok(vec![DocumentEntry::new(
            DocumentId::new("event-01").expect("valid ID"),
            event(observation as u64),
        )])
    }
}

#[tokio::test]
async fn changing_source_is_rejected_before_snapshot_creation() {
    let source = ChangingInventory {
        observations: AtomicUsize::new(0),
    };
    let collection = CollectionName::new("events").expect("valid collection");
    assert!(matches!(
        export_document_snapshot(
            &source,
            &binding(&collection),
            &recovery_key(13),
            DocumentRecoveryPolicy::default()
        )
        .await,
        Err(DocumentRecoveryError::SourceChanged)
    ));
}

#[tokio::test]
async fn unordered_inventory_is_rejected_before_snapshot_creation() {
    let collection = CollectionName::new("events").expect("valid collection");
    assert!(matches!(
        export_document_snapshot(
            &UnorderedInventory,
            &binding(&collection),
            &recovery_key(15),
            DocumentRecoveryPolicy::default()
        )
        .await,
        Err(DocumentRecoveryError::InvalidInventory)
    ));
}
