//! Bounded Traffic Shield database probe.

use std::future::Future;
use std::time::{Duration, Instant};

/// Latency recorded for a probe that returned an error.
const FAILED_PROBE_MS: u64 = 9_999;

/// Runs one probe and returns the latency to record, in milliseconds.
///
/// A probe still running at `deadline` is abandoned and recorded as having
/// taken at least `deadline`, so a hung or blackholed database connection
/// reports a critical latency instead of leaving the last healthy value in
/// place indefinitely.
#[cfg_attr(not(feature = "orm"), allow(dead_code))]
pub(super) async fn measure<T, E>(
    deadline: Duration,
    probe: impl Future<Output = Result<T, E>>,
) -> u64 {
    let start = Instant::now();
    match tokio::time::timeout(deadline, probe).await {
        Ok(Ok(_)) => super::duration_millis_u64(start.elapsed()),
        Ok(Err(_)) => FAILED_PROBE_MS,
        Err(_) => super::duration_millis_u64(start.elapsed().max(deadline)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_hung_probe_is_recorded_at_its_deadline() {
        let deadline = Duration::from_millis(50);
        let latency = tokio::time::timeout(
            Duration::from_secs(5),
            measure(deadline, std::future::pending::<Result<(), ()>>()),
        )
        .await
        .unwrap_or(0);
        assert!(latency >= 50, "recorded {latency} ms");
    }

    #[tokio::test]
    async fn completed_and_failed_probes_keep_their_measurement() {
        let quick = measure(Duration::from_secs(5), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok::<_, ()>(())
        })
        .await;
        assert!((20..5_000).contains(&quick), "recorded {quick} ms");
        let failed = measure(Duration::from_secs(5), async { Err::<(), _>(()) }).await;
        assert_eq!(failed, FAILED_PROBE_MS);
    }
}
