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
