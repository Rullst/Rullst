//! Transaction-aware post-commit callbacks.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::Error;

type CallbackFuture = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'static>>;
type Callback = Box<dyn FnOnce() -> CallbackFuture + Send + 'static>;
type CallbackQueue = Arc<Mutex<Vec<Callback>>>;

tokio::task_local! {
    static CALLBACKS: CallbackQueue;
}

/// Database mutation represented by a committed model event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelOperation {
    /// A new model row was created.
    Created,
    /// An existing model row was updated.
    Updated,
    /// A model row was deleted or soft-deleted.
    Deleted,
}

impl ModelOperation {
    /// Stable lowercase name used by event transports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
        }
    }
}

/// Owned model snapshot delivered only after a managed transaction commits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCommittedEvent {
    /// Persisted table name generated from the model metadata.
    pub table: &'static str,
    /// Model primary key.
    pub id: i32,
    /// Mutation that was committed.
    pub operation: ModelOperation,
    /// Model JSON generated with the model's hidden-field policy.
    pub payload: String,
}

impl ModelCommittedEvent {
    /// Creates an owned committed-event snapshot.
    pub fn new(
        table: &'static str,
        id: i32,
        operation: ModelOperation,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            table,
            id,
            operation,
            payload: payload.into(),
        }
    }
}

/// Owns callbacks collected by one managed database transaction.
///
/// This type is public only so generated ORM code can share the same commit
/// boundary. Applications should normally use [`crate::Orm::transaction`] and
/// [`after_commit`]. Dropping a scope discards every pending callback.
#[doc(hidden)]
pub struct PostCommitScope {
    callbacks: CallbackQueue,
}

impl PostCommitScope {
    /// Creates an empty callback scope.
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Runs a future with this scope as the current commit boundary.
    #[doc(hidden)]
    pub async fn run<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        CALLBACKS.scope(self.callbacks.clone(), future).await
    }

    /// Runs all callbacks after the caller has confirmed its database commit.
    #[doc(hidden)]
    pub async fn commit(self) -> Result<(), Error> {
        let callbacks = {
            let mut callbacks = self.callbacks.lock().await;
            std::mem::take(&mut *callbacks)
        };
        run_callbacks(callbacks).await
    }
}

impl Default for PostCommitScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs a callback after the current [`crate::Orm::transaction`] commits.
///
/// If no managed transaction is active, the callback runs immediately. This
/// matches an already committed/autocommit operation. A callback failure is
/// returned as [`Error::PostCommit`], making it explicit that the database
/// write may already be durable.
pub async fn after_commit<F, Fut>(callback: F) -> Result<(), Error>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), Error>> + Send + 'static,
{
    if let Ok(queue) = CALLBACKS.try_with(Clone::clone) {
        queue
            .lock()
            .await
            .push(Box::new(move || Box::pin(callback())));
        return Ok(());
    }

    callback()
        .await
        .map_err(|error| Error::PostCommit(error.to_string()))
}

async fn run_callbacks(callbacks: Vec<Callback>) -> Result<(), Error> {
    let mut failures = Vec::new();
    for callback in callbacks {
        if let Err(error) = callback().await {
            failures.push(error.to_string());
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::PostCommit(failures.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{PostCommitScope, after_commit};
    use crate::Error;

    #[tokio::test]
    async fn callback_without_scope_runs_immediately() {
        let calls = Arc::new(AtomicUsize::new(0));
        let callback_calls = calls.clone();
        after_commit(move || async move {
            callback_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .expect("run immediate callback");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn scope_defers_callbacks_and_drop_discards_them() {
        let calls = Arc::new(AtomicUsize::new(0));
        let scope = PostCommitScope::new();
        let callback_calls = calls.clone();
        scope
            .run(after_commit(move || async move {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .await
            .expect("register callback");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        drop(scope);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn commit_runs_every_callback_and_reports_failures() {
        let calls = Arc::new(AtomicUsize::new(0));
        let scope = PostCommitScope::new();
        scope
            .run(async {
                after_commit(|| async { Err(Error::CacheError("cache unavailable".to_string())) })
                    .await?;
                let callback_calls = calls.clone();
                after_commit(move || async move {
                    callback_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
            })
            .await
            .expect("register callbacks");

        let error = scope
            .commit()
            .await
            .expect_err("failed callback must be reported");
        assert!(matches!(error, Error::PostCommit(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
