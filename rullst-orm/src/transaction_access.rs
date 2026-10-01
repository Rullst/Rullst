//! Internal guard for implicit executor access from borrowed mutation callbacks.

use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

tokio::task_local! {
    static IN_MUTATION_CALLBACK: ();
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
    use super::{SharedTransaction, ensure_allowed, hold_for_stream};

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
