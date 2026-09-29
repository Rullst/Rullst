//! Regression tests for bounded failure/jail admission at capacity.

use super::tests::past;
use super::*;

#[test]
fn full_failure_index_evicts_the_oldest_counter_so_a_new_identity_is_jailed() {
    let guard = LoginGuard {
        max_identities: 3,
        max_failures: 3,
        ..LoginGuard::default()
    };
    for index in 0..3 {
        guard.insert_failure(
            &format!("filler-{index}"),
            1,
            past(Duration::from_secs(10 - index)),
        );
    }
    assert_eq!(guard.failure_count(), 3);

    assert_eq!(guard.record_login_failure("victim"), Duration::ZERO);
    assert_eq!(guard.record_login_failure("victim"), Duration::from_secs(1));
    assert_eq!(guard.record_login_failure("victim"), JAILED_DELAY);
    assert!(guard.is_jailed("victim"));
    assert!(guard.failure_count() <= 3);

    // The least recently failed filler was evicted; the newest survived.
    let state = guard.state.lock().unwrap();
    assert!(!state.failures.contains(&identity_key("filler-0")));
    assert!(state.failures.contains(&identity_key("filler-2")));
    assert!(!state.failures.contains(&identity_key("victim")));
}

#[test]
fn full_jail_index_evicts_the_soonest_expiring_jail_and_jails_the_new_offender() {
    let guard = LoginGuard {
        max_identities: 2,
        max_failures: 2,
        ..LoginGuard::default()
    };
    guard.insert_jail("soonest", Instant::now() + Duration::from_secs(30));
    guard.insert_jail("latest", Instant::now() + Duration::from_secs(600));

    assert_eq!(guard.record_login_failure("offender"), Duration::ZERO);
    assert_eq!(guard.record_login_failure("offender"), JAILED_DELAY);

    assert!(guard.is_jailed("offender"));
    assert!(guard.is_jailed("latest"));
    assert!(!guard.is_jailed("soonest"));
    assert_eq!(guard.jail_count(), 2);
    // The counter is cleared only because the jail now exists.
    assert_eq!(guard.failure_count(), 0);
}

#[test]
fn identity_flood_cannot_disable_the_jail_for_a_later_identity() {
    let guard = LoginGuard {
        max_identities: 64,
        ..LoginGuard::default()
    };
    for index in 0..1_000 {
        guard.record_login_failure(&format!("random-user-{index}"));
    }
    assert_eq!(guard.failure_count(), 64);
    for _ in 0..guard.max_failures {
        guard.record_login_failure("real-account");
    }
    assert!(guard.is_jailed("real-account"));
    assert!(guard.failure_count() <= 64);
    assert!(guard.jail_count() <= 64);
}

#[test]
fn expired_counters_and_jails_are_pruned_on_the_next_operation() {
    let guard = LoginGuard {
        window_duration: Duration::from_secs(1),
        ..LoginGuard::default()
    };
    guard.insert_failure("stale-failure", 3, past(Duration::from_secs(5)));
    guard.insert_failure("fresh-failure", 1, Instant::now());
    guard.insert_jail("stale-jail", past(Duration::from_secs(1)));

    assert!(!guard.is_jailed("unrelated"));
    assert_eq!(guard.failure_count(), 1);
    assert_eq!(guard.jail_count(), 0);
}

#[test]
fn zero_jail_duration_keeps_counting_without_creating_a_jail() {
    let guard = LoginGuard {
        max_failures: 1,
        jail_duration: Duration::ZERO,
        ..LoginGuard::default()
    };
    assert_eq!(guard.record_login_failure("no-jail"), JAILED_DELAY);
    assert!(!guard.is_jailed("no-jail"));
    assert_eq!(guard.jail_count(), 0);
    assert_eq!(guard.failure_count(), 1);
}

#[test]
fn unrepresentable_jail_duration_saturates_instead_of_panicking() {
    let guard = LoginGuard {
        max_failures: 1,
        jail_duration: Duration::MAX,
        ..LoginGuard::default()
    };
    assert_eq!(guard.record_login_failure("forever"), JAILED_DELAY);
    assert!(guard.is_jailed("forever"));
}
