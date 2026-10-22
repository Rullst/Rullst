//! Bounded retry for the connections that `initialize` and `open` establish.
//!
//! Establishing a pooled connection (opening the file and its WAL, and on a
//! new file the WAL conversion with its full syncs) is the only disk work the
//! store bounds, by the pool's 5 s acquisition wait; statements and commits
//! wait for the disk instead. Loaded hosted Windows runners stall all file I/O
//! of a process for 5 to 12 seconds at times: `initialize` then failed with
//! `Storage`, leaving a new file that a second `initialize` refuses, while
//! every other store in the process only waited. No other task uses the pool
//! while `initialize` or `open` runs, so a `PoolTimedOut` there means that an
//! establishment outlived the wait, not contention, and it is tried again.

use std::future::Future;

/// Attempts, in total, at one bootstrap step whose connection times out.
pub(super) const ATTEMPTS: u32 = 3;

/// Runs `attempt` again while it fails with `PoolTimedOut`, at most
/// [`ATTEMPTS`] times; any other outcome is returned as it is.
pub(super) async fn retry<T, F, Fut>(mut attempt: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let mut made = 1;
    loop {
        match attempt().await {
            Err(sqlx::Error::PoolTimedOut) if made < ATTEMPTS => made += 1,
            outcome => return outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::retry;
    use std::cell::Cell;

    /// Times out until the tenth call, so an unbounded retry would succeed.
    async fn timeouts(calls: &Cell<u32>) -> Result<u32, sqlx::Error> {
        calls.set(calls.get() + 1);
        if calls.get() < 10 {
            Err(sqlx::Error::PoolTimedOut)
        } else {
            Ok(calls.get())
        }
    }

    #[tokio::test]
    async fn a_timed_out_step_is_tried_three_times_in_total() {
        let calls = Cell::new(0);
        let outcome = retry(|| timeouts(&calls)).await;
        assert!(matches!(outcome, Err(sqlx::Error::PoolTimedOut)));
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test]
    async fn the_third_attempt_may_succeed() {
        let calls = Cell::new(7);
        assert_eq!(retry(|| timeouts(&calls)).await.unwrap(), 10);
        assert_eq!(calls.get(), 10);
    }

    #[tokio::test]
    async fn other_failures_and_successes_are_not_retried() {
        let calls = Cell::new(0);
        let closed = retry(|| {
            calls.set(calls.get() + 1);
            async { Err::<(), _>(sqlx::Error::PoolClosed) }
        })
        .await;
        assert!(matches!(closed, Err(sqlx::Error::PoolClosed)));
        assert_eq!(calls.get(), 1);
        let succeeded = retry(|| {
            calls.set(calls.get() + 1);
            async { Ok::<_, sqlx::Error>(5) }
        })
        .await;
        assert_eq!(succeeded.unwrap(), 5);
        assert_eq!(calls.get(), 2);
    }
}
