//! Bounded retry for opening the SQLite stores: `SqliteBroker::connect` and
//! its variants, and `WebhookOutbox::open`.
//!
//! Each attempt of an opening has a deadline: 10 seconds, while the SQLite
//! pool beneath waits up to 30 seconds for a connection and SQLite's busy
//! handler waits as long for another connection's lock. Opening establishes
//! the pooled connections (opening the file and its WAL, and on a new file the
//! WAL conversion with its full syncs) and prepares the schema; the outbox
//! also records its immutable configuration. Loaded hosted Windows runners
//! stall a process's file I/O for 5 to 12 seconds at times, and the outbox's
//! `open` then failed with `Timeout` although the store itself would have
//! waited. Each attempt opens its own pool, which no other task uses yet, and
//! every step is idempotent (an expired attempt rolls back, or leaves
//! committed state that the next attempt verifies), so an expired deadline
//! means a slow establishment or another connection's lock, and opening is
//! tried again. The deadline is shorter than SQLite's waits, so it is what
//! ends an attempt on every platform, never a race between those waits.

use std::future::Future;

/// Attempts, in total, at an opening whose deadline expires.
const ATTEMPTS: u32 = 3;

/// Runs `attempt` again while it fails with an error that `expired` accepts,
/// at most [`ATTEMPTS`] times; any other outcome is returned as it is.
pub(crate) async fn retry<T, E, F, Fut>(
    mut attempt: F,
    expired: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    for _ in 1..ATTEMPTS {
        match attempt().await {
            Err(error) if expired(&error) => {
                #[cfg(test)]
                record_retry();
            }
            outcome => return outcome,
        }
    }
    attempt().await
}

#[cfg(test)]
std::thread_local! {
    /// Openings tried again on this thread. A `#[tokio::test]` polls the
    /// store on its own thread, so a test reads the retries that it caused.
    static RETRIES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_retry() {
    RETRIES.set(RETRIES.get() + 1);
}

/// The openings tried again on the current thread so far.
#[cfg(test)]
pub(crate) fn retries() -> u32 {
    RETRIES.get()
}

#[cfg(test)]
#[path = "establish_tests.rs"]
mod tests;
