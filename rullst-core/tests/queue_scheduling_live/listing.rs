//! Live Redis contract of the bounded `list_all_jobs` window, the legacy
//! failure purge and the retry of a corrupt failure record.

use super::{live_redis, unique_namespace};
use rullst_core::queue::{QueueDriver, QueueError, RedisDriver};

async fn connection(redis_url: &str) -> redis::aio::MultiplexedConnection {
    redis::Client::open(redis_url)
        .expect("Redis client")
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection")
}

#[tokio::test]
async fn redis_listing_reads_at_most_the_limit_from_each_structure() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let namespace = unique_namespace("listing");
    let driver = RedisDriver::new(redis_url.clone())
        .expect("Redis queue configuration")
        .try_with_namespace(&namespace)
        .expect("isolated queue namespace");

    // Four rejected envelopes become dead letters, the newest last.
    for index in 0..4 {
        driver
            .push(&format!("dead-{index}"), "job", "not-json")
            .await
            .expect("push malformed job");
        assert!(driver.pop().await.is_err());
    }
    driver
        .push("pending", "job", "{}")
        .await
        .expect("push pending job");
    let listed = driver.list_all_jobs(3).await.expect("listing");
    let rows: Vec<(&str, &str)> = listed
        .iter()
        .map(|job| (job.status.as_str(), job.id.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("dead-letter", "dead-3"),
            ("dead-letter", "dead-2"),
            ("dead-letter", "dead-1")
        ]
    );

    // The legacy trait hook purges failures and dead letters.
    QueueDriver::purge_completed_jobs(&driver)
        .await
        .expect("purge failures");
    let listed = driver.list_all_jobs(10).await.expect("listing");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "pending");

    // A failure index entry without its record still uses its slot.
    let claim = driver.pop().await.expect("claim").expect("job");
    driver
        .mark_failed(&claim.id, "terminal")
        .await
        .expect("record failure");
    driver
        .push("waiting", "job", "{}")
        .await
        .expect("push another pending job");
    let mut redis = connection(&redis_url).await;
    redis::cmd("ZADD")
        .arg(format!("rullst:queue:{namespace}:failed:index"))
        .arg(9_000_000_000_000_000_i64)
        .arg("ghost")
        .query_async::<i64>(&mut redis)
        .await
        .expect("inject index entry");
    let listed = driver.list_all_jobs(1).await.expect("listing");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "pending");
    assert_eq!(listed[0].id, "waiting");
}

#[tokio::test]
async fn redis_retry_reports_a_corrupt_failure_record() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let namespace = unique_namespace("corrupt");
    let driver = RedisDriver::new(redis_url.clone())
        .expect("Redis queue configuration")
        .try_with_namespace(&namespace)
        .expect("isolated queue namespace");
    let mut redis = connection(&redis_url).await;
    redis::cmd("HSET")
        .arg(format!("rullst:queue:{namespace}:failed"))
        .arg("corrupt")
        .arg("not a failure record")
        .query_async::<i64>(&mut redis)
        .await
        .expect("inject corrupt record");

    match driver.retry_failed_job("corrupt").await {
        Err(QueueError::StateTransition { message, .. }) => {
            assert!(message.contains("not a valid failure record"), "{message}");
        }
        other => panic!("expected a state transition error, got {other:?}"),
    }
}
