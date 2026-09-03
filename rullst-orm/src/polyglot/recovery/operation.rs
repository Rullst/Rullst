use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    DocumentRecoveryBinding, DocumentRecoveryError, DocumentRecoveryKey, DocumentRecoveryPolicy,
    DocumentRecoveryReport, EncryptedDocumentSnapshot, codec,
};
use crate::polyglot::{DocumentId, DocumentInventory, DocumentPage, PolyglotError};

const PAYLOAD_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct StoredSnapshot {
    version: u8,
    application_namespace: String,
    collection: String,
    entries: Vec<StoredEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct StoredEntry {
    id: String,
    entity: Value,
}

/// Exports two matching observations of one quiesced document collection.
///
/// The source must remain unchanged for the duration of this call. Two exact
/// scans detect ordinary concurrent mutation, but only caller-owned write
/// exclusion can provide a formal snapshot boundary.
pub async fn export_document_snapshot<R, T>(
    source: &R,
    binding: &DocumentRecoveryBinding,
    key: &DocumentRecoveryKey,
    policy: DocumentRecoveryPolicy,
) -> Result<EncryptedDocumentSnapshot, DocumentRecoveryError>
where
    R: DocumentInventory<T> + ?Sized,
    T: Serialize + DeserializeOwned + Send + Sync,
{
    let first = collect_entries(source, binding, policy).await?;
    let second = collect_entries(source, binding, policy).await?;
    if first != second {
        return Err(DocumentRecoveryError::SourceChanged);
    }
    let snapshot = StoredSnapshot {
        version: PAYLOAD_VERSION,
        application_namespace: binding.application_namespace().to_owned(),
        collection: binding.collection().as_str().to_owned(),
        entries: first,
    };
    codec::seal(&snapshot, key, binding, policy)
}

/// Restores an exact empty-or-matching-subset destination and verifies it.
///
/// Successful inserts are not rolled back after a later failure. Repeating the
/// operation is safe when retained rows still exactly match the snapshot. The
/// caller must quiesce unrelated destination writers; this function never
/// deletes extra records and does not claim a cross-store transaction.
pub async fn restore_document_snapshot<R, T>(
    destination: &R,
    snapshot: &EncryptedDocumentSnapshot,
    binding: &DocumentRecoveryBinding,
    key: &DocumentRecoveryKey,
    policy: DocumentRecoveryPolicy,
) -> Result<DocumentRecoveryReport, DocumentRecoveryError>
where
    R: DocumentInventory<T> + ?Sized,
    T: Serialize + DeserializeOwned + Send + Sync,
{
    let snapshot = codec::open(snapshot, key, binding, policy)?;
    validate_snapshot(&snapshot, binding, policy)?;
    let expected = entry_map(&snapshot.entries);
    let before = entry_map(&collect_entries(destination, binding, policy).await?);
    ensure_subset(&before, &expected)?;

    let mut inserted = 0_u32;
    let mut replayed =
        u32::try_from(before.len()).map_err(|_| DocumentRecoveryError::CapacityExceeded)?;
    for entry in &snapshot.entries {
        if before.contains_key(&entry.id) {
            continue;
        }
        let id =
            DocumentId::new(entry.id.clone()).map_err(|_| DocumentRecoveryError::InvalidPayload)?;
        let entity: T = serde_json::from_value(entry.entity.clone())
            .map_err(|_| DocumentRecoveryError::InvalidPayload)?;
        match destination.create(binding.collection(), &id, &entity).await {
            Ok(()) => inserted = inserted.saturating_add(1),
            Err(PolyglotError::Conflict) => {
                let Some(current) = destination
                    .find(binding.collection(), &id)
                    .await
                    .map_err(DocumentRecoveryError::from)?
                else {
                    return Err(DocumentRecoveryError::DestinationChanged);
                };
                let current = portable_value(&current)?;
                if current != entry.entity {
                    return Err(DocumentRecoveryError::DestinationConflict);
                }
                replayed = replayed.saturating_add(1);
            }
            Err(error) => return Err(DocumentRecoveryError::from(error)),
        }
    }

    let after = entry_map(&collect_entries(destination, binding, policy).await?);
    if after != expected {
        return Err(DocumentRecoveryError::DestinationChanged);
    }
    let verified =
        u32::try_from(after.len()).map_err(|_| DocumentRecoveryError::CapacityExceeded)?;
    Ok(DocumentRecoveryReport {
        inserted,
        replayed,
        verified,
    })
}

async fn collect_entries<R, T>(
    repository: &R,
    binding: &DocumentRecoveryBinding,
    policy: DocumentRecoveryPolicy,
) -> Result<Vec<StoredEntry>, DocumentRecoveryError>
where
    R: DocumentInventory<T> + ?Sized,
    T: Serialize + DeserializeOwned + Send + Sync,
{
    let mut entries = Vec::new();
    let mut previous_id: Option<DocumentId> = None;
    let mut estimated_bytes = 0_usize;
    loop {
        let count =
            u32::try_from(entries.len()).map_err(|_| DocumentRecoveryError::CapacityExceeded)?;
        let remaining = policy.max_documents().saturating_sub(count);
        let limit = if remaining == 0 {
            1
        } else {
            policy.page_size().min(remaining)
        };
        let offset =
            u64::try_from(entries.len()).map_err(|_| DocumentRecoveryError::CapacityExceeded)?;
        let page =
            DocumentPage::new(offset, limit).map_err(|_| DocumentRecoveryError::InvalidPolicy)?;
        let batch = repository
            .list_entries(binding.collection(), page)
            .await
            .map_err(DocumentRecoveryError::from)?;
        if batch.len() > limit as usize {
            return Err(DocumentRecoveryError::InvalidInventory);
        }
        if remaining == 0 {
            return if batch.is_empty() {
                Ok(entries)
            } else {
                Err(DocumentRecoveryError::CapacityExceeded)
            };
        }
        let batch_len = batch.len();
        for entry in batch {
            validate_order(&mut previous_id, entry.id())?;
            let (id, entity) = entry.into_parts();
            let entity = portable_value(&entity)?;
            let encoded_len = serde_json::to_vec(&entity)
                .map_err(|_| DocumentRecoveryError::InvalidPayload)?
                .len();
            estimated_bytes = estimated_bytes
                .checked_add(id.as_str().len())
                .and_then(|size| size.checked_add(encoded_len))
                .and_then(|size| size.checked_add(32))
                .ok_or(DocumentRecoveryError::CapacityExceeded)?;
            if estimated_bytes > policy.max_snapshot_bytes() {
                return Err(DocumentRecoveryError::CapacityExceeded);
            }
            entries.push(StoredEntry {
                id: id.as_str().to_owned(),
                entity,
            });
        }
        if batch_len < limit as usize {
            return Ok(entries);
        }
    }
}

fn validate_snapshot(
    snapshot: &StoredSnapshot,
    binding: &DocumentRecoveryBinding,
    policy: DocumentRecoveryPolicy,
) -> Result<(), DocumentRecoveryError> {
    if snapshot.version != PAYLOAD_VERSION
        || snapshot.application_namespace != binding.application_namespace()
        || snapshot.collection != binding.collection().as_str()
        || snapshot.entries.len() > policy.max_documents() as usize
    {
        return Err(DocumentRecoveryError::InvalidPayload);
    }
    let mut previous_id = None;
    for entry in &snapshot.entries {
        let id =
            DocumentId::new(entry.id.clone()).map_err(|_| DocumentRecoveryError::InvalidPayload)?;
        validate_order(&mut previous_id, &id).map_err(|_| DocumentRecoveryError::InvalidPayload)?;
        validate_portable_value(&entry.entity)
            .map_err(|_| DocumentRecoveryError::InvalidPayload)?;
    }
    Ok(())
}

fn validate_order(
    previous: &mut Option<DocumentId>,
    current: &DocumentId,
) -> Result<(), DocumentRecoveryError> {
    if previous.as_ref().is_some_and(|id| id >= current) {
        return Err(DocumentRecoveryError::InvalidInventory);
    }
    *previous = Some(current.clone());
    Ok(())
}

fn portable_value<T: Serialize>(entity: &T) -> Result<Value, DocumentRecoveryError> {
    let value =
        serde_json::to_value(entity).map_err(|_| DocumentRecoveryError::NonPortableModel)?;
    validate_portable_value(&value)?;
    Ok(value)
}

fn validate_portable_value(value: &Value) -> Result<(), DocumentRecoveryError> {
    let Some(object) = value.as_object() else {
        return Err(DocumentRecoveryError::NonPortableModel);
    };
    if object.contains_key("id") || object.contains_key("_id") {
        return Err(DocumentRecoveryError::NonPortableModel);
    }
    Ok(())
}

fn entry_map(entries: &[StoredEntry]) -> BTreeMap<String, Value> {
    entries
        .iter()
        .map(|entry| (entry.id.clone(), entry.entity.clone()))
        .collect()
}

fn ensure_subset(
    observed: &BTreeMap<String, Value>,
    expected: &BTreeMap<String, Value>,
) -> Result<(), DocumentRecoveryError> {
    if observed
        .iter()
        .any(|(id, value)| expected.get(id) != Some(value))
    {
        return Err(DocumentRecoveryError::DestinationConflict);
    }
    Ok(())
}
