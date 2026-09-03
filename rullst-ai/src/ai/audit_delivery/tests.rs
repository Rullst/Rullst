use super::*;

#[test]
fn retry_policy_bounds_and_backoff_are_exact() {
    assert!(AuditRetryPolicy::try_new(0, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(6, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(1, Duration::from_secs(6)).is_err());

    let policy = AuditRetryPolicy::try_new(5, Duration::from_secs(1)).expect("valid policy");
    assert_eq!(policy.delay_for(1), Duration::from_secs(1));
    assert_eq!(policy.delay_for(2), Duration::from_secs(2));
    assert_eq!(policy.delay_for(3), Duration::from_secs(4));
    assert_eq!(policy.delay_for(4), Duration::from_secs(5));
    assert_eq!(policy.delay_for(u8::MAX), Duration::from_secs(5));

    let default = AuditRetryPolicy::default();
    assert_eq!(default.max_attempts, 3);
    assert_eq!(default.base_delay, Duration::from_millis(200));
}

#[test]
fn retry_classification_is_narrow() {
    assert!(retryable_delivery(&AuditDeliveryError::Transport));
    assert!(retryable_delivery(&AuditDeliveryError::Deadline));
    assert!(retryable_delivery(&AuditDeliveryError::Rejected {
        status: 429
    }));
    assert!(retryable_delivery(&AuditDeliveryError::Rejected {
        status: 500
    }));
    assert!(!retryable_delivery(&AuditDeliveryError::Rejected {
        status: 400
    }));
    assert!(!retryable_delivery(&AuditDeliveryError::InvalidAck));
    assert!(!retryable_delivery(&AuditDeliveryError::Cancelled));
}

#[test]
fn hexadecimal_encoding_is_lowercase_and_empty_safe() {
    assert_eq!(hex(&[]), "");
    assert_eq!(hex(&[0x00, 0x0f, 0x10, 0xab, 0xff]), "000f10abff");
}
