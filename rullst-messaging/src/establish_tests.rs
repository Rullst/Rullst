#![allow(clippy::unwrap_used, clippy::expect_used)]
//! An opening is tried again when its deadline expires, and only then.

use super::{retries, retry};
use std::cell::Cell;

#[derive(Debug, PartialEq)]
enum Failure {
    Expired,
    Refused,
}

fn expired(failure: &Failure) -> bool {
    *failure == Failure::Expired
}

/// Expires until the tenth call, so an unbounded retry would succeed.
async fn expiring(calls: &Cell<u32>) -> Result<u32, Failure> {
    calls.set(calls.get() + 1);
    if calls.get() < 10 {
        Err(Failure::Expired)
    } else {
        Ok(calls.get())
    }
}

#[tokio::test]
async fn an_expired_opening_is_tried_three_times_in_total() {
    let calls = Cell::new(0);
    let before = retries();
    assert_eq!(
        retry(|| expiring(&calls), expired).await,
        Err(Failure::Expired)
    );
    assert_eq!(calls.get(), 3);
    assert_eq!(retries() - before, 2);
}

#[tokio::test]
async fn the_third_attempt_may_succeed() {
    let calls = Cell::new(7);
    assert_eq!(retry(|| expiring(&calls), expired).await, Ok(10));
    assert_eq!(calls.get(), 10);
}

#[tokio::test]
async fn other_failures_and_successes_are_not_retried() {
    let before = retries();
    let calls = Cell::new(0);
    let refused = retry(
        || {
            calls.set(calls.get() + 1);
            std::future::ready(Err::<(), _>(Failure::Refused))
        },
        expired,
    )
    .await;
    assert_eq!(refused, Err(Failure::Refused));
    assert_eq!(calls.get(), 1);
    let opened = retry(
        || {
            calls.set(calls.get() + 1);
            std::future::ready(Ok(5))
        },
        expired,
    )
    .await;
    assert_eq!(opened, Ok(5));
    assert_eq!(calls.get(), 2);
    assert_eq!(retries(), before);
}
