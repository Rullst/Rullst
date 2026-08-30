use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::RwLock;

use super::{
    Backend, BackendCapabilities, Capability, CollectionName, DocumentId, DocumentPage,
    DocumentRepository, PolyglotError,
};

type DocumentKey = (String, String);

/// Deterministic offline document adapter used directly or by remote drivers.
pub struct MockDocumentStore<T> {
    documents: Arc<RwLock<BTreeMap<DocumentKey, Vec<u8>>>>,
    marker: PhantomData<fn() -> T>,
}

impl<T> MockDocumentStore<T> {
    /// Creates an empty offline store.
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(BTreeMap::new())),
            marker: PhantomData,
        }
    }

    /// Declares the adapter's bounded behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(Backend::Mock, &[Capability::Documents])
    }
}

impl<T> Clone for MockDocumentStore<T> {
    fn clone(&self) -> Self {
        Self {
            documents: Arc::clone(&self.documents),
            marker: PhantomData,
        }
    }
}

impl<T> Default for MockDocumentStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> DocumentRepository<T> for MockDocumentStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let key = key(collection, id);
        let encoded = serde_json::to_vec(entity).map_err(PolyglotError::serialization)?;
        let mut documents = self.documents.write().await;
        if documents.contains_key(&key) {
            return Err(PolyglotError::Conflict);
        }
        documents.insert(key, encoded);
        Ok(())
    }

    async fn find(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<Option<T>, PolyglotError> {
        let documents = self.documents.read().await;
        documents
            .get(&key(collection, id))
            .map(|encoded| serde_json::from_slice(encoded).map_err(PolyglotError::serialization))
            .transpose()
    }

    async fn replace(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let key = key(collection, id);
        let encoded = serde_json::to_vec(entity).map_err(PolyglotError::serialization)?;
        let mut documents = self.documents.write().await;
        let Some(stored) = documents.get_mut(&key) else {
            return Err(PolyglotError::NotFound);
        };
        *stored = encoded;
        Ok(())
    }

    async fn delete(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<bool, PolyglotError> {
        Ok(self
            .documents
            .write()
            .await
            .remove(&key(collection, id))
            .is_some())
    }

    async fn list(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<T>, PolyglotError> {
        let documents = self.documents.read().await;
        let offset =
            usize::try_from(page.offset()).map_err(|_| PolyglotError::InvalidIdentifier {
                kind: "document page",
                reason: "offset exceeds the target index range",
            })?;
        documents
            .iter()
            .filter(|((stored_collection, _), _)| stored_collection == collection.as_str())
            .skip(offset)
            .take(page.limit() as usize)
            .map(|(_, encoded)| {
                serde_json::from_slice(encoded).map_err(PolyglotError::serialization)
            })
            .collect()
    }
}

fn key(collection: &CollectionName, id: &DocumentId) -> DocumentKey {
    (collection.as_str().to_owned(), id.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Event {
        sequence: u64,
        label: String,
    }

    fn event(sequence: u64) -> Event {
        Event {
            sequence,
            label: format!("event-{sequence}"),
        }
    }

    #[tokio::test]
    async fn provides_deterministic_document_crud() {
        let store = MockDocumentStore::<Event>::new();
        let collection = CollectionName::new("events").unwrap();
        let first = DocumentId::new("01").unwrap();
        let second = DocumentId::new("02").unwrap();

        store.create(&collection, &second, &event(2)).await.unwrap();
        store.create(&collection, &first, &event(1)).await.unwrap();
        assert!(matches!(
            store.create(&collection, &first, &event(3)).await,
            Err(PolyglotError::Conflict)
        ));

        assert_eq!(
            store.find(&collection, &first).await.unwrap(),
            Some(event(1))
        );
        store
            .replace(&collection, &first, &event(10))
            .await
            .unwrap();
        assert_eq!(
            store
                .list(&collection, DocumentPage::new(0, 1).unwrap())
                .await
                .unwrap(),
            vec![event(10)]
        );
        assert!(store.delete(&collection, &first).await.unwrap());
        assert!(!store.delete(&collection, &first).await.unwrap());
        assert!(matches!(
            store.replace(&collection, &first, &event(1)).await,
            Err(PolyglotError::NotFound)
        ));
    }
}
