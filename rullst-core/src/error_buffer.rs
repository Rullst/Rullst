//! Bounded error reporting shared by the queue worker and the scheduler.
//!
//! Background loops report failures through a bounded channel, so a handle
//! that is kept alive but never drained cannot grow memory without limit.
//! When the buffer is full, the newest error is dropped, counted and emitted
//! as a `tracing` warning so it remains observable.

use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc::{self, error::TrySendError};

/// Maximum number of undrained errors buffered by a worker or scheduler handle.
pub(crate) const ERROR_BUFFER_CAPACITY: usize = 256;

/// Sending half used by background loops; never blocks and never grows.
pub(crate) struct ErrorReporter<E> {
    sender: mpsc::Sender<E>,
    dropped: Arc<AtomicU64>,
    component: &'static str,
}

impl<E> Clone for ErrorReporter<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            dropped: Arc::clone(&self.dropped),
            component: self.component,
        }
    }
}

impl<E: Display> ErrorReporter<E> {
    /// Buffers `error`, or drops, counts and logs it when the buffer is full.
    ///
    /// An error reported after the handle was dropped is discarded silently,
    /// because nothing can observe it any more.
    pub(crate) fn report(&self, error: E) {
        if let Err(TrySendError::Full(error)) = self.sender.try_send(error) {
            let dropped = self
                .dropped
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::warn!(
                component = self.component,
                dropped,
                %error,
                "background error buffer is full; dropping the error"
            );
        }
    }
}

/// Receiving half owned by a lifecycle handle.
pub(crate) struct ErrorBuffer<E> {
    receiver: mpsc::Receiver<E>,
    dropped: Arc<AtomicU64>,
}

impl<E> ErrorBuffer<E> {
    pub(crate) async fn next(&mut self) -> Option<E> {
        self.receiver.recv().await
    }

    pub(crate) fn try_next(&mut self) -> Option<E> {
        self.receiver.try_recv().ok()
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Creates a reporter/buffer pair holding at most `capacity` errors.
pub(crate) fn error_buffer<E>(
    capacity: usize,
    component: &'static str,
) -> (ErrorReporter<E>, ErrorBuffer<E>) {
    let (sender, receiver) = mpsc::channel(capacity.max(1));
    let dropped = Arc::new(AtomicU64::new(0));
    (
        ErrorReporter {
            sender,
            dropped: Arc::clone(&dropped),
            component,
        },
        ErrorBuffer { receiver, dropped },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_drops_newest_errors_and_counts_them() {
        let (reporter, mut buffer) = error_buffer::<String>(2, "test");
        for index in 0..5 {
            reporter.report(format!("error {index}"));
        }

        assert_eq!(buffer.try_next().as_deref(), Some("error 0"));
        assert_eq!(buffer.try_next().as_deref(), Some("error 1"));
        assert_eq!(buffer.try_next(), None);
        assert_eq!(buffer.dropped(), 3);

        reporter.report("after drain".to_string());
        assert_eq!(buffer.try_next().as_deref(), Some("after drain"));
        assert_eq!(buffer.dropped(), 3);
    }

    #[test]
    fn reporting_after_the_handle_is_dropped_is_silent() {
        let (reporter, buffer) = error_buffer::<String>(1, "test");
        let dropped = Arc::clone(&buffer.dropped);
        drop(buffer);
        reporter.report("unobserved".to_string());
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }
}
