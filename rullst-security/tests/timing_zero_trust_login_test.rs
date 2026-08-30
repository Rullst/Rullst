// tests/timing_zero_trust_login_test.rs — Comprehensive Zero-Trust Fingerprinting, Timing & Login Guard tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_security::login_guard::LoginGuard;
use rullst_security::timing_guard::synthetic_argon2_cpu_work;
use rullst_security::zero_trust::{generate_fingerprint, verify_fingerprint};
use std::time::Duration;

#[test]
fn test_zero_trust_fingerprint_generation_and_verification() {
    let key = b"my_secret_key_zero_trust_12345678";

    let fp1 = generate_fingerprint(
        key,
        Some("Mozilla/5.0"),
        Some("192.168.1.50"),
        Some("en-US"),
    );
    assert!(!fp1.is_empty());

    // Same subnet should produce same fingerprint
    let fp2 = generate_fingerprint(
        key,
        Some("Mozilla/5.0"),
        Some("192.168.1.75"),
        Some("en-US"),
    );
    assert_eq!(fp1, fp2);

    // Verify valid fingerprint
    assert!(verify_fingerprint(
        &fp1,
        key,
        Some("Mozilla/5.0"),
        Some("192.168.1.50"),
        Some("en-US")
    ));

    // Different User-Agent should fail
    assert!(!verify_fingerprint(
        &fp1,
        key,
        Some("Curl/7.68"),
        Some("192.168.1.50"),
        Some("en-US")
    ));
}

#[test]
fn test_login_guard_failures_and_jail() {
    let mut guard = LoginGuard::new();
    guard.max_failures = 3;
    guard.jail_duration = Duration::from_secs(60);

    let ip = "203.0.113.42";
    assert!(!guard.is_jailed(ip));

    // 1st failure (delay = 0s)
    let delay1 = guard.record_login_failure(ip);
    assert!(!guard.is_jailed(ip));
    assert_eq!(delay1, Duration::ZERO);

    // 2nd failure (delay = 1s)
    let delay2 = guard.record_login_failure(ip);
    assert!(!guard.is_jailed(ip));
    assert_eq!(delay2, Duration::from_secs(1));

    // 3rd failure (triggers jail, delay = 5s)
    let delay3 = guard.record_login_failure(ip);
    assert!(guard.is_jailed(ip));
    assert_eq!(delay3, Duration::from_secs(5));
    assert!(guard.remaining_jail_time(ip).is_some());

    // Success clears failure state
    guard.record_login_success(ip);
    assert!(!guard.is_jailed(ip));
}

#[test]
fn test_synthetic_argon2_work() {
    // Synthetic work executes without panicking and creates measurable CPU cycles
    synthetic_argon2_cpu_work();
}
