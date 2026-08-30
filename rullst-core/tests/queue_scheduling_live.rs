#![cfg(feature = "queue-redis")]
#![allow(clippy::expect_used)]

use rullst_core::queue::{QueueDriver, RedisDriver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn redis_promotes_scheduled_jobs_only_after_server_time_is_due() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("skipping Redis queue scheduling contract: RULLST_TEST_REDIS_URL is unset");
        return;
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock")
        .as_nanos();
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(format!("schedule_{unique}"))
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
