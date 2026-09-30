#![cfg(feature = "queue-redis")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::queue::{QueueDriver, RedisDriver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use testcontainers::GenericImage;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;

async fn live_redis() -> Option<(testcontainers::ContainerAsync<GenericImage>, String)> {
    let container = match GenericImage::new(
        "redis",
        "7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf",
    )
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .with_exposed_port(6379.tcp())
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
                panic!("Redis testcontainer is required but could not start: {error}");
            }
            eprintln!("skipping Redis queue contract: {error}");
            return None;
        }
    };
    let host = container.get_host().await.expect("Redis host");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("Redis port");
    Some((container, format!("redis://{host}:{port}")))
}

fn unique_namespace(prefix: &str) -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock")
        .as_nanos();
    format!("{prefix}_{unique}")
}

#[tokio::test]
async fn redis_promotes_scheduled_jobs_only_after_server_time_is_due() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("schedule"))
        .expect("isolated queue namespace");

    driver
        .push_at(
            "scheduled",
            "mail",
            r#"{"scheduled":true}"#,
            SystemTime::now() + Duration::from_millis(250),
        )
        .await
        .expect("schedule Redis job");
    driver
        .push("immediate", "mail", r#"{"scheduled":false}"#)
        .await
        .expect("push immediate Redis job");
    assert_eq!(driver.pending_count().await.expect("pending count"), 2);

    let immediate = driver
        .pop()
        .await
        .expect("claim immediate job")
        .expect("immediate job");
    assert_eq!(immediate.id, "immediate");
    driver
        .mark_complete(&immediate.id)
        .await
        .expect("complete immediate job");
    assert!(driver.pop().await.expect("early scheduled poll").is_none());

    tokio::time::sleep(Duration::from_millis(300)).await;
    let scheduled = driver
        .pop()
        .await
        .expect("claim scheduled job")
        .expect("scheduled job after due time");
    assert_eq!(scheduled.id, "scheduled");
    assert_eq!(scheduled.payload["scheduled"], true);
    driver
        .mark_complete(&scheduled.id)
        .await
        .expect("complete scheduled job");
    assert_eq!(driver.pending_count().await.expect("empty queue"), 0);
}

#[tokio::test]
async fn redis_queue_exercises_recovery_failure_requeue_and_rejection_contracts() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let namespace = unique_namespace("lifecycle");
    let driver = RedisDriver::new(redis_url.clone())
        .expect("Redis queue configuration")
        .try_with_namespace(&namespace)
        .expect("isolated queue namespace");

    driver
        .push("recover", "job", r#"{"step":1}"#)
        .await
        .expect("push recoverable job");
    let first = driver.pop().await.expect("claim").expect("job");
    assert_eq!(first.attempts, 1);
    assert_eq!(driver.recover_stalled(Duration::ZERO).await.unwrap(), 1);
    let recovered = driver.pop().await.expect("reclaim").expect("job");
    assert_eq!(recovered.attempts, 2);
    driver
        .requeue(&recovered.id, "retry")
        .await
        .expect("requeue claimed job");
    let retried = driver.pop().await.expect("retry claim").expect("job");
    assert_eq!(retried.attempts, 3);
    driver
        .mark_failed(&retried.id, "terminal")
        .await
        .expect("record terminal failure");

    assert!(driver.mark_complete("missing").await.is_err());
    assert!(driver.mark_failed("missing", "none").await.is_err());
    assert!(driver.requeue("missing", "none").await.is_err());

    driver
        .push("invalid", "job", "not-json")
        .await
        .expect("push malformed payload envelope");
    assert!(driver.pop().await.is_err());

    let client = redis::Client::open(redis_url).expect("Redis client");
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection");
    let queue_key = format!("rullst:queue:{namespace}");
    redis::cmd("RPUSH")
        .arg(&queue_key)
        .arg(r#"{"id":"overflow","name":"job","payload":"{}","attempts":4294967296}"#)
        .query_async::<i64>(&mut connection)
        .await
        .expect("inject overflow envelope");
    assert!(driver.pop().await.is_err());

    driver
        .push("duplicate", "job", "{}")
        .await
        .expect("push first duplicate");
    driver
        .push("duplicate", "job", "{}")
        .await
        .expect("push second duplicate");
    let duplicate = driver.pop().await.expect("first duplicate claim").unwrap();
    assert_eq!(duplicate.id, "duplicate");
    assert!(driver.pop().await.is_err());
    driver
        .mark_complete(&duplicate.id)
        .await
        .expect("complete duplicate lease");
}

#[tokio::test]
async fn redis_stale_claims_cannot_finish_a_reclaimed_job() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("fencing"))
        .expect("isolated queue namespace");

    driver
        .push("fenced", "job", r#"{"step":1}"#)
        .await
        .expect("push fenced job");
    let stale = driver.pop().await.expect("claim").expect("job");
    assert_eq!(driver.recover_stalled(Duration::ZERO).await.unwrap(), 1);
    let current = driver.pop().await.expect("reclaim").expect("job");
    assert_eq!(current.attempts, stale.attempts + 1);

    assert!(
        driver
            .mark_complete_attempt(&stale.id, stale.attempts)
            .await
            .is_err()
    );
    assert!(
        driver
            .mark_failed_attempt(&stale.id, stale.attempts, "stale")
            .await
            .is_err()
    );
    assert!(
        driver
            .requeue_attempt(&stale.id, stale.attempts, "stale")
            .await
            .is_err()
    );
    driver
        .requeue_attempt(&current.id, current.attempts, "current")
        .await
        .expect("requeue the current claim");

    let latest = driver.pop().await.expect("claim").expect("job");
    assert!(
        driver
            .mark_complete_attempt(&latest.id, current.attempts)
            .await
            .is_err()
    );
    driver
        .mark_complete_attempt(&latest.id, latest.attempts)
        .await
        .expect("complete the current claim");
    assert_eq!(driver.pending_count().await.expect("pending count"), 0);
    assert!(driver.pop().await.expect("empty queue").is_none());
}

