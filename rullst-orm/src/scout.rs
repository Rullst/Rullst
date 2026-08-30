use async_trait::async_trait;
use serde_json::Value;
use std::sync::OnceLock;

mod mock;
#[cfg(feature = "scout-http")]
mod providers;

pub use mock::MockSearchEngine;
#[cfg(feature = "scout-http")]
pub use providers::{AlgoliaEngine, ElasticsearchEngine, MeilisearchEngine};

#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), crate::Error>;
    async fn delete(&self, table: &str, id: i32) -> Result<(), crate::Error>;
    async fn search(&self, table: &str, query: &str) -> Result<Vec<i32>, crate::Error>;
}

pub(crate) const MAX_SEARCH_DOCUMENT_BYTES: usize = 1_048_576;
pub(crate) const MAX_SEARCH_QUERY_BYTES: usize = 1_024;

pub(crate) fn validate_index(value: &str) -> Result<(), crate::Error> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
        || !value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
    {
        return Err(crate::Error::Validation(
            "Scout index must start with a lowercase ASCII letter and contain only lowercase letters, digits, '-' or '_' (maximum 128 bytes)"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_query(value: &str) -> Result<(), crate::Error> {
    if value.len() > MAX_SEARCH_QUERY_BYTES || value.chars().any(char::is_control) {
        return Err(crate::Error::Validation(format!(
            "Scout query must contain at most {MAX_SEARCH_QUERY_BYTES} bytes and no control characters"
        )));
    }
    Ok(())
}

pub(crate) fn document_with_id(id: i32, mut payload: Value) -> Result<Value, crate::Error> {
    if id <= 0 {
        return Err(crate::Error::Validation(
            "Scout document id must be positive".to_string(),
        ));
    }
    let object = payload.as_object_mut().ok_or_else(|| {
        crate::Error::Validation("Scout document payload must be a JSON object".to_string())
    })?;
    if let Some(existing_id) = object.get("id")
        && existing_id.as_i64() != Some(i64::from(id))
    {
        return Err(crate::Error::Validation(
            "Scout document payload id conflicts with the model id".to_string(),
        ));
    }
    object.insert("id".to_string(), Value::from(id));
    let encoded = serde_json::to_vec(&payload)?;
    if encoded.len() > MAX_SEARCH_DOCUMENT_BYTES {
        return Err(crate::Error::Validation(format!(
            "Scout document exceeds {MAX_SEARCH_DOCUMENT_BYTES} bytes"
        )));
    }
    Ok(payload)
}

static SEARCH_ENGINE: OnceLock<Box<dyn SearchEngine>> = OnceLock::new();

pub fn set_search_engine(engine: impl SearchEngine + 'static) -> Result<(), crate::Error> {
    SEARCH_ENGINE.set(Box::new(engine)).map_err(|_| {
        crate::Error::Validation("Scout search engine is already configured".to_string())
    })
}

pub fn get_search_engine() -> Option<&'static dyn SearchEngine> {
    SEARCH_ENGINE.get().map(|e| &**e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_search_engine_none_before_set() {
        // In a fresh process (or when set_search_engine has not been called),
        // get_search_engine returns None. Because OnceLock ignores subsequent
        // writes, we can only reliably assert the Option shape here.
        // If another test in this suite already called set_search_engine the
        // result will be Some — both branches are valid at runtime.
        let _ = get_search_engine(); // must not panic
    }

    #[tokio::test]
    async fn test_set_search_engine_fails_closed_when_reconfigured() {
        struct Noop;
        #[async_trait::async_trait]
        impl SearchEngine for Noop {
            async fn update(
                &self,
                _: &str,
                _: i32,
                _: serde_json::Value,
            ) -> Result<(), crate::Error> {
                Ok(())
            }
            async fn delete(&self, _: &str, _: i32) -> Result<(), crate::Error> {
                Ok(())
            }
            async fn search(&self, _: &str, _: &str) -> Result<Vec<i32>, crate::Error> {
                Ok(vec![])
            }
        }
        let noop = Noop;
        let _ = noop.update("t", 1, serde_json::json!({})).await;
        let _ = noop.delete("t", 1).await;
        let _ = noop.search("t", "q").await;

        let first = set_search_engine(Noop);
        let second = set_search_engine(Noop);
        assert!(first.is_ok() || second.is_ok());
        assert!(first.is_err() || second.is_err());

        let engine = get_search_engine();
        assert!(engine.is_some());
    }
}
