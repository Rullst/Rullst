//! Start-up validation and queue probes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::endpoint::{QUEUE_PROBE_TIMEOUT, queue_reports};
use super::payload::Pending;
use super::test_support::BASE64_TOKEN;
use super::{Installed, Monitor, MonitorError};
use crate::config::Environment;
use crate::queue::{Queue, QueueDriver, QueueError, QueuedJob};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn prepare(monitor: Monitor) -> Result<bool, MonitorError> {
    Installed::prepare(monitor.with_token(BASE64_TOKEN), Environment::Production)
        .map(|installed| installed.is_some())
}

#[test]
fn the_application_name_and_version_are_bounded_labels() {
    let with = |name: &str, version: &str| prepare(Monitor::new().with_application(name, version));
    assert_eq!(with("shop", "1.4.0+abc123"), Ok(true));
    assert_eq!(with(&"n".repeat(128), &"v".repeat(128)), Ok(true));
    for (name, version) in [
        ("", "1.0.0"),
        ("shop", ""),
        (&"n".repeat(129) as &str, "1.0.0"),
        ("shop", &"v".repeat(129) as &str),
        ("shop\n", "1.0.0"),
        ("shop", "1.0\u{7}"),
    ] {
        assert_eq!(
            with(name, version),
            Err(MonitorError::InvalidApplication),
            "{name:?} {version:?}"
        );
    }
}

struct Driver {
    pending: Result<u64, ()>,
    delay: Duration,
}

#[async_trait]
impl QueueDriver for Driver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        tokio::time::sleep(self.delay).await;
        self.pending
            .map_err(|()| QueueError::Driver("redis://admin:secret@10.0.0.5 refused".to_string()))
    }
}

fn queue(pending: Result<u64, ()>, delay: Duration) -> Arc<Queue> {
    Arc::new(Queue::custom(Box::new(Driver { pending, delay })))
}

#[test]
fn queue_names_are_bounded_unique_labels_and_at_most_sixteen() {
    let one = |name: &str| prepare(Monitor::new().with_queue(name, queue(Ok(0), Duration::ZERO)));
    for name in [
        "default",
        "mail.v2",
        "high_priority",
        "a-b",
        &"q".repeat(64),
    ] {
        assert_eq!(one(name), Ok(true), "{name}");
    }
    for name in ["", "with space", "slash/name", "ação", &"q".repeat(65)] {
        assert_eq!(one(name), Err(MonitorError::InvalidQueueName), "{name}");
    }
    let duplicate = Monitor::new()
        .with_queue("mail", queue(Ok(0), Duration::ZERO))
        .with_queue("other", queue(Ok(0), Duration::ZERO))
        .with_queue("mail", queue(Ok(0), Duration::ZERO));
    assert_eq!(prepare(duplicate), Err(MonitorError::DuplicateQueue));

    let many = |count: usize| {
        (0..count).fold(Monitor::new(), |monitor, index| {
            monitor.with_queue(format!("q{index}"), queue(Ok(0), Duration::ZERO))
        })
    };
    assert_eq!(prepare(many(16)), Ok(true));
    assert_eq!(
        prepare(many(17)),
        Err(MonitorError::TooManyQueues { maximum: 16 })
    );
}

#[test]
fn configuration_errors_are_reported_even_without_credentials() {
    let monitor = Monitor::new().with_application("", "1");
    assert_eq!(
        Installed::prepare(monitor, Environment::Production).err(),
        Some(MonitorError::InvalidApplication)
    );
}

#[tokio::test]
async fn queue_probes_report_counts_and_fixed_reasons_within_the_limit() {
    let queues = vec![
        ("mail".to_string(), queue(Ok(7), Duration::ZERO)),
        ("slow".to_string(), queue(Ok(1), Duration::from_secs(30))),
        ("broken".to_string(), queue(Err(()), Duration::ZERO)),
    ];
    queues[0].1.activity().completed();
    let started = Instant::now();
    let reports = queue_reports(&queues).await;
    // The probes run concurrently, so the slow store delays only itself.
    assert!(started.elapsed() < QUEUE_PROBE_TIMEOUT * 8);

    let names: Vec<&str> = reports.iter().map(|report| report.name.as_str()).collect();
    assert_eq!(names, ["mail", "slow", "broken"]);
    assert_eq!(reports[0].pending, Pending::Observed { count: 7 });
    assert_eq!(reports[0].completed_total, 1);
    assert_eq!(
        reports[1].pending,
        Pending::Unavailable { reason: "timeout" }
    );
    assert_eq!(
        reports[2].pending,
        Pending::Unavailable {
            reason: "driver_error"
        }
    );
    let json = serde_json::to_string(&reports[2]).unwrap();
    assert!(!json.contains("secret"));
    assert!(!json.contains("10.0.0.5"));
}

#[test]
fn errors_explain_themselves_without_values() {
    assert_eq!(
        MonitorError::TooManyTokens { maximum: 8 }.to_string(),
        "at most 8 monitor tokens may be configured"
    );
    assert_eq!(
        MonitorError::PlaceholderToken { position: 3 }.to_string(),
        "monitor token 3 is not random enough or is a placeholder"
    );
    assert_eq!(
        MonitorError::InvalidToken { position: 1 }.to_string(),
        "monitor token 1 must contain only base64, base64url or hexadecimal characters, at most 512 of them"
    );
}
