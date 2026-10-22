//! Internal guard for implicit executor access from borrowed mutation callbacks.

use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

tokio::task_local! {
    static IN_MUTATION_CALLBACK: ();
    static IN_MIGRATION_TRANSACTION: &'static str;
}

/// Managed transactions whose connection an open generated `stream()` holds,
/// identified by the address of their shared handle. The count lets
/// `ensure_allowed` skip the lock while no such stream is open.
static STREAMING_TRANSACTIONS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static STREAMING_COUNT: AtomicUsize = AtomicUsize::new(0);

type SharedTransaction =
    std::sync::Arc<tokio::sync::Mutex<Option<crate::db::Transaction<'static>>>>;

/// Generated mutation policies/hooks borrow an executor that cannot be lent
/// again through the process-global API. This scope restores itself on return,
/// cancellation, or unwind and does not propagate into independently spawned tasks.
pub async fn run<F: Future>(callback: F) -> F::Output {
    IN_MUTATION_CALLBACK.scope((), callback).await
}

/// Runs the `up()`/`down()` of the named migration, which the runner applies
/// inside its transaction. Until `body` finishes, `Orm::pool()` and
/// `Orm::read_pool()` on this task fail instead of handing out a connection
/// outside that transaction. Like the transaction itself, the scope does not
/// propagate into independently spawned tasks.
pub(crate) async fn run_migration<F: Future>(name: &'static str, body: F) -> F::Output {
    IN_MIGRATION_TRANSACTION.scope(name, body).await
}

/// Refuses direct pool access from a migration body that runs inside the
/// runner's transaction (see [`run_migration`]).
pub(crate) fn ensure_pool_allowed() -> Result<(), crate::Error> {
    match IN_MIGRATION_TRANSACTION.try_with(|name| *name) {
        Ok(name) => Err(crate::Error::Validation(format!(
            "migration {name} returns true from within_transaction(), so Orm::pool() and Orm::read_pool() are unavailable in its up()/down(): they would run on another connection outside the migration's transaction; use Schema, Orm::raw(sql).execute() or execute_query!, or return false from within_transaction()"
        ))),
        Err(_) => Ok(()),
    }
}

pub fn ensure_allowed() -> Result<(), crate::Error> {
    if IN_MUTATION_CALLBACK.try_with(|()| ()).is_ok() {
        return Err(crate::Error::Validation(
            "reentrant ORM access from a mutation policy or lifecycle callback is unsupported while its transaction is borrowed; perform database authorization before the mutation and defer only post-commit effects to after_commit".to_string(),
        ));
    }
    if STREAMING_COUNT.load(Ordering::Acquire) > 0
        && let Ok(transaction) = crate::CURRENT_TX.try_with(transaction_key)
        && streaming_transactions().contains(&transaction)
    {
        return Err(crate::Error::Validation(
            "an open stream() holds this managed transaction until it is consumed or dropped; finish or drop the stream (or collect rows with get() or chunk_by_id()) before other ORM calls on the transaction".to_string(),
        ));
    }
    Ok(())
}

/// Marks a managed transaction as held by an open generated `stream()` until
/// the returned guard is dropped. Other ORM calls on that transaction then fail
/// with `Validation` instead of waiting for its lock, which the stream keeps
/// between rows, forever.
#[doc(hidden)]
pub fn hold_for_stream(transaction: &SharedTransaction) -> StreamHold {
    let key = transaction_key(transaction);
    streaming_transactions().push(key);
    STREAMING_COUNT.fetch_add(1, Ordering::AcqRel);
    StreamHold { key }
}

/// Releases a transaction marked by [`hold_for_stream`] when dropped.
#[doc(hidden)]
#[must_use = "the stream hold is released when the guard is dropped"]
pub struct StreamHold {
    key: usize,
}

impl Drop for StreamHold {
    fn drop(&mut self) {
        let mut held = streaming_transactions();
        if let Some(position) = held.iter().position(|key| *key == self.key) {
            held.swap_remove(position);
            STREAMING_COUNT.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

fn transaction_key(transaction: &SharedTransaction) -> usize {
    std::sync::Arc::as_ptr(transaction).addr()
}

fn streaming_transactions() -> std::sync::MutexGuard<'static, Vec<usize>> {
    STREAMING_TRANSACTIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::{
        SharedTransaction, ensure_allowed, ensure_pool_allowed, hold_for_stream, run_migration,
    };

    #[tokio::test]
    async fn pool_access_fails_only_inside_a_transactional_migration_body() {
        let expected = "migration m20261022_000001_create_widgets returns true from within_transaction(), so Orm::pool() and Orm::read_pool() are unavailable in its up()/down(): they would run on another connection outside the migration's transaction; use Schema, Orm::raw(sql).execute() or execute_query!, or return false from within_transaction()";
        let refused = |result: Result<(), crate::Error>| matches!(result, Err(crate::Error::Validation(message)) if message == expected);
        let (guard, pool, read_pool, begin, nested) =
            run_migration("m20261022_000001_create_widgets", async {
                (
                    ensure_pool_allowed(),
                    crate::Orm::try_pool().map(|_| ()),
                    crate::Orm::try_read_pool().map(|_| ()),
                    crate::Orm::begin_transaction().await.map(|_| ()),
                    async { ensure_pool_allowed() }.await,
                )
            })
            .await;
        assert!(refused(guard));
        assert!(refused(pool));
        assert!(refused(read_pool));
        assert!(refused(begin));
        assert!(refused(nested));
        // The scope ends with the body: this test never initializes the ORM.
        assert!(ensure_pool_allowed().is_ok());
        assert!(matches!(
            crate::Orm::try_pool(),
            Err(crate::Error::NotInitialized)
        ));
        let spawned = run_migration("m20261022_000001_create_widgets", async {
            tokio::spawn(async { ensure_pool_allowed() }).await
        })
        .await;
        assert!(matches!(spawned, Ok(Ok(()))));
    }

    #[tokio::test]
    async fn a_held_transaction_rejects_other_calls_until_the_hold_drops() {
        let held: SharedTransaction = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let other: SharedTransaction = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let hold = hold_for_stream(&held);
        let in_held = crate::CURRENT_TX
            .scope(held.clone(), async { ensure_allowed() })
            .await;
        assert!(
            matches!(in_held, Err(crate::Error::Validation(message)) if message.contains("open stream()"))
        );
        let in_other = crate::CURRENT_TX
            .scope(other, async { ensure_allowed() })
            .await;
        assert!(in_other.is_ok());
        assert!(ensure_allowed().is_ok());
        drop(hold);
        let released = crate::CURRENT_TX
            .scope(held, async { ensure_allowed() })
            .await;
        assert!(released.is_ok());
    }
}
