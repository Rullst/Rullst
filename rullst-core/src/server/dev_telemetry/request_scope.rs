//! Correlates ORM operations with the request whose task started them.
//!
//! The recording middleware runs each request inside a task-local scope. The
//! ORM layer captures that scope when an outermost operation span is created
//! and adds the operation's label fingerprint when the span closes. Spans
//! created on another task (for example inside `tokio::spawn`) have no scope
//! and are not attributed to any request, so repetitions are undercounted,
//! never invented.

use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

/// Fingerprints kept per request; later operations are not attributed.
const MAX_OPERATIONS_PER_REQUEST: usize = 4_096;

tokio::task_local! {
    static CURRENT: RequestOperations;
}

/// The ORM operation fingerprints of one request.
#[derive(Clone, Debug, Default)]
pub(super) struct RequestOperations(Arc<Mutex<Vec<String>>>);

impl RequestOperations {
    /// The scope of the request running on this task, if any.
    pub(super) fn current() -> Option<Self> {
        CURRENT.try_with(Clone::clone).ok()
    }

    pub(super) fn record(&self, fingerprint: String) {
        let mut operations = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if operations.len() < MAX_OPERATIONS_PER_REQUEST {
            operations.push(fingerprint);
        }
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Runs `future` as one request and returns the fingerprints of the ORM
/// operations it started.
pub(super) async fn observe<F: Future>(future: F) -> (F::Output, Vec<String>) {
    let operations = RequestOperations::default();
    let output = CURRENT.scope(operations.clone(), future).await;
    (output, operations.take())
}
