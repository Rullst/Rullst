//! Bounded retry for the waits of `initialize` and `open`.
//!
//! Establishing a pooled connection (opening the file and its WAL, and on a
//! new file the WAL conversion with its full syncs) is the only disk work the
//! store bounds, by the pool's 5 s acquisition wait; statements and commits
//! wait for the disk instead. Loaded hosted Windows runners stall all file I/O
//! of a process for 5 to 12 seconds at times: `initialize` then failed with
//! `Storage`, leaving a new file that a second `initialize` refuses, while
//! every other store in the process only waited. Another connection that keeps
//! the file locked delays a step as well, and SQLite's busy handler waits 5 s
//! for that lock, as long as the pool: which wait runs out first
//! (`PoolTimedOut` or `SQLITE_BUSY`) depends on the platform's timers. No
//! other task uses the pool while `initialize` or `open` runs, so either means
//! a slow establishment or another connection's lock, and the step is tried
//! again. A step whose waits run out at every attempt fails with
//! `StorageTimeout`.

use std::{future::Future, time::Duration};

/// Attempts, in total, at one bootstrap step whose wait runs out.
pub(super) const ATTEMPTS: u32 = 3;

/// How long a connection waits: for the pool to establish or lend it, and in
/// SQLite's busy handler for another connection's lock.
#[derive(Debug, Clone, Copy)]
pub(super) struct Waits {
    pub(super) pool: Duration,
    pub(super) lock: Duration,
}

impl Waits {
    /// The store's waits, the same as the Media, Supervision and Privacy
    /// stores': a full-sync WAL commit on a slow disk can hold the write lock
    /// for seconds.
    pub(super) const STORE: Self = Self {
        pool: Duration::from_secs(5),
        lock: Duration::from_secs(5),
    };
}

/// Whether `error` ended a wait that ran out: the pool's (`PoolTimedOut`) or
/// SQLite's busy handler (`SQLITE_BUSY`, 5, with any extended code).
pub(super) fn outlived(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::PoolTimedOut => true,
        sqlx::Error::Database(error) => error
            .code()
            .and_then(|code| code.parse::<i32>().ok())
            .is_some_and(|code| code & 0xff == 5),
        _ => false,
    }
}

/// Runs `attempt` again while it fails because a wait ran out ([`outlived`]),
/// at most [`ATTEMPTS`] times; any other outcome is returned as it is.
pub(super) async fn retry<T, F, Fut>(mut attempt: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let mut made = 1;
    loop {
        match attempt().await {
            Err(error) if made < ATTEMPTS && outlived(&error) => {
                made += 1;
                #[cfg(test)]
                count_retry();
            }
            outcome => return outcome,
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Steps tried again on this thread. A `#[tokio::test]` polls the store
    /// on its own thread, so a test reads the retries that it caused.
    static RETRIED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_retry() {
    RETRIED.with(|retried| retried.set(retried.get() + 1));
}

/// The steps tried again on the current thread so far.
#[cfg(test)]
pub(super) fn retried() -> u32 {
    RETRIED.with(std::cell::Cell::get)
}

#[cfg(test)]
#[path = "establish_tests.rs"]
mod locked;

#[cfg(test)]
mod tests {
    use super::{outlived, retry};
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

    /// A database error with only a result code.
    #[derive(Debug)]
    struct Code(&'static str);

    impl std::fmt::Display for Code {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for Code {}

    impl sqlx::error::DatabaseError for Code {
        fn message(&self) -> &str {
            self.0
        }
        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(self.0.into())
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    #[test]
    fn busy_results_and_pool_timeouts_are_waits_that_ran_out() {
        // SQLITE_BUSY and its extended codes: RECOVERY, SNAPSHOT, TIMEOUT.
        for code in ["5", "261", "517", "773"] {
            assert!(
                outlived(&sqlx::Error::Database(Box::new(Code(code)))),
                "{code}"
            );
        }
        // SQLITE_LOCKED, SQLITE_ERROR, SQLITE_CONSTRAINT_UNIQUE, no number.
        for code in ["6", "1", "2067", "busy"] {
            assert!(
                !outlived(&sqlx::Error::Database(Box::new(Code(code)))),
                "{code}"
            );
        }
        assert!(outlived(&sqlx::Error::PoolTimedOut));
        assert!(!outlived(&sqlx::Error::PoolClosed));
        assert!(!outlived(&sqlx::Error::RowNotFound));
    }
}
