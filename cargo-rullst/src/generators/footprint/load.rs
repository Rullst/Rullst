//! A bounded closed-loop HTTP load: `concurrency` workers each send one GET
//! at a time until the duration ends. Bodies are drained, not stored.
use std::time::{Duration, Instant};

pub(super) const METHOD: &str = "closed loop: each connection sends the next GET only after the \
    previous response body was fully read; latency is measured per request from send to last byte";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Latency samples kept across all workers (4 bytes each).
const SAMPLE_BUDGET: usize = 4_000_000;

/// What one load run observed.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Outcome {
    /// Requests that received a complete response, any status.
    pub responses: u64,
    /// Connection failures, timeouts and truncated bodies.
    pub transport_errors: u64,
    /// Responses with a status of 400 or more.
    pub http_errors: u64,
    pub elapsed: Duration,
    /// Response latencies in microseconds, sorted ascending.
    pub latencies_us: Vec<u32>,
}

impl Outcome {
    pub(super) fn errors(&self) -> u64 {
        self.transport_errors.saturating_add(self.http_errors)
    }

    pub(super) fn requests_per_second(&self) -> Option<f64> {
        let seconds = self.elapsed.as_secs_f64();
        (seconds > 0.0).then(|| self.responses as f64 / seconds)
    }

    /// Nearest-rank percentile in milliseconds.
    pub(super) fn latency_ms(&self, percentile: f64) -> Option<f64> {
        nearest_rank(&self.latencies_us, percentile).map(|micros| f64::from(micros) / 1000.0)
    }
}

/// The nearest-rank percentile (`ceil(p/100 × n)`-th smallest) of a sorted slice.
pub(super) fn nearest_rank(sorted: &[u32], percentile: f64) -> Option<u32> {
    if sorted.is_empty() || !(0.0..=100.0).contains(&percentile) {
        return None;
    }
    let rank = (percentile / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.clamp(1, sorted.len()) - 1).copied()
}

/// A client that never uses a proxy or follows redirects, so requests stay
/// on the validated loopback origin.
pub(super) fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .build()
}

#[derive(Default)]
struct Worker {
    responses: u64,
    transport_errors: u64,
    http_errors: u64,
    latencies_us: Vec<u32>,
}

async fn worker(
    client: reqwest::Client,
    url: reqwest::Url,
    deadline: Instant,
    budget: usize,
) -> Worker {
    let mut tally = Worker::default();
    while Instant::now() < deadline {
        let started = Instant::now();
        let result = async {
            let mut response = client.get(url.clone()).send().await?;
            let status = response.status();
            while response.chunk().await?.is_some() {}
            Ok::<_, reqwest::Error>(status)
        }
        .await;
        match result {
            Ok(status) => {
                tally.responses += 1;
                if status.as_u16() >= 400 {
                    tally.http_errors += 1;
                }
                if tally.latencies_us.len() < budget {
                    let micros = started.elapsed().as_micros();
                    tally
                        .latencies_us
                        .push(u32::try_from(micros).unwrap_or(u32::MAX));
                }
            }
            Err(_) => {
                tally.transport_errors += 1;
                // A refused connection fails instantly; do not spin.
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    tally
}

/// Runs the load and waits for in-flight requests to finish.
pub(super) async fn run(
    client: &reqwest::Client,
    url: &reqwest::Url,
    duration: Duration,
    concurrency: u16,
) -> Outcome {
    let workers = usize::from(concurrency.max(1));
    let budget = SAMPLE_BUDGET / workers;
    let started = Instant::now();
    let deadline = started + duration;
    let handles: Vec<_> = (0..workers)
        .map(|_| tokio::spawn(worker(client.clone(), url.clone(), deadline, budget)))
        .collect();
    let mut outcome = Outcome::default();
    for handle in handles {
        let Ok(tally) = handle.await else {
            continue;
        };
        outcome.responses += tally.responses;
        outcome.transport_errors += tally.transport_errors;
        outcome.http_errors += tally.http_errors;
        outcome.latencies_us.extend(tally.latencies_us);
    }
    outcome.elapsed = started.elapsed();
    outcome.latencies_us.sort_unstable();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentiles() {
        let hundred: Vec<u32> = (1..=100).collect();
        assert_eq!(nearest_rank(&hundred, 50.0), Some(50));
        assert_eq!(nearest_rank(&hundred, 95.0), Some(95));
        assert_eq!(nearest_rank(&hundred, 99.0), Some(99));
        assert_eq!(nearest_rank(&hundred, 100.0), Some(100));
        assert_eq!(nearest_rank(&hundred, 0.0), Some(1));
        let ten: Vec<u32> = (1..=10).map(|value| value * 10).collect();
        assert_eq!(nearest_rank(&ten, 95.0), Some(100));
        assert_eq!(nearest_rank(&ten, 50.0), Some(50));
        assert_eq!(nearest_rank(&[7], 99.0), Some(7));
        assert_eq!(nearest_rank(&[], 50.0), None);
        assert_eq!(nearest_rank(&hundred, 101.0), None);
    }

    #[test]
    fn outcome_rates_and_latencies() {
        let outcome = Outcome {
            responses: 500,
            transport_errors: 2,
            http_errors: 3,
            elapsed: Duration::from_secs(2),
            latencies_us: vec![1_000, 2_000, 3_000, 4_000],
        };
        assert_eq!(outcome.errors(), 5);
        assert_eq!(outcome.requests_per_second(), Some(250.0));
        assert_eq!(outcome.latency_ms(50.0), Some(2.0));
        assert_eq!(outcome.latency_ms(99.0), Some(4.0));
        assert_eq!(Outcome::default().requests_per_second(), None);
        assert_eq!(Outcome::default().latency_ms(50.0), None);
    }
}
