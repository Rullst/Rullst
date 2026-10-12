//! Deadlines for worker tests: a worker that never observes shutdown, or never
//! reports an error, fails the test instead of hanging the test binary.

#![allow(clippy::expect_used)]

use super::{QueueError, WorkerHandle};
use std::time::Duration;

const BUDGET: Duration = Duration::from_secs(10);

/// Shuts `handle` down, failing the test when the worker does not stop in time.
pub(super) async fn shut_down(handle: WorkerHandle) -> Result<(), QueueError> {
    tokio::time::timeout(BUDGET, handle.shutdown())
        .await
        .expect("the worker stops within the deadline")
}

/// Waits for the next reported error, failing the test when none arrives in time.
pub(super) async fn next_error(handle: &mut WorkerHandle) -> Option<QueueError> {
    tokio::time::timeout(BUDGET, handle.next_error())
        .await
        .expect("an error is reported within the deadline")
}
