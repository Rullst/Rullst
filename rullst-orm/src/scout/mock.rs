use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde_json::Value;

use super::{SearchEngine, document_with_id, validate_index, validate_query};
use crate::Error;

type Documents = BTreeMap<String, BTreeMap<i32, Value>>;

/// Deterministic in-memory Scout backend for tests and offline development.
#[derive(Clone, Default)]
pub struct MockSearchEngine {
    documents: Arc<RwLock<Documents>>,
}

impl MockSearchEngine {
    /// Creates an empty deterministic search store.
    pub fn new() -> Self {
        Self::default()
    }

    fn read_documents(&self) -> Result<std::sync::RwLockReadGuard<'_, Documents>, Error> {
        self.documents
            .read()
            .map_err(|_| Error::Internal("Scout mock read lock is poisoned".to_string()))
    }

    fn write_documents(&self) -> Result<std::sync::RwLockWriteGuard<'_, Documents>, Error> {
        self.documents
            .write()
            .map_err(|_| Error::Internal("Scout mock write lock is poisoned".to_string()))
    }
}

#[async_trait]
impl SearchEngine for MockSearchEngine {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), Error> {
        validate_index(table)?;
        let payload = document_with_id(id, payload)?;
        self.write_documents()?
            .entry(table.to_string())
            .or_default()
            .insert(id, payload);
        Ok(())
    }

    async fn delete(&self, table: &str, id: i32) -> Result<(), Error> {
        validate_index(table)?;
        if id <= 0 {
            return Err(Error::Validation(
                "Scout document id must be positive".to_string(),
            ));
        }
        if let Some(index) = self.write_documents()?.get_mut(table) {
            index.remove(&id);
        }
        Ok(())
    }

    async fn search(&self, table: &str, query: &str) -> Result<Vec<i32>, Error> {
        validate_index(table)?;
        validate_query(query)?;
        let normalized = query.to_lowercase();
        let documents = self.read_documents()?;
        let Some(index) = documents.get(table) else {
            return Ok(Vec::new());
        };
        let mut ids = Vec::new();
        for (id, payload) in index {
            if normalized.is_empty() || payload.to_string().to_lowercase().contains(&normalized) {
                ids.push(*id);
            }
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_search_is_deterministic_and_bounded() {
        let engine = MockSearchEngine::new();
        engine
            .update("articles", 2, serde_json::json!({"title": "Rust"}))
            .await
            .expect("insert second document");
        engine
            .update("articles", 1, serde_json::json!({"title": "Rullst"}))
            .await
            .expect("insert first document");
        assert_eq!(
            engine.search("articles", "rust").await.expect("search"),
            vec![2]
        );
        assert_eq!(
            engine.search("articles", "").await.expect("list all"),
            vec![1, 2]
        );
        engine.delete("articles", 1).await.expect("delete document");
        assert_eq!(
            engine.search("articles", "").await.expect("list remaining"),
            vec![2]
        );
        assert!(
            engine
                .update("Bad Index", 3, serde_json::json!({}))
                .await
                .is_err()
        );
        assert!(
            engine
                .update("articles", 3, serde_json::json!({"id": 4}))
                .await
                .is_err()
        );
        assert!(engine.search("articles", "\n").await.is_err());
    }
}
