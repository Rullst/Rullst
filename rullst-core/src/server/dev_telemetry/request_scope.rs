//! Correlates ORM operations with the request whose task started them.
//!
//! The recording middleware (development telemetry or the production
//! monitor) runs each request inside a task-local scope. The ORM layer
//! captures that scope when an outermost operation span is created and, when
//! the span closes, counts the operation and adds its label fingerprint.
//! Spans created on another task (for example inside `tokio::spawn`) have no
//! scope and are not attributed to any request, so repetitions are
//! undercounted, never invented.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// Fingerprints kept per request; later operations are not attributed.
const MAX_OPERATIONS_PER_REQUEST: usize = 4_096;

tokio::task_local! {
    static CURRENT: RequestOperations;
}

/// The ORM operations of one request.
#[derive(Clone, Debug, Default)]
pub(crate) struct RequestOperations(Arc<Operations>);

#[derive(Debug, Default)]
struct Operations {
    fingerprints: Mutex<Vec<String>>,
    count: AtomicU64,
}

/// The ORM work one request (or part of it) started.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RequestQueries {
    /// Outermost ORM operations, with or without a fingerprint.
    pub(crate) operations: u64,
    /// Fingerprints of the operations that have a model or table label.
    pub(crate) fingerprints: Vec<String>,
}

/// Position in a scope from which [`RequestOperations::since`] reports.
#[derive(Debug, Clone, Copy)]
struct Mark {
    fingerprints: usize,
    operations: u64,
}

impl RequestOperations {
    /// The scope of the request running on this task, if any.
    pub(crate) fn current() -> Option<Self> {
        CURRENT.try_with(Clone::clone).ok()
    }

    /// Counts one operation and keeps its fingerprint, if it has one.
    pub(super) fn record(&self, fingerprint: Option<String>) {
        self.0.count.fetch_add(1, Ordering::Relaxed);
        let Some(fingerprint) = fingerprint else {
            return;
        };
        let mut fingerprints = self
            .0
            .fingerprints
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if fingerprints.len() < MAX_OPERATIONS_PER_REQUEST {
            fingerprints.push(fingerprint);
        }
    }

    fn mark(&self) -> Mark {
        Mark {
            fingerprints: self.fingerprints().len(),
            operations: self.0.count.load(Ordering::Relaxed),
        }
    }

    /// What was recorded after `mark`, leaving the scope unchanged.
    fn since(&self, mark: Mark) -> RequestQueries {
        let fingerprints = self.fingerprints();
        RequestQueries {
            operations: self
                .0
                .count
                .load(Ordering::Relaxed)
                .saturating_sub(mark.operations),
            fingerprints: fingerprints
                .get(mark.fingerprints..)
                .unwrap_or_default()
                .to_vec(),
        }
    }

    fn take(&self) -> RequestQueries {
        RequestQueries {
            operations: self.0.count.swap(0, Ordering::Relaxed),
            fingerprints: std::mem::take(&mut *self.fingerprints()),
        }
    }

    fn fingerprints(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.0
            .fingerprints
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runs `future` as one request and returns the fingerprints of the ORM
/// operations it started.
pub(super) async fn observe<F: Future>(future: F) -> (F::Output, Vec<String>) {
    let operations = RequestOperations::default();
    let output = CURRENT.scope(operations.clone(), future).await;
    (output, operations.take().fingerprints)
}

/// Like [`observe`], but when an enclosing recorder already opened a scope
/// for this request, joins it and reports only what `future` added, so both
/// recorders see the same operations.
pub(crate) async fn observe_shared<F: Future>(future: F) -> (F::Output, RequestQueries) {
    if let Some(enclosing) = RequestOperations::current() {
        let mark = enclosing.mark();
        let output = future.await;
        return (output, enclosing.since(mark));
    }
    let operations = RequestOperations::default();
    let output = CURRENT.scope(operations.clone(), future).await;
    (output, operations.take())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn record(count: usize, fingerprint: Option<&str>) {
        let scope = RequestOperations::current().unwrap_or_default();
        for _ in 0..count {
            scope.record(fingerprint.map(str::to_string));
        }
    }

    #[tokio::test]
    async fn a_scope_counts_every_operation_and_keeps_labelled_fingerprints() {
        let ((), queries) = observe_shared(async {
            record(2, Some("Post.find (posts)"));
            record(3, None);
        })
        .await;
        assert_eq!(queries.operations, 5);
        assert_eq!(
            queries.fingerprints,
            ["Post.find (posts)", "Post.find (posts)"]
        );
    }

    #[tokio::test]
    async fn a_nested_recorder_sees_only_what_ran_inside_it_and_leaves_the_outer_scope_intact() {
        let ((inner, after_inner), outer) = observe(async {
            record(1, Some("User.find (users)"));
            let ((), inner) = observe_shared(async {
                record(2, Some("Post.find (posts)"));
                record(1, None);
            })
            .await;
            let current = RequestOperations::current().unwrap();
            (inner, current.mark().operations)
        })
        .await;

        assert_eq!(inner.operations, 3);
        assert_eq!(
            inner.fingerprints,
            ["Post.find (posts)", "Post.find (posts)"]
        );
        assert_eq!(after_inner, 4);
        assert_eq!(
            outer,
            [
                "User.find (users)",
                "Post.find (posts)",
                "Post.find (posts)"
            ]
        );
    }

    #[test]
    fn fingerprints_beyond_the_per_request_bound_are_counted_but_not_kept() {
        let scope = RequestOperations::default();
        for _ in 0..MAX_OPERATIONS_PER_REQUEST + 2 {
            scope.record(Some("Post.find (posts)".to_string()));
        }
        let queries = scope.take();
        assert_eq!(queries.operations, 4_098);
        assert_eq!(queries.fingerprints.len(), 4_096);
        assert_eq!(scope.take(), RequestQueries::default());
    }
}
