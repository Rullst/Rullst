#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::window::to_millis;
use super::*;
use crate::queue::{QueueDriver, SqliteDriver};
use std::time::{Duration, SystemTime};

/// A shared in-memory database, a queue over it, a driver that claims from
/// it and a pool that inspects its rows.
async fn shared() -> (String, sqlx::SqlitePool) {
    let name = format!("rullst_deferral_{}", uuid::Uuid::new_v4().simple());
    let url = format!("sqlite:file:{name}?mode=memory&cache=shared");
    let keeper = sqlx::SqlitePool::connect(&url).await.unwrap();
    (url, keeper)
}

fn utc_hour(now: SystemTime) -> u8 {
    let hour = to_millis(now).rem_euclid(86_400_000) / 3_600_000;
    u8::try_from(hour).unwrap()
}

/// A one-hour window that opened at most an hour ago.
fn open_window(now: SystemTime) -> TimeWindow {
    let hour = utc_hour(now);
    TimeWindow::daily(hour, 0, hour + 1, 0).unwrap()
}

/// A one-hour window that opens between one and two hours from now.
fn closed_window(now: SystemTime) -> TimeWindow {
    let start = (utc_hour(now) + 2) % 24;
    TimeWindow::daily(start, 0, start + 1, 0).unwrap()
}

async fn available_at_ms(pool: &sqlx::SqlitePool, id: &str) -> i64 {
    sqlx::query_scalar("SELECT available_at_ms FROM rullst_jobs WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn deferred_jobs_wait_for_their_window_and_open_windows_run_now() {
    let (url, pool) = shared().await;
    let queue = Queue::sqlite(url.clone()).await.unwrap();
    let driver = SqliteDriver::new(url).await.unwrap();
    let now = SystemTime::now();
    let tomorrow = now + Duration::from_secs(36 * 3_600);

    let waiting = Deferral::until(tomorrow)
        .window(closed_window(now))
        .unwrap();
    let deferred = queue
        .dispatch_deferred("report", serde_json::json!({"kind": "monthly"}), &waiting)
        .await
        .unwrap();
    assert_eq!(deferred.plan.reason, DeferralReason::Window);
    assert!(deferred.plan.run_at > now);
    assert_eq!(
        available_at_ms(&pool, &deferred.id).await,
        to_millis(deferred.plan.run_at)
    );

    let open = Deferral::until(tomorrow).window(open_window(now)).unwrap();
    let ready = queue
        .dispatch_deferred("report", serde_json::json!({"kind": "daily"}), &open)
        .await
        .unwrap();
    assert_eq!(ready.plan.reason, DeferralReason::Window);

    assert_eq!(driver.pop().await.unwrap().unwrap().id, ready.id);
    assert!(driver.pop().await.unwrap().is_none());
    assert_eq!(queue.pending_count().await.unwrap(), 1);
}

#[tokio::test]
async fn a_deferred_job_becomes_claimable_at_its_deadline() {
    let (url, _pool) = shared().await;
    let queue = Queue::sqlite(url.clone()).await.unwrap();
    let driver = SqliteDriver::new(url).await.unwrap();
    let now = SystemTime::now();
    let deferral = Deferral::until(now + Duration::from_millis(150))
        .window(closed_window(now))
        .unwrap();
    let job = queue
        .dispatch_deferred("export", serde_json::json!({}), &deferral)
        .await
        .unwrap();
    assert_eq!(job.plan.reason, DeferralReason::Deadline);
    assert!(job.plan.run_at <= deferral.run_by());

    assert!(driver.pop().await.unwrap().is_none());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(driver.pop().await.unwrap().unwrap().id, job.id);
}

#[tokio::test]
async fn the_planner_stores_the_lowest_intensity_slot() {
    let (url, pool) = shared().await;
    let queue = Queue::sqlite(url).await.unwrap();
    let now = SystemTime::now();
    let low = now + Duration::from_secs(1_800);
    let source = FixedIntensitySource::new("fixed-test", "gCO2eq/kWh")
        .slot(now, low, 400.0)
        .slot(low, now + Duration::from_secs(3_600), 90.0);
    let planner = CarbonAwarePlanner::new(source);
    let deferral = Deferral::until(now + Duration::from_secs(7_200));
    let job = queue
        .dispatch_deferred_with(&planner, "reindex", serde_json::json!({}), &deferral)
        .await
        .unwrap();
    assert_eq!(job.plan.reason, DeferralReason::Intensity);
    assert_eq!(job.plan.source.as_deref(), Some("fixed-test"));
    assert_eq!(to_millis(job.plan.run_at), to_millis(low));
    assert_eq!(available_at_ms(&pool, &job.id).await, to_millis(low));
}

#[tokio::test]
async fn invalid_names_and_distant_deadlines_are_rejected_before_planning() {
    let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
    let far = Deferral::until(SystemTime::now() + Duration::from_secs(400 * 24 * 3_600));
    assert!(matches!(
        queue
            .dispatch_deferred("report", serde_json::json!({}), &far)
            .await,
        Err(QueueError::InvalidConfiguration(_))
    ));
    let soon = Deferral::until(SystemTime::now() + Duration::from_secs(60));
    assert!(matches!(
        queue
            .dispatch_deferred("", serde_json::json!({}), &soon)
            .await,
        Err(QueueError::InvalidConfiguration(_))
    ));
    assert_eq!(queue.pending_count().await.unwrap(), 0);
}

/// Rows written by a 12.x process (its table had no lease column) keep their
/// behaviour next to deferred jobs, and non-deferred dispatch is unchanged.
#[tokio::test]
async fn rows_from_a_12x_table_and_plain_jobs_are_unaffected() {
    let (url, pool) = shared().await;
    sqlx::query(
        r#"CREATE TABLE rullst_jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            error TEXT,
            attempts INTEGER NOT NULL DEFAULT 0,
            available_at_ms INTEGER NOT NULL DEFAULT 0,
            stalled_recoveries INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES \
         ('legacy-immediate', 'mail', '{\"v\":12}', datetime('now', '-2 minutes'))",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO rullst_jobs (id, name, payload, available_at_ms, created_at) VALUES \
         ('legacy-scheduled', 'mail', '{}', 1, datetime('now', '-1 minutes'))",
    )
    .execute(&pool)
    .await
    .unwrap();

    let queue = Queue::sqlite(url.clone()).await.unwrap();
    let driver = SqliteDriver::new(url).await.unwrap();
    let now = SystemTime::now();
    let deferred = queue
        .dispatch_deferred(
            "report",
            serde_json::json!({}),
            &Deferral::until(now + Duration::from_secs(36 * 3_600))
                .window(closed_window(now))
                .unwrap(),
        )
        .await
        .unwrap();
    let plain = queue.dispatch("mail", serde_json::json!({})).await.unwrap();
    assert!(available_at_ms(&pool, &plain).await <= to_millis(SystemTime::now()));

    let first = driver.pop().await.unwrap().unwrap();
    assert_eq!((first.id.as_str(), first.attempts), ("legacy-immediate", 1));
    assert_eq!(first.payload["v"], 12);
    assert_eq!(driver.pop().await.unwrap().unwrap().id, "legacy-scheduled");
    assert_eq!(driver.pop().await.unwrap().unwrap().id, plain);
    assert!(driver.pop().await.unwrap().is_none());
    assert_eq!(
        available_at_ms(&pool, &deferred.id).await,
        to_millis(deferred.plan.run_at)
    );
}
