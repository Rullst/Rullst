#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::watch;

/// A check whose answer and change signal the test controls.
struct Controlled {
    valid: Arc<AtomicBool>,
    checks: Arc<AtomicUsize>,
    signal: watch::Receiver<u32>,
}

impl Revalidate for Controlled {
    async fn still_valid(&mut self) -> bool {
        self.checks.fetch_add(1, Ordering::SeqCst);
        self.valid.load(Ordering::SeqCst)
    }

    async fn changed(&mut self) {
        if self.signal.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

fn controlled() -> (
    Controlled,
    Arc<AtomicBool>,
    Arc<AtomicUsize>,
    watch::Sender<u32>,
) {
    let valid = Arc::new(AtomicBool::new(true));
    let checks = Arc::new(AtomicUsize::new(0));
    let (sender, signal) = watch::channel(0);
    (
        Controlled {
            valid: Arc::clone(&valid),
            checks: Arc::clone(&checks),
            signal,
        },
        valid,
        checks,
        sender,
    )
}

#[test]
fn the_documented_bounds_hold() {
    assert_eq!(DEFAULT_REVALIDATION_INTERVAL, Duration::from_secs(15));
    assert_eq!(REVALIDATION_DEADLINE, Duration::from_secs(5));
    assert_eq!(clamp_interval(Duration::ZERO), Duration::from_secs(1));
    assert_eq!(
        clamp_interval(Duration::from_secs(15)),
        Duration::from_secs(15)
    );
    assert_eq!(
        clamp_interval(Duration::from_secs(600)),
        Duration::from_secs(300)
    );
}

#[tokio::test]
async fn a_check_answers_through_its_closure_or_the_unchecked_default() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let mut check = revalidate_with(move || {
        let counted = Arc::clone(&counted);
        async move { counted.fetch_add(1, Ordering::SeqCst) == 0 }
    });
    assert!(check.still_valid().await);
    assert!(!check.still_valid().await);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    // A closure check has no change signal.
    assert!(
        tokio::time::timeout(Duration::from_millis(50), check.changed())
            .await
            .is_err()
    );
    assert!(NoRevalidation.still_valid().await);
}

#[tokio::test]
async fn a_check_is_due_after_its_interval_or_at_once_on_a_signal() {
    let (check, _, _, signal) = controlled();
    let mut revalidator = Revalidator::new(check, Duration::from_secs(1));
    assert!(!revalidator.due_now());
    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(3), revalidator.due())
        .await
        .unwrap();
    let elapsed = started.elapsed();
    assert!(elapsed >= Duration::from_millis(900), "{elapsed:?}");
    // The next interval starts again; a signal does not wait for it.
    assert!(!revalidator.due_now());
    signal.send_replace(1);
    assert!(revalidator.due_now());
    signal.send_replace(2);
    tokio::time::timeout(Duration::from_millis(200), revalidator.due())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_check_answers_false_when_invalid_or_too_slow() {
    let (check, valid, checks, _signal) = controlled();
    let mut revalidator = Revalidator::new(check, Duration::from_secs(1));
    assert!(revalidator.check().await);
    valid.store(false, Ordering::SeqCst);
    assert!(!revalidator.check().await);
    assert_eq!(checks.load(Ordering::SeqCst), 2);

    let mut slow = Revalidator::new(
        revalidate_with(|| async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            true
        }),
        Duration::from_secs(1),
    );
    let started = Instant::now();
    assert!(!slow.check().await);
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(4_900) && elapsed < Duration::from_secs(10),
        "{elapsed:?}"
    );
}

#[tokio::test]
async fn without_a_revalidator_no_check_is_ever_due() {
    let mut none: Option<Revalidator<NoRevalidation>> = None;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), due(&mut none))
            .await
            .is_err()
    );
    let (check, _, _, signal) = controlled();
    let mut some = Some(Revalidator::new(check, Duration::from_secs(1)));
    signal.send_replace(1);
    tokio::time::timeout(Duration::from_millis(200), due(&mut some))
        .await
        .unwrap();
}
