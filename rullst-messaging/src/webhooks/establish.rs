//! Bounded retry for opening a webhook outbox.
//!
//! Every outbox storage call has a 10-second deadline, `open` included, while
//! the SQLite pool beneath waits up to 30 seconds for a connection and for a
//! lock. Opening establishes the pooled connections (opening the file and its
//! WAL, and on a new file the WAL conversion with its full syncs), prepares the
//! schema and records the immutable configuration. Loaded hosted Windows
//! runners stall a process's file I/O for 5 to 12 seconds at times, and `open`
//! then failed with `Timeout` although the store itself would have waited.
//! Each attempt opens its own pool, which no other task uses yet, and every
//! step is idempotent (an expired attempt rolls back, or leaves committed state
//! that the next attempt verifies), so a `Timeout` there means a slow
//! establishment and opening is tried again. Every other call keeps the single
//! deadline.

use super::{Result, WebhookError};
use std::future::Future;

/// Attempts, in total, at opening an outbox whose deadline expires.
const ATTEMPTS: u32 = 3;

/// Runs `attempt` again while it fails with [`WebhookError::Timeout`], at most
/// [`ATTEMPTS`] times; any other outcome is returned as it is.
pub(super) async fn retry<T, F, Fut>(mut attempt: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for _ in 1..ATTEMPTS {
        match attempt().await {
            Err(WebhookError::Timeout) => {
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
    /// outbox on its own thread, so a test reads the retries that it caused.
    static RETRIES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_retry() {
    RETRIES.set(RETRIES.get() + 1);
}

/// The openings tried again on the current thread so far.
#[cfg(test)]
fn retries() -> u32 {
    RETRIES.get()
}

#[cfg(test)]
#[path = "establish_tests.rs"]
mod tests;
