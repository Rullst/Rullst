//! The local history: retention, alerts and stored columns
//! (`TM-STUDIO-3`, `REMOTE-08`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::client::FetchError;
use super::document::MonitorDocument;
use super::history::{History, Retention, Sample};
use super::server_test::{TempDir, document};

fn sample(at: u64) -> Sample {
    Sample {
        at,
        reachable: true,
        requests_per_second: Some(1.5),
        window_requests: Some(30),
        error_rate_percent: Some(2.0),
        p95_ms: Some(25.0),
        queue_pending: Some(3),
        queue_failed_total: Some(4),
    }
}

async fn open(directory: &TempDir, retention: Retention) -> History {
    History::open(&directory.paths().history, retention)
        .await
        .unwrap()
}

#[tokio::test]
async fn samples_are_kept_in_order_and_bounded_by_age_and_count() {
    let directory = TempDir::new("history");
    let history = open(
        &directory,
        Retention {
            max_age_seconds: 1_000,
            max_samples_per_project: 3,
            max_alerts: 10,
        },
    )
    .await;
    for at in [10_000, 10_060, 10_120] {
        history.record("shop", &sample(at)).await.unwrap();
    }
    history.record("blog", &sample(10_000)).await.unwrap();
    let ats = |samples: Vec<Sample>| samples.iter().map(|sample| sample.at).collect::<Vec<_>>();
    assert_eq!(
        ats(history.samples("shop", 0).await.unwrap()),
        [10_000, 10_060, 10_120]
    );
    assert_eq!(history.samples("shop", 0).await.unwrap()[2], sample(10_120));

    // A fourth sample pushes the oldest out (count bound).
    history.record("shop", &sample(10_180)).await.unwrap();
    assert_eq!(
        ats(history.samples("shop", 0).await.unwrap()),
        [10_060, 10_120, 10_180]
    );
    assert_eq!(
        ats(history.samples("shop", 10_100).await.unwrap()),
        [10_120, 10_180]
    );
    assert_eq!(
        history
            .previous("shop", 10_180)
            .await
            .unwrap()
            .map(|sample| sample.at),
        Some(10_120)
    );

    // A sample 1,000 s later removes everything older than that (age bound).
    history.record("shop", &sample(11_150)).await.unwrap();
    assert_eq!(
        ats(history.samples("shop", 0).await.unwrap()),
        [10_180, 11_150]
    );
    assert!(history.samples("blog", 0).await.unwrap().is_empty());

    history.forget("shop").await.unwrap();
    assert!(history.samples("shop", 0).await.unwrap().is_empty());
}

#[tokio::test]
async fn alerts_are_bounded_counted_and_acknowledged() {
    let directory = TempDir::new("history-alerts");
    let history = open(
        &directory,
        Retention {
            max_alerts: 2,
            ..Retention::default()
        },
    )
    .await;
    for (at, rule) in [
        (100, "p95_latency"),
        (200, "error_rate"),
        (300, "p95_latency"),
    ] {
        history
            .record_alert("shop", at, rule, "shop: p95", "no webhook configured")
            .await
            .unwrap();
    }
    let alerts = history.alerts(10).await.unwrap();
    assert_eq!(
        alerts.iter().map(|alert| alert.at).collect::<Vec<_>>(),
        [300, 200]
    );
    assert_eq!(history.unacknowledged().await.unwrap(), 2);
    let last = history.last_fired("shop").await.unwrap();
    assert_eq!(last.get("p95_latency"), Some(&300));
    assert_eq!(last.get("error_rate"), Some(&200));

    history
        .set_delivery(alerts[0].id, "delivered")
        .await
        .unwrap();
    assert_eq!(history.alerts(1).await.unwrap()[0].delivery, "delivered");
    history.acknowledge_all().await.unwrap();
    assert_eq!(history.unacknowledged().await.unwrap(), 0);
    assert!(
        history
            .alerts(10)
            .await
            .unwrap()
            .iter()
            .all(|alert| alert.acknowledged)
    );
}

#[tokio::test]
async fn only_numeric_aggregates_are_stored() {
    let directory = TempDir::new("history-columns");
    let history = open(&directory, Retention::default()).await;
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('samples')")
        .fetch_all(history_pool(&history))
        .await
        .unwrap();
    assert_eq!(
        columns,
        [
            "project",
            "at",
            "reachable",
            "requests_per_second",
            "window_requests",
            "error_rate_percent",
            "p95_ms",
            "queue_pending",
            "queue_failed_total"
        ]
    );
}

fn history_pool(history: &History) -> &sqlx::SqlitePool {
    history.pool_for_tests()
}

#[test]
fn a_sample_takes_the_documents_aggregates_or_marks_the_project_unreachable() {
    let parsed: MonitorDocument = serde_json::from_value(document("/orders/{id}")).unwrap();
    let sample = Sample::from_outcome(77, &Ok(parsed));
    assert_eq!(
        sample,
        Sample {
            at: 77,
            reachable: true,
            requests_per_second: Some(0.333),
            window_requests: Some(40),
            error_rate_percent: Some(5.0),
            p95_ms: Some(25.0),
            queue_pending: Some(3),
            queue_failed_total: Some(4),
        }
    );
    let mut idle = document("/orders/{id}");
    idle["http"]["window"]["requests"] = serde_json::json!(0);
    idle["http"]["window"]["status"]["5xx"] = serde_json::json!(0);
    let idle: MonitorDocument = serde_json::from_value(idle).unwrap();
    // No request in the window: no rate rather than 0/0.
    assert_eq!(Sample::from_outcome(79, &Ok(idle)).error_rate_percent, None);
    let down = Sample::from_outcome(78, &Err(FetchError::Timeout));
    assert!(!down.reachable);
    assert_eq!(down.requests_per_second, None);
    assert_eq!(down.queue_failed_total, None);
}

#[cfg(unix)]
#[tokio::test]
async fn the_database_is_private_to_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let directory = TempDir::new("history-mode");
    let _history = open(&directory, Retention::default()).await;
    let path = directory.paths().history;
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
}
