//! Savepoints that let a nested [`Orm::transaction`](super::Orm::transaction)
//! join the task-scoped transaction.
//!
//! The savepoint is opened, released and rolled back through SQLx's own
//! transaction manager, so SQLx keeps tracking the nesting depth that generated
//! model savepoints and the outer commit rely on. Dropping an unfinished
//! savepoint (for example when its future is cancelled) queues its rollback,
//! exactly like dropping a SQLx `Transaction`.

use futures::future::BoxFuture;
use sqlx_core::transaction::TransactionManager;

use crate::database::RullstDatabase;

type Manager = <RullstDatabase as sqlx::Database>::TransactionManager;
pub(super) type SharedTransaction =
    std::sync::Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>;

/// One open savepoint on a shared managed transaction.
pub(super) struct ManagedSavepoint {
    transaction: SharedTransaction,
    depth: usize,
    open: bool,
}

impl ManagedSavepoint {
    /// Opens a savepoint on `transaction`, or returns `None` when no managed
    /// transaction is available in the shared handle.
    pub(super) fn begin(
        transaction: SharedTransaction,
    ) -> BoxFuture<'static, Result<Option<Self>, crate::Error>> {
        Box::pin(async move {
            let depth = {
                let guard = transaction.clone().lock_owned().await;
                let Some(tx) = guard.as_ref() else {
                    return Ok(None);
                };
                <Manager as TransactionManager>::get_transaction_depth(&**tx) + 1
            };
            // Declared before the lock guard, so on cancellation the guard is
            // released first and `Drop` can roll back a savepoint that the
            // interrupted statement did open.
            let mut savepoint = Self {
                transaction: transaction.clone(),
                depth,
                open: true,
            };
            let mut guard = transaction.lock_owned().await;
            let Some(tx) = guard.as_mut() else {
                savepoint.open = false;
                return Ok(None);
            };
            savepoint.depth = <Manager as TransactionManager>::get_transaction_depth(&**tx) + 1;
            <Manager as TransactionManager>::begin(&mut **tx, None).await?;
            drop(guard);
            Ok(Some(savepoint))
        })
    }

    /// Releases the savepoint; its changes now commit with the outer transaction.
    pub(super) async fn release(&mut self) -> Result<(), crate::Error> {
        self.finish(false).await
    }

    /// Rolls the transaction back to the state before the savepoint.
    pub(super) async fn rollback(&mut self) -> Result<(), crate::Error> {
        self.finish(true).await
    }

    async fn finish(&mut self, rollback: bool) -> Result<(), crate::Error> {
        let mut guard = self.transaction.clone().lock_owned().await;
        let Some(tx) = guard.as_mut() else {
            self.open = false;
            return Err(crate::Error::Internal(
                "managed transaction ownership was removed before its nested savepoint finished"
                    .to_string(),
            ));
        };
        if <Manager as TransactionManager>::get_transaction_depth(&**tx) != self.depth {
            self.open = false;
            return Err(crate::Error::Internal(
                "nested transaction savepoints were left unbalanced".to_string(),
            ));
        }
        if rollback {
            <Manager as TransactionManager>::rollback(&mut **tx).await?;
        } else {
            <Manager as TransactionManager>::commit(&mut **tx).await?;
        }
        // Only a completed statement settles the savepoint; after a failure the
        // drop below still queues its rollback and restores the depth.
        self.open = false;
        Ok(())
    }
}

impl Drop for ManagedSavepoint {
    fn drop(&mut self) {
        if !self.open {
            return;
        }
        let Ok(mut guard) = self.transaction.try_lock() else {
            return;
        };
        if let Some(tx) = guard.as_mut()
            && <Manager as TransactionManager>::get_transaction_depth(&**tx) == self.depth
        {
            <Manager as TransactionManager>::start_rollback(&mut **tx);
        }
    }
}
