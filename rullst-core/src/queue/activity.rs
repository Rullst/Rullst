//! Process-local counters of what this process's workers did with a queue.
//!
//! A [`super::Queue`] owns one [`QueueActivity`] and every [`super::Worker`]
//! built from it shares it, so the production monitor
//! ([`crate::monitor`]) can report in-flight jobs and completed, failed and
//! retried transitions without querying the store. The counters cover only
//! this process since it started; the pending count comes from the store.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counters shared by a queue and its workers.
#[derive(Debug, Default)]
pub(crate) struct QueueActivity {
    in_flight: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    retried: AtomicU64,
    stalled_recovered: AtomicU64,
}

/// A copy of the counters at one instant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct QueueActivitySnapshot {
    /// Handlers running now.
    pub(crate) in_flight: u64,
    /// Jobs marked complete.
    pub(crate) completed: u64,
    /// Jobs marked failed (handler error, panic, timeout or no handler).
    pub(crate) failed: u64,
    /// Jobs returned to pending for another attempt: requeued on shutdown,
    /// handed back for lack of a handler, or retried with
    /// [`super::Queue::retry_failed_job`].
    pub(crate) retried: u64,
    /// Stalled leases the driver recovered (requeued or failed).
    pub(crate) stalled_recovered: u64,
}

impl QueueActivity {
    /// Counts one running handler until the returned guard is dropped.
    pub(crate) fn start(self: &Arc<Self>) -> InFlight {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        InFlight(Arc::clone(self))
    }

    pub(crate) fn completed(&self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn retried(&self) {
        self.retried.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn stalled_recovered(&self, count: u64) {
        self.stalled_recovered.fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> QueueActivitySnapshot {
        QueueActivitySnapshot {
            in_flight: self.in_flight.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            retried: self.retried.load(Ordering::Relaxed),
            stalled_recovered: self.stalled_recovered.load(Ordering::Relaxed),
        }
    }
}

/// One running handler; dropping it (on completion, cancellation or panic)
/// decrements the in-flight count.
#[derive(Debug)]
pub(crate) struct InFlight(Arc<QueueActivity>);

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_start_at_zero_and_count_each_transition() {
        let activity = Arc::new(QueueActivity::default());
        assert_eq!(activity.snapshot(), QueueActivitySnapshot::default());

        let first = activity.start();
        let second = activity.start();
        activity.completed();
        activity.failed();
        activity.failed();
        activity.retried();
        activity.stalled_recovered(3);
        activity.stalled_recovered(2);
        assert_eq!(
            activity.snapshot(),
            QueueActivitySnapshot {
                in_flight: 2,
                completed: 1,
                failed: 2,
                retried: 1,
                stalled_recovered: 5,
            }
        );

        drop(first);
        assert_eq!(activity.snapshot().in_flight, 1);
        drop(second);
        assert_eq!(activity.snapshot().in_flight, 0);
    }
}