#[tokio::test]
async fn redis_deferred_claims_wait_for_their_delay_and_are_fenced() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("deferral"))
        .expect("isolated queue namespace");

    driver
        .push("deferred", "export_csv", "{}")
        .await
        .expect("push unhandled job");
    driver
        .push("next", "mail", "{}")
        .await
        .expect("push next job");
    let claim = driver.pop().await.expect("claim").expect("job");
    assert_eq!(claim.id, "deferred");

    let delay = Duration::from_millis(250);
    assert!(
        driver
            .requeue_attempt_after(&claim.id, claim.attempts + 1, "stale", delay)
            .await
            .is_err()
    );
    driver
        .requeue_attempt_after(&claim.id, claim.attempts, "no handler", delay)
        .await
        .expect("defer the current claim");
    assert_eq!(driver.pending_count().await.expect("pending count"), 2);

    let next = driver.pop().await.expect("claim").expect("job");
    assert_eq!(next.id, "next");
    driver
        .mark_complete_attempt(&next.id, next.attempts)
        .await
        .expect("complete next job");
    assert!(driver.pop().await.expect("early poll").is_none());

    tokio::time::sleep(Duration::from_millis(350)).await;
    let reclaimed = driver.pop().await.expect("claim").expect("deferred job");
    assert_eq!(reclaimed.id, "deferred");
    assert_eq!(reclaimed.attempts, claim.attempts + 1);
    driver
        .mark_complete_attempt(&reclaimed.id, reclaimed.attempts)
        .await
        .expect("complete deferred job");
}

#[tokio::test]
async fn redis_failed_jobs_and_dead_letters_are_retained_up_to_the_limit() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let namespace = unique_namespace("retention");
    let driver = RedisDriver::new(redis_url.clone())
        .expect("Redis queue configuration")
        .try_with_namespace(&namespace)
        .expect("isolated queue namespace")
        .try_with_failure_retention(2, 2)
        .expect("bounded retention");

    for index in 0..3 {
        driver
            .push(&format!("failed-{index}"), "job", "{}")
            .await
            .expect("push failing job");
        let claim = driver.pop().await.expect("claim").expect("job");
        driver
            .mark_failed_attempt(&claim.id, claim.attempts, "terminal")
            .await
            .expect("record failure");
    }
    for _ in 0..3 {
        driver
            .push("invalid", "job", "not-json")
            .await
            .expect("push malformed payload");
        assert!(driver.pop().await.is_err());
    }

    let client = redis::Client::open(redis_url).expect("Redis client");
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection");
    let queue_key = format!("rullst:queue:{namespace}");
    let mut failed: Vec<String> = redis::cmd("HKEYS")
        .arg(format!("{queue_key}:failed"))
        .query_async(&mut connection)
        .await
        .expect("failed ids");
    failed.sort();
    assert_eq!(failed, vec!["failed-1".to_string(), "failed-2".to_string()]);
    let indexed: u64 = redis::cmd("ZCARD")
        .arg(format!("{queue_key}:failed:index"))
        .query_async(&mut connection)
        .await
        .expect("failure index size");
    assert_eq!(indexed, 2);
    let dead_letters: u64 = redis::cmd("LLEN")
        .arg(format!("{queue_key}:dead-letter"))
        .query_async(&mut connection)
        .await
        .expect("dead-letter size");
    assert_eq!(dead_letters, 2);
}

