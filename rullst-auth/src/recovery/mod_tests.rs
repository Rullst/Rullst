//! Shared timestamp, email and system-clock helpers.
use super::*;

#[test]
fn timestamps_leave_a_31_day_margin_below_the_largest_i64() {
    assert_eq!(timestamp(0), Ok(0));
    assert_eq!(
        timestamp(9_223_372_036_852_097_407),
        Ok(9_223_372_036_852_097_407)
    );
    assert_eq!(
        timestamp(9_223_372_036_852_097_408),
        Err(RecoveryError::InvalidInput)
    );
    assert_eq!(timestamp(u64::MAX), Err(RecoveryError::InvalidInput));
}

#[test]
fn emails_are_trimmed_lowercased_bounded_and_need_both_parts() {
    assert_eq!(
        normalized_email("  Learner@Example.COM "),
        Ok("learner@example.com".to_owned())
    );
    let longest = format!("{}@example.com", "a".repeat(242));
    assert_eq!(longest.len(), 254);
    assert_eq!(normalized_email(&longest), Ok(longest.clone()));
    for bad in [
        format!("a{longest}"),
        "a b@example.com".to_owned(),
        "a\u{1}b@example.com".to_owned(),
        "a\tb@example.com".to_owned(),
        "@example.com".to_owned(),
        "learner@".to_owned(),
        "learner@a@example.com".to_owned(),
        "learner@localhost".to_owned(),
        "learner.example.com".to_owned(),
        "álvaro@example.com".to_owned(),
    ] {
        assert_eq!(
            normalized_email(&bad),
            Err(RecoveryError::InvalidInput),
            "{bad:?}"
        );
    }
}

#[test]
fn the_system_clock_reports_the_current_unix_time() {
    let now = SystemAuthClock.now().unwrap();
    let reference = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        now.abs_diff(reference) <= 5,
        "clock is off by {}",
        now.abs_diff(reference)
    );
}
