//! Live Redis contract of the bounded `list_job_previews` projection.

use super::{live_redis, unique_namespace};
use rullst_core::queue::{QueueDriver, RedisDriver};

#[tokio::test]
async fn redis_previews_cut_payloads_and_errors_inside_the_script() {
    let Some((_container, redis_url)) = live_redis().await else {
        return;
    };
    let driver = RedisDriver::new(redis_url)
        .expect("Redis queue configuration")
        .try_with_namespace(unique_namespace("previews"))
        .expect("isolated queue namespace");

    let large = format!("{{\"blob\":\"{}\"}}", "é".repeat(1_000_000));
    driver
        .push("large", "report", &large)
        .await
        .expect("push large job");
    let claim = driver.pop().await.expect("claim").expect("job");
    driver
        .mark_failed_attempt(&claim.id, claim.attempts, &"ü".repeat(500_000))
        .await
        .expect("record long failure");
    driver
        .push("invalid", "mail", "not-json")
        .await
        .expect("push malformed payload");
    assert!(driver.pop().await.is_err());
    driver
        .push("leased", "mail", r#"{"to":2}"#)
        .await
        .expect("push leased job");
    driver.pop().await.expect("claim").expect("leased job");
    driver
        .push("waiting", "mail", r#"{"to":1}"#)
        .await
        .expect("push pending job");

    let budget = 64;
    let complete = driver.list_all_jobs(10).await.expect("complete listing");
    let previews = driver
        .list_job_previews(10, budget)
        .await
        .expect("bounded listing");
    let listed: Vec<(&str, &str)> = previews
        .iter()
        .map(|job| (job.id.as_str(), job.status.as_str()))
        .collect();
    assert_eq!(
        listed,
        vec![
            ("large", "failed"),
            ("invalid", "dead-letter"),
            ("leased", "processing"),
            ("waiting", "pending"),
        ]
    );
    assert_eq!(previews.len(), complete.len());
    for (preview, detail) in previews.iter().zip(&complete) {
        // The projection agrees with the complete listing, cut to the budget.
        assert_eq!(
            (
                &preview.id,
                &preview.name,
                &preview.status,
                preview.attempts
            ),
            (&detail.id, &detail.name, &detail.status, detail.attempts)
        );
        assert_eq!(preview.updated_at, detail.updated_at);
        assert!(preview.payload.len() <= 64 && detail.payload.starts_with(&preview.payload));
        assert_eq!(preview.payload_truncated, detail.payload.len() > 64);
        match (&preview.error, &detail.error) {
            (Some(head), Some(error)) => {
                assert!(head.len() <= 64 && error.starts_with(head.as_str()));
                assert_eq!(preview.error_truncated, error.len() > 64);
            }
            (None, None) => assert!(!preview.error_truncated),
            _ => panic!("error presence differs for {}", preview.id),
        }
    }
    assert!(previews[0].payload.starts_with("{\"blob\":\"é") && previews[0].payload_truncated);
    assert!(previews[0].error_truncated);
    assert_eq!(previews[1].payload, "not-json");

    let limited = driver
        .list_job_previews(2, budget)
        .await
        .expect("limited listing");
    assert_eq!(limited.len(), 2);
    assert!(
        driver
            .list_job_previews(0, budget)
            .await
            .unwrap()
            .is_empty()
    );
}
