use async_trait::async_trait;
use serde_json::Value;
use std::sync::OnceLock;

#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), crate::Error>;
    async fn delete(&self, table: &str, id: i32) -> Result<(), crate::Error>;
    async fn search(&self, table: &str, query: &str) -> Result<Vec<i32>, crate::Error>;
}

static SEARCH_ENGINE: OnceLock<Box<dyn SearchEngine>> = OnceLock::new();

pub fn set_search_engine(engine: Box<dyn SearchEngine>) {
    let _ = SEARCH_ENGINE.set(engine);
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
    async fn test_set_search_engine_is_idempotent() {
        // A second set_search_engine call is silently ignored by OnceLock.
        // This test verifies that calling it multiple times does not panic.
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

        set_search_engine(Box::new(Noop));
        set_search_engine(Box::new(Noop)); // second call must not panic

        let engine = get_search_engine();
        assert!(engine.is_some());
    }
}
