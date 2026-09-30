//! Savepoints that let a nested [`Orm::transaction`](super::Orm::transaction)
//! join the task-scoped transaction.
//!
//! The savepoint is opened, released and rolled back through SQLx's own
//! transaction manager, so SQLx keeps tracking the nesting depth that generated
//! model savepoints and the outer commit rely on.
//!
//! Savepoints on one connection form a stack, so sibling nested transactions
//! (for example futures joined with `tokio::join!` on one task) take turns:
//! each level has a gate that a nested transaction holds from its `SAVEPOINT`
//! until its `RELEASE`/`ROLLBACK TO`. A nested transaction started inside
//! another one waits on that one's own gate, so real nesting never waits for
//! its parent.
//!
//! Dropping an unfinished savepoint (for example when its future is cancelled)
//! queues its rollback, exactly like dropping a SQLx `Transaction`. When the
//! connection is busy at that moment the savepoint stays open and the enclosing
//! level fails closed: a parent savepoint rolls back instead of releasing, a
//! managed transaction rolls back instead of committing, and the pool closes a
//! connection that is returned while still inside a transaction.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use futures::future::BoxFuture;
use sqlx_core::transaction::TransactionManager;

use crate::database::RullstDatabase;

type Manager = <RullstDatabase as sqlx::Database>::TransactionManager;
type Connection = <RullstDatabase as sqlx::Database>::Connection;
pub(super) type SharedTransaction =
    Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>;

/// Serializes the savepoints opened directly on one level.
type Gate = Arc<tokio::sync::Mutex<()>>;

tokio::task_local! {
    /// The managed level whose closure is running; nested transactions started
    /// from it open their savepoints one at a time through its gate.
    static LEVEL: Level;
}

struct Level {
    transaction: SharedTransaction,
    children: Gate,
}

/// Gates of task-scoped transactions not opened by `Orm::transaction` (such as
/// the `#[rullst_orm::test]` sandbox), keyed by the shared handle's address.
/// Every holder of a live gate also holds a clone of that handle, so the
/// address cannot be reused while its entry is alive.
static HANDLE_GATES: Mutex<BTreeMap<usize, Weak<tokio::sync::Mutex<()>>>> =
    Mutex::new(BTreeMap::new());

fn handle_gate(transaction: &SharedTransaction) -> Gate {
    let key = Arc::as_ptr(transaction).addr();
    let mut gates = HANDLE_GATES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(gate) = gates.get(&key).and_then(Weak::upgrade) {
        return gate;
    }
    gates.retain(|_, gate| gate.strong_count() > 0);
    let gate = Gate::default();
    gates.insert(key, Arc::downgrade(&gate));
    gate
}

fn parent_gate(transaction: &SharedTransaction) -> Gate {
    LEVEL
        .try_with(|level| {
            Arc::ptr_eq(&level.transaction, transaction).then(|| level.children.clone())
        })
        .ok()
        .flatten()
        .unwrap_or_else(|| handle_gate(transaction))
}

/// Runs a managed transaction's closure as the level that its nested
/// transactions join.
pub(super) fn run_level<F: Future>(
    transaction: SharedTransaction,
    future: F,
) -> impl Future<Output = F::Output> {
    LEVEL.scope(
        Level {
            transaction,
            children: Gate::default(),
        },
        future,
    )
}

fn current_depth(connection: &Connection) -> usize {
    <Manager as TransactionManager>::get_transaction_depth(connection)
}

/// Rolls back every savepoint or transaction level above `depth`.
async fn unwind(connection: &mut Connection, depth: usize) -> Result<(), crate::Error> {
    for _ in depth..current_depth(connection) {
        <Manager as TransactionManager>::rollback(connection).await?;
    }
    Ok(())
}

