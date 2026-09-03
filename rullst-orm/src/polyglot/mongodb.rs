use std::marker::PhantomData;

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    Client, Database,
    bson::{Document, doc},
    error::{ErrorKind, WriteFailure},
};
use serde::{Serialize, de::DeserializeOwned};

use super::{
    Backend, BackendCapabilities, Capability, CollectionName, DocumentEntry, DocumentId,
    DocumentInventory, DocumentPage, DocumentRepository, MockDocumentStore, PolyglotError,
};

/// MongoDB document adapter with deterministic mock fallback.
#[non_exhaustive]
pub enum MongoDbStore<T> {
    /// A live driver connection.
    Live {
        database: Database,
        marker: PhantomData<fn() -> T>,
    },
    /// An explicit offline store selected by empty or `mock_*` credentials.
    Mock(MockDocumentStore<T>),
}

#[async_trait]
impl<T> DocumentInventory<T> for MongoDbStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    async fn list_entries(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<DocumentEntry<T>>, PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.list_entries(collection, page).await;
        };
        let mut cursor = database
            .collection::<Document>(collection.as_str())
            .find(doc! {})
            .sort(doc! { "_id": 1 })
            .skip(page.offset())
            .limit(i64::from(page.limit()))
            .await
            .map_err(map_mongodb_error)?;
        let mut documents = Vec::with_capacity(page.limit() as usize);
        while let Some(document) = cursor.try_next().await.map_err(map_mongodb_error)? {
            documents.push(decode_document_entry(document)?);
        }
        Ok(documents)
    }
}

impl<T> MongoDbStore<T> {
    /// Connects to MongoDB, or selects the deterministic mock for an empty or
    /// `mock_*` URI.
    pub async fn connect_or_mock(
        uri: impl Into<String>,
        database: impl Into<String>,
    ) -> Result<Self, PolyglotError> {
        let uri = uri.into();
        let database = CollectionName::new(database)?;
        if is_mock_credential(&uri) {
            return Ok(Self::Mock(MockDocumentStore::new()));
        }
        let client =
            Client::with_uri_str(&uri)
                .await
                .map_err(|_| PolyglotError::InvalidConfiguration {
                    backend: "MongoDB",
                    reason: "the connection URI could not be parsed",
                })?;
        Ok(Self::Live {
            database: client.database(database.as_str()),
            marker: PhantomData,
        })
    }

    /// Declares the adapter's bounded behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(Backend::MongoDb, &[Capability::Documents])
    }

    /// Returns whether this instance is using the deterministic offline store.
    pub const fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }
}

#[async_trait]
impl<T> DocumentRepository<T> for MongoDbStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.create(collection, id, entity).await;
        };
        let document = encode_document(entity, id)?;
        database
            .collection::<Document>(collection.as_str())
            .insert_one(document)
            .await
            .map(|_| ())
            .map_err(map_mongodb_write_error)
    }

    async fn find(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<Option<T>, PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.find(collection, id).await;
        };
        database
            .collection::<Document>(collection.as_str())
            .find_one(doc! { "_id": id.as_str() })
            .await
            .map_err(map_mongodb_error)?
            .map(decode_document)
            .transpose()
    }

    async fn replace(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.replace(collection, id, entity).await;
        };
        let document = encode_document(entity, id)?;
        let result = database
            .collection::<Document>(collection.as_str())
            .replace_one(doc! { "_id": id.as_str() }, document)
            .await
            .map_err(map_mongodb_write_error)?;
        if result.matched_count == 0 {
            return Err(PolyglotError::NotFound);
        }
        Ok(())
    }

    async fn delete(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<bool, PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.delete(collection, id).await;
        };
        database
            .collection::<Document>(collection.as_str())
            .delete_one(doc! { "_id": id.as_str() })
            .await
            .map(|result| result.deleted_count == 1)
            .map_err(map_mongodb_write_error)
    }

    async fn list(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<T>, PolyglotError> {
        let Self::Live { database, .. } = self else {
            return mock(self)?.list(collection, page).await;
        };
        let mut cursor = database
            .collection::<Document>(collection.as_str())
            .find(doc! {})
            .sort(doc! { "_id": 1 })
            .skip(page.offset())
            .limit(i64::from(page.limit()))
            .await
            .map_err(map_mongodb_error)?;
        let mut documents = Vec::with_capacity(page.limit() as usize);
        while let Some(document) = cursor.try_next().await.map_err(map_mongodb_error)? {
            documents.push(decode_document(document)?);
        }
        Ok(documents)
    }
}

fn mock<T>(store: &MongoDbStore<T>) -> Result<&MockDocumentStore<T>, PolyglotError> {
    match store {
        MongoDbStore::Mock(store) => Ok(store),
        MongoDbStore::Live { .. } => Err(PolyglotError::Driver {
            backend: "MongoDB",
            message: "adapter state changed during operation".to_owned(),
        }),
    }
}

fn encode_document<T: Serialize>(entity: &T, id: &DocumentId) -> Result<Document, PolyglotError> {
    let mut document = mongodb::bson::to_document(entity).map_err(PolyglotError::serialization)?;
    if document.contains_key("_id") {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "MongoDB model",
            reason: "the portable model must not define an _id field",
        });
    }
    document.insert("_id", id.as_str());
    Ok(document)
}

fn decode_document<T: DeserializeOwned>(mut document: Document) -> Result<T, PolyglotError> {
    document.remove("_id");
    mongodb::bson::from_document(document).map_err(PolyglotError::serialization)
}

fn decode_document_entry<T: DeserializeOwned>(
    mut document: Document,
) -> Result<DocumentEntry<T>, PolyglotError> {
    let id = document
        .remove("_id")
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or_else(|| PolyglotError::Driver {
            backend: "MongoDB",
            message: "document inventory returned a non-portable identifier".to_owned(),
        })?;
    let id = DocumentId::new(id)?;
    let entity = mongodb::bson::from_document(document).map_err(PolyglotError::serialization)?;
    Ok(DocumentEntry::new(id, entity))
}

fn is_mock_credential(value: &str) -> bool {
    value.is_empty() || value.starts_with("mock_") || value.starts_with("mock://")
}

fn map_mongodb_write_error(error: mongodb::error::Error) -> PolyglotError {
    if matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(write)) if write.code == 11000
    ) {
        PolyglotError::Conflict
    } else {
        map_mongodb_error(error)
    }
}

fn map_mongodb_error(error: mongodb::error::Error) -> PolyglotError {
    PolyglotError::driver("MongoDB", error)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Event {
        label: String,
    }

    #[tokio::test]
    async fn empty_credentials_select_the_offline_contract() {
        let store = MongoDbStore::<Event>::connect_or_mock("", "ignored")
            .await
            .unwrap();
        assert!(store.is_mock());
        let collection = CollectionName::new("events").unwrap();
        let id = DocumentId::new("evt-1").unwrap();
        let event = Event {
            label: "offline".to_owned(),
        };
        store.create(&collection, &id, &event).await.unwrap();
        assert_eq!(store.find(&collection, &id).await.unwrap(), Some(event));
        assert!(
            MongoDbStore::<Event>::connect_or_mock("", "invalid database")
                .await
                .is_err()
        );
    }

    #[test]
    fn rejects_models_that_own_the_driver_identifier() {
        #[derive(Serialize)]
        struct Invalid {
            #[serde(rename = "_id")]
            id: String,
        }

        let id = DocumentId::new("evt-1").unwrap();
        assert!(encode_document(&Invalid { id: "x".into() }, &id).is_err());
    }
}
