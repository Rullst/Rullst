//! Transaction spans and outcomes for the global ORM facade.

use futures::future::BoxFuture;

use super::Orm;
use super::savepoint::{ManagedSavepoint, SharedTransaction};
use crate::post_commit::PostCommitScope;

impl Orm {
    /// Opens a transaction under a secret-free tracing span.
    #[tracing::instrument(
        name = "rullst.orm.transaction.begin",
        target = "rullst_orm",
        fields(orm.driver = tracing::field::Empty)
    )]
    pub async fn begin_transaction() -> Result<crate::db::Transaction<'static>, crate::Error> {
        Self::record_driver();
        let pool = Self::pool()?;
        pool.begin().await.map_err(Into::into)
    }

    /// Executes a closure inside an isolated transaction and records its final
    /// commit or rollback outcome without recording SQL, bindings, or errors.
    ///
    /// Called while another managed transaction is active on the same task,
    /// the closure joins that transaction through a savepoint and receives the
    /// same shared handle: an `Err` rolls back only the nested work, while a
    /// success releases the savepoint so the work commits (or rolls back) with
    /// the outer transaction. `after_commit` callbacks registered inside it
    /// are promoted to the outer commit boundary on success and discarded on
    /// failure. Do not hold the shared handle's lock across a nested call.
    ///
    /// The returned future is `Send` when `R` and `E` are, so the call can be
    /// nested in another transaction closure or run in a spawned task.
    #[tracing::instrument(
        name = "rullst.orm.transaction",
        target = "rullst_orm",
        skip(f),
        fields(
            orm.driver = tracing::field::Empty,
            orm.outcome = tracing::field::Empty
        )
    )]
    pub async fn transaction<F, R, E>(f: F) -> Result<R, crate::Error>
    where
        F: FnOnce(
                SharedTransaction,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, E>> + Send>>
            + Send,
        E: std::fmt::Display,
    {
        Self::record_driver();
        crate::__transaction_access::ensure_allowed()?;
        // Lock-holding steps run in non-generic boxed futures: a generic
        // future that awaits a borrowed lock is rejected by rustc as "Send is
        // not general enough", which would forbid nesting this call.
        let nested = match crate::CURRENT_TX.try_with(Clone::clone) {
            Ok(outer) => ManagedSavepoint::begin(outer.clone())
                .await?
                .map(|savepoint| (outer, savepoint)),
            Err(_) => None,
        };
        let (transaction, savepoint) = match nested {
            Some((outer, savepoint)) => (outer, Some(savepoint)),
            None => {
                let tx = Self::begin_transaction().await?;
                (std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx))), None)
            }
        };

        let post_commit = PostCommitScope::new();
        let result = match savepoint {
            // The nested closure already runs inside the outer task scope.
            Some(_) => post_commit.scope(f(transaction.clone())).await,
            None => {
                post_commit
                    .scope(crate::CURRENT_TX.scope(transaction.clone(), f(transaction.clone())))
                    .await
            }
        };
        let (value, failure) = match result {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let finished = match savepoint {
            Some(savepoint) => finish_savepoint(savepoint, post_commit, failure).await,
            None => finish_transaction(transaction, post_commit, failure).await,
        };
        finished?;
        value.ok_or_else(|| {
            crate::Error::Internal("managed transaction finished without a value".to_string())
        })
    }

    fn record_driver() {
        if let Ok(driver) = Self::try_driver() {
            tracing::Span::current().record("orm.driver", driver);
        }
    }
}

fn record_outcome(outcome: &'static str) {
    tracing::Span::current().record("orm.outcome", outcome);
}

/// Commits (after a successful closure) or rolls back the owned transaction.
fn finish_transaction(
    transaction: SharedTransaction,
    post_commit: PostCommitScope,
    failure: Option<String>,
) -> BoxFuture<'static, Result<(), crate::Error>> {
    Box::pin(async move {
        let owned = transaction.lock_owned().await.take();
        let Some(owned) = owned else {
            let (outcome, action) = if failure.is_some() {
                ("rollback_ownership_missing", "rollback")
            } else {
                ("commit_ownership_missing", "commit")
            };
            record_outcome(outcome);
            return Err(crate::Error::Internal(format!(
                "managed transaction ownership was removed before automatic {action}"
            )));
        };
        if let Some(error) = failure {
            if let Err(rollback_error) = owned.rollback().await {
                record_outcome("rollback_failed");
                return Err(crate::Error::DatabaseError(format!(
                    "Transaction failed: {error}; rollback also failed: {rollback_error}",
                )));
            }
            record_outcome("rolled_back");
            return Err(crate::Error::DatabaseError(format!(
                "Transaction failed: {error}",
            )));
        }
        if let Err(error) = owned.commit().await {
            record_outcome("commit_failed");
            return Err(error.into());
        }
        if let Err(error) = post_commit.commit().await {
            record_outcome("committed_post_commit_failed");
            return Err(error);
        }
        record_outcome("committed");
        Ok(())
    })
}

/// Releases the savepoint and promotes its callbacks to the enclosing commit
/// boundary, or rolls the savepoint back and discards them.
fn finish_savepoint(
    mut savepoint: ManagedSavepoint,
    post_commit: PostCommitScope,
    failure: Option<String>,
) -> BoxFuture<'static, Result<(), crate::Error>> {
    Box::pin(async move {
        if let Some(error) = failure {
            drop(post_commit);
            if let Err(rollback_error) = savepoint.rollback().await {
                record_outcome("savepoint_rollback_failed");
                return Err(crate::Error::DatabaseError(format!(
                    "Transaction failed: {error}; savepoint rollback also failed: {rollback_error}",
                )));
            }
            record_outcome("savepoint_rolled_back");
            return Err(crate::Error::DatabaseError(format!(
                "Transaction failed: {error}",
            )));
        }
        if let Err(error) = savepoint.release().await {
            record_outcome("savepoint_release_failed");
            return Err(error);
        }
        if let Err(error) = post_commit.promote_to_parent().await {
            record_outcome("savepoint_released_post_commit_failed");
            return Err(error);
        }
        record_outcome("savepoint_released");
        Ok(())
    })
}
