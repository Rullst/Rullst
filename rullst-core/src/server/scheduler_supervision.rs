//! Observes a `Server`-owned scheduler and scopes its shutdown result.
//!
//! `Server::schedule` takes the only `SchedulerHandle`, so the application
//! cannot drain `next_error` itself. The server therefore logs every task
//! failure as it is reported, and a task failure from before shutdown never
//! turns a clean HTTP drain into an error exit.

use super::ServerError;
use crate::scheduler::{SchedulerError, SchedulerHandle};
use std::future::Future;

/// Runs `server` to completion while logging each reported task failure.
pub(crate) async fn serve_while_draining<F>(
    server: F,
    scheduler: Option<&mut SchedulerHandle>,
) -> Result<(), ServerError>
where
    F: Future<Output = Result<(), ServerError>>,
{
    let Some(scheduler) = scheduler else {
        return server.await;
    };
    tokio::pin!(server);
    let mut draining = true;
    loop {
        tokio::select! {
            result = &mut server => return result,
            failure = scheduler.next_error(), if draining => match failure {
                Some(error) => log_task_failure(&error),
                // Every loop has ended; nothing more can be reported.
                None => draining = false,
            },
        }
    }
}

/// Stops the scheduler after the HTTP server has finished.
///
/// Task failures (timeouts, panics, cancellations, exhausted schedules) are
/// logged; only a failure of a scheduler loop itself is returned.
pub(crate) async fn stop_scheduler(scheduler: Option<SchedulerHandle>) -> Result<(), ServerError> {
    let Some(mut scheduler) = scheduler else {
        return Ok(());
    };
    while let Some(error) = scheduler.try_next_error() {
        log_task_failure(&error);
    }
    match scheduler.shutdown().await {
        Ok(()) => Ok(()),
        Err(error @ SchedulerError::LoopFailed { .. }) => Err(ServerError::Scheduler(error)),
        Err(error) => {
            log_task_failure(&error);
            Ok(())
        }
    }
}

fn log_task_failure(error: &SchedulerError) {
    tracing::error!(target: "rullst::scheduler", %error, "scheduled task failed");
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub(crate) mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Collects formatted `tracing` output for assertions.
    #[derive(Clone, Default)]
    pub(crate) struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

    impl CapturedLogs {
        pub(crate) fn subscriber(&self) -> impl tracing::Subscriber + Send + Sync + 'static {
            let logs = self.clone();
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(move || logs.clone())
                .finish()
        }

        pub(crate) fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    impl std::io::Write for CapturedLogs {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn timing_out_scheduler() -> crate::scheduler::Scheduler {
        crate::scheduler::contract_tests::every_second(Duration::from_millis(5), || async {
            tokio::time::sleep(Duration::from_secs(1)).await;
        })
    }

    #[tokio::test]
    async fn failures_are_logged_while_serving_and_not_returned_at_shutdown() {
        let logs = CapturedLogs::default();
        let _subscriber = tracing::subscriber::set_default(logs.subscriber());
        let mut scheduler = Some(timing_out_scheduler().start().unwrap());

        let served = serve_while_draining(
            async {
                tokio::time::sleep(Duration::from_millis(2_300)).await;
                Ok(())
            },
            scheduler.as_mut(),
        )
        .await;
        assert!(served.is_ok());
        assert!(
            logs.text().contains("exceeded its 5ms timeout"),
            "a failure must be logged while the server runs: {}",
            logs.text()
        );

        assert!(stop_scheduler(scheduler).await.is_ok());
    }

    #[tokio::test]
    async fn a_buffered_failure_does_not_fail_shutdown() {
        let logs = CapturedLogs::default();
        let _subscriber = tracing::subscriber::set_default(logs.subscriber());
        let scheduler = timing_out_scheduler().start().unwrap();
        tokio::time::sleep(Duration::from_millis(2_300)).await;

        assert!(stop_scheduler(Some(scheduler)).await.is_ok());
        assert!(logs.text().contains("scheduled task failed"));
        assert!(stop_scheduler(None).await.is_ok());
    }
}