#[tokio::test]
async fn redis_failed_jobs_can_be_listed_retried_and_purged() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("operations"))
        .expect("isolated queue namespace");

    driver
        .push("failing", "mail", r#"{"to":"a@example.com"}"#)
        .await
        .expect("push failing job");
    let claim = driver.pop().await.expect("claim").expect("job");
    driver
        .mark_failed_attempt(&claim.id, claim.attempts, "smtp down")
        .await
        .expect("record failure");
    driver
        .push("invalid", "mail", "not-json")
        .await
        .expect("push malformed payload");
    assert!(driver.pop().await.is_err());
    driver
        .push("waiting", "mail", "{}")
        .await
        .expect("push pending job");

    let jobs = driver.list_all_jobs(10).await.expect("list jobs");
    let listed: Vec<(&str, &str)> = jobs
        .iter()
        .map(|job| (job.id.as_str(), job.status.as_str()))
        .collect();
    assert_eq!(
        listed,
        vec![
            ("failing", "failed"),
            ("invalid", "dead-letter"),
            ("waiting", "pending")
        ]
    );
    assert_eq!(jobs[0].error.as_deref(), Some("smtp down"));
    assert_eq!(jobs[0].payload, r#"{"to":"a@example.com"}"#);
    assert!(!jobs[0].updated_at.is_empty());
    assert_eq!(
        driver.list_all_jobs(1).await.expect("bounded list").len(),
        1
    );

    driver
        .retry_failed_job("failing")
        .await
        .expect("retry failed job");
    assert!(driver.retry_failed_job("failing").await.is_err());
    let waiting = driver.pop().await.expect("claim").expect("job");
    assert_eq!(waiting.id, "waiting");
    let retried = driver.pop().await.expect("claim").expect("job");
    assert_eq!(retried.id, "failing");
    assert_eq!(retried.attempts, claim.attempts + 1);
    driver
        .mark_failed_attempt(&retried.id, retried.attempts, "still down")
        .await
        .expect("record second failure");

    driver.purge_failed_jobs().await.expect("purge failures");
    let remaining = driver.list_all_jobs(10).await.expect("list after purge");
    let listed: Vec<(&str, &str)> = remaining
        .iter()
        .map(|job| (job.id.as_str(), job.status.as_str()))
        .collect();
    assert_eq!(listed, vec![("waiting", "processing")]);
}

#[tokio::test]
async fn redis_fails_a_job_whose_lease_keeps_stalling() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("poison"))
        .expect("isolated queue namespace");
    driver
        .push("poison", "resize_image", "{}")
        .await
        .expect("push crashing job");

    for stall in 1..=4 {
        let claim = driver.pop().await.expect("claim").expect("job");
        assert_eq!(claim.attempts, stall);
        assert_eq!(driver.recover_stalled(Duration::ZERO).await.unwrap(), 1);
    }
    let fifth = driver.pop().await.expect("claim").expect("job");
    assert_eq!(fifth.attempts, 5);
    assert_eq!(driver.recover_stalled(Duration::ZERO).await.unwrap(), 1);
    assert!(
        driver.pop().await.expect("empty queue").is_none(),
        "the fifth stalled lease must fail the job instead of requeuing it"
    );
    let jobs = driver.list_all_jobs(10).await.expect("list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "failed");
    assert!(
        jobs[0]
            .error
            .as_deref()
            .unwrap()
            .contains("stalled 5 times")
    );

    // A manual retry restarts the stalled-lease count.
    driver.retry_failed_job("poison").await.expect("retry");
    driver.pop().await.expect("claim").expect("job");
    assert_eq!(driver.recover_stalled(Duration::ZERO).await.unwrap(), 1);
    let requeued = driver.pop().await.expect("claim").expect("requeued job");
    assert_eq!(requeued.id, "poison");
    driver
        .mark_complete_attempt(&requeued.id, requeued.attempts)
        .await
        .expect("complete the requeued claim");
}

#[tokio::test]
async fn redis_configuration_and_connection_failures_are_typed() {
    assert!(RedisDriver::new("not a redis URL").is_err());
    let driver = RedisDriver::new("redis://127.0.0.1:1")
        .expect("syntactically valid Redis URL")
        .try_with_namespace("connection_failure")
        .expect("valid namespace");
    assert!(driver.pending_count().await.is_err());
    assert!(driver.push("id", "job", "{}").await.is_err());
    assert!(
        driver
            .push_at("id", "job", "{}", SystemTime::now())
            .await
            .is_err()
    );
    assert!(driver.pop().await.is_err());
    assert!(driver.recover_stalled(Duration::ZERO).await.is_err());
}