/// Queues SQLx's rollback of every level above `depth`, as dropping a SQLx
/// transaction does. The count is fixed first because SQLite applies the
/// queued statements, and their depth changes, on its worker thread.
fn queue_unwind(connection: &mut Connection, depth: usize) {
    for _ in depth..current_depth(connection) {
        <Manager as TransactionManager>::start_rollback(connection);
    }
}

/// Rolls back savepoints still open above a managed transaction that is about
/// to finish, and reports whether there were any.
pub(super) async fn unwind_leaked(
    transaction: &mut crate::db::Transaction<'static>,
) -> Result<bool, crate::Error> {
    let leaked = current_depth(transaction) > 1;
    unwind(transaction, 1).await?;
    Ok(leaked)
}

/// Pool `after_release` hook: a connection whose transaction is still open is
/// closed, which makes the server discard that work, instead of being reused.
pub(super) fn release_outside_transaction(
    connection: &mut Connection,
    _metadata: sqlx::pool::PoolConnectionMetadata,
) -> BoxFuture<'_, Result<bool, sqlx::Error>> {
    Box::pin(async move {
        if current_depth(connection) == 0 {
            return Ok(true);
        }
        // SQLite applies a queued rollback on its worker thread; the ping
        // runs after it, so the depth is settled when it returns.
        sqlx::Connection::ping(connection).await?;
        Ok(current_depth(connection) == 0)
    })
}

/// One open savepoint on a shared managed transaction.
pub(super) struct ManagedSavepoint {
    transaction: SharedTransaction,
    depth: usize,
    open: bool,
    /// This level's turn, released only after the savepoint is settled.
    _turn: tokio::sync::OwnedMutexGuard<()>,
}

impl ManagedSavepoint {
    /// Opens a savepoint on `transaction`, or returns `None` when no managed
    /// transaction is available in the shared handle.
    pub(super) fn begin(
        transaction: SharedTransaction,
    ) -> BoxFuture<'static, Result<Option<Self>, crate::Error>> {
        Box::pin(async move {
            // Wait until the previous sibling on this level has settled its
            // savepoint, so savepoints always close in LIFO order.
            let turn = parent_gate(&transaction).lock_owned().await;
            // Declared before the lock guard, so on cancellation the guard is
            // released first and `Drop` can roll back a savepoint that the
            // interrupted statement did open.
            let mut savepoint = Self {
                transaction: transaction.clone(),
                depth: 0,
                open: false,
                _turn: turn,
            };
            let mut guard = transaction.lock_owned().await;
            let Some(tx) = guard.as_mut() else {
                return Ok(None);
            };
            savepoint.depth = current_depth(tx) + 1;
            savepoint.open = true;
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
        let current = current_depth(tx);
        if current < self.depth {
            self.open = false;
            return Err(crate::Error::Internal(
                "nested transaction savepoint was already closed by another operation".to_string(),
            ));
        }
        let parent = self.depth.saturating_sub(1);
        let result = if current > self.depth {
            // Releasing would keep the partial work of the deeper savepoint
            // left open, so discard everything since this savepoint.
            unwind(tx, parent).await.and(Err(crate::Error::Internal(
                "nested transaction savepoints were left unbalanced; the nested work was rolled back"
                    .to_string(),
            )))
        } else if rollback {
            unwind(tx, parent).await
        } else {
            <Manager as TransactionManager>::commit(&mut **tx)
                .await
                .map_err(Into::into)
        };
        if result.is_err() {
            // Never leave the connection a level deeper than its owner expects.
            queue_unwind(tx, parent);
        }
        self.open = false;
        result
    }
}

impl Drop for ManagedSavepoint {
    fn drop(&mut self) {
        if !self.open {
            return;
        }
        // While another operation holds the connection the savepoint stays
        // open; the enclosing level then fails closed when it finishes.
        let Ok(mut guard) = self.transaction.try_lock() else {
            return;
        };
        if let Some(tx) = guard.as_mut() {
            queue_unwind(tx, self.depth.saturating_sub(1));
        }
    }
}
