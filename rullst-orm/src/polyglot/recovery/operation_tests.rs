#![allow(clippy::expect_used, clippy::unwrap_used)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    PAYLOAD_VERSION, StoredEntry, StoredSnapshot, collect_entries, export_document_snapshot,
    restore_document_snapshot, validate_snapshot,
};
use crate::polyglot::recovery::{
    DocumentRecoveryBinding, DocumentRecoveryError, DocumentRecoveryKey, DocumentRecoveryPolicy,
};
use crate::polyglot::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage, DocumentRepository,
    MockDocumentStore, PolyglotError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Note {
    sequence: u64,
    label: String,
}

fn note(label: &str) -> Note {
    Note {
        sequence: 1,
        label: label.to_owned(),
    }
}

fn binding() -> DocumentRecoveryBinding {
    DocumentRecoveryBinding::try_new("academy", CollectionName::new("notes").unwrap()).unwrap()
}

fn snapshot(version: u8, namespace: &str, collection: &str, entries: usize) -> StoredSnapshot {
    StoredSnapshot {
        version,
        application_namespace: namespace.to_owned(),
        collection: collection.to_owned(),
        entries: (0..entries)
            .map(|index| StoredEntry {
                id: format!("note-{index}"),
                entity: json!({ "sequence": index, "label": "x" }),
            })
            .collect(),
    }
}

#[test]
fn stored_snapshots_must_match_their_scope_version_and_document_bound() {
    let policy = DocumentRecoveryPolicy::try_new(10, 2, 1_024).unwrap();
    let binding = binding();
    validate_snapshot(
        &snapshot(PAYLOAD_VERSION, "academy", "notes", 2),
        &binding,
        policy,
    )
    .expect("a matching snapshot at the document bound");
    for invalid in [
        snapshot(PAYLOAD_VERSION + 1, "academy", "notes", 1),
        snapshot(PAYLOAD_VERSION, "other", "notes", 1),
        snapshot(PAYLOAD_VERSION, "academy", "other", 1),
        snapshot(PAYLOAD_VERSION, "academy", "notes", 3),
    ] {
        assert!(matches!(
            validate_snapshot(&invalid, &binding, policy),
            Err(DocumentRecoveryError::InvalidPayload)
        ));
    }
}

#[tokio::test]
async fn inventories_reach_exactly_the_estimated_snapshot_bound() {
    let policy = DocumentRecoveryPolicy::try_new(10, 10, 1_024).unwrap();
    let collection = CollectionName::new("notes").unwrap();
    // Estimate: id "note-1" (6) + `{"sequence":1,"label":"…"}` (25 + label) + 32.
    for (label_len, fits) in [(961, true), (962, false)] {
        let store = MockDocumentStore::<Note>::new();
        store
            .create(
                &collection,
                &DocumentId::new("note-1").unwrap(),
                &note(&"x".repeat(label_len)),
            )
            .await
            .unwrap();
        let collected = collect_entries(&store, &binding(), policy).await;
        assert_eq!(collected.is_ok(), fits, "{label_len}");
    }
}

/// A destination where another writer creates each record first.
struct Racing {
    inner: MockDocumentStore<Note>,
    concurrent: Note,
}

#[async_trait]
impl DocumentRepository<Note> for Racing {
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        _: &Note,
    ) -> Result<(), PolyglotError> {
        self.inner.create(collection, id, &self.concurrent).await?;
        Err(PolyglotError::Conflict)
    }

    async fn find(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<Option<Note>, PolyglotError> {
        self.inner.find(collection, id).await
    }

    async fn replace(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        note: &Note,
    ) -> Result<(), PolyglotError> {
        self.inner.replace(collection, id, note).await
    }

    async fn delete(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<bool, PolyglotError> {
        self.inner.delete(collection, id).await
    }

    async fn list(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<Note>, PolyglotError> {
        self.inner.list(collection, page).await
    }
}

#[async_trait]
impl DocumentInventory<Note> for Racing {
    async fn list_entries(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<DocumentEntry<Note>>, PolyglotError> {
        self.inner.list_entries(collection, page).await
    }
}

#[tokio::test]
async fn a_concurrent_identical_create_counts_as_a_replay() {
    let policy = DocumentRecoveryPolicy::default();
    let key = DocumentRecoveryKey::try_new("backup-1", [3; 32]).unwrap();
    let source = MockDocumentStore::<Note>::new();
    let collection = CollectionName::new("notes").unwrap();
    source
        .create(
            &collection,
            &DocumentId::new("note-1").unwrap(),
            &note("same"),
        )
        .await
        .unwrap();
    let snapshot = export_document_snapshot(&source, &binding(), &key, policy)
        .await
        .unwrap();

    let identical = Racing {
        inner: MockDocumentStore::new(),
        concurrent: note("same"),
    };
    let report = restore_document_snapshot(&identical, &snapshot, &binding(), &key, policy)
        .await
        .expect("an identical concurrent write is a replay");
    assert_eq!(
        (report.inserted(), report.replayed(), report.verified()),
        (0, 1, 1)
    );

    let different = Racing {
        inner: MockDocumentStore::new(),
        concurrent: note("other"),
    };
    assert!(matches!(
        restore_document_snapshot(&different, &snapshot, &binding(), &key, policy).await,
        Err(DocumentRecoveryError::DestinationConflict)
    ));
}
