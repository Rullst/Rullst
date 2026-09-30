//! Unit tests for the Redis queue driver that need no Redis server.

use super::*;

#[test]
fn claimed_envelope_requires_valid_json_payload() {
    let result = parse_claimed_job(r#"{"id":"job-1","name":"test","payload":"{bad","attempts":1}"#);
    assert!(matches!(result, Err(QueueError::Serialization(_))));
}

#[test]
fn claimed_envelope_is_strict_and_lossless() {
    let job =
        parse_claimed_job(r#"{"id":"job-1","name":"test","payload":"{\"ok\":true}","attempts":2}"#)
            .unwrap();
    assert_eq!(job.id, "job-1");
    assert_eq!(job.payload["ok"], true);
    assert_eq!(job.attempts, 2);
}

#[test]
fn failure_retention_is_bounded_and_defaults_to_ten_thousand() {
    let driver = RedisDriver::new("redis://127.0.0.1/").unwrap();
    assert_eq!(driver.failed_retention, DEFAULT_FAILURE_RETENTION);
    assert_eq!(driver.dead_letter_retention, DEFAULT_FAILURE_RETENTION);
    assert_eq!(DEFAULT_FAILURE_RETENTION, 10_000);

    let driver = driver
        .try_with_namespace("tenant")
        .unwrap()
        .try_with_failure_retention(2, MAX_FAILURE_RETENTION)
        .unwrap();
    assert_eq!(driver.failed_retention, 2);
    assert_eq!(driver.dead_letter_retention, MAX_FAILURE_RETENTION);
    assert_eq!(driver.failed_index_key, "rullst:queue:tenant:failed:index");

    for (failed, dead) in [(0, 1), (1, 0), (MAX_FAILURE_RETENTION + 1, 1)] {
        let result = RedisDriver::new("redis://127.0.0.1/")
            .unwrap()
            .try_with_failure_retention(failed, dead);
        assert!(matches!(result, Err(QueueError::InvalidConfiguration(_))));
    }
}

#[test]
fn namespace_is_bounded_and_syntax_checked() {
    let valid = RedisDriver::new("redis://127.0.0.1/")
        .unwrap()
        .try_with_namespace("tenant_42-prod");
    assert!(valid.is_ok());

    for invalid in ["", "../shared", "contains space"] {
        let result = RedisDriver::new("redis://127.0.0.1/")
            .unwrap()
            .try_with_namespace(invalid);
        assert!(matches!(result, Err(QueueError::InvalidConfiguration(_))));
    }
}
