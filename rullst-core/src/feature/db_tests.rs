//! Cache policy of `DbFeatureDriver`, driven by scripted lookups.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::atomic::AtomicUsize;

const TTL: Duration = Duration::from_millis(200);

/// Resolves `flag` through `driver`, counting lookups that actually ran.
async fn resolve(
    driver: &DbFeatureDriver,
    flag: &str,
    lookups: &AtomicUsize,
    result: Lookup,
) -> Option<FlagRow> {
    driver
        .resolve_with(flag, || async {
            lookups.fetch_add(1, Ordering::SeqCst);
            result
        })
        .await
}

fn enabled() -> Lookup {
    Lookup::Found((true, None, None))
}

#[tokio::test]
async fn a_missing_flag_is_cached_for_the_ttl() {
    let driver = DbFeatureDriver::with_ttl(TTL);
    let lookups = AtomicUsize::new(0);

    assert_eq!(
        resolve(&driver, "absent", &lookups, Lookup::Missing).await,
        None
    );
    assert_eq!(resolve(&driver, "absent", &lookups, enabled()).await, None);
    assert_eq!(lookups.load(Ordering::SeqCst), 1, "the miss was not cached");

    tokio::time::sleep(TTL + Duration::from_millis(50)).await;
    assert!(
        resolve(&driver, "absent", &lookups, enabled())
            .await
            .is_some()
    );
    assert_eq!(lookups.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_failed_lookup_is_cached_and_serves_the_last_known_value() {
    let driver = DbFeatureDriver::with_ttl(TTL);
    let lookups = AtomicUsize::new(0);

    // No value was ever read: the failure is cached as absent.
    assert_eq!(
        resolve(&driver, "fresh", &lookups, Lookup::Failed).await,
        None
    );
    assert_eq!(resolve(&driver, "fresh", &lookups, enabled()).await, None);
    assert_eq!(
        lookups.load(Ordering::SeqCst),
        1,
        "the failure was not cached"
    );

    // A value was read before: an expired entry survives a failed refresh.
    assert!(
        resolve(&driver, "known", &lookups, enabled())
            .await
            .is_some()
    );
    tokio::time::sleep(TTL + Duration::from_millis(50)).await;
    assert_eq!(
        resolve(&driver, "known", &lookups, Lookup::Failed).await,
        Some((true, None, None))
    );
    assert_eq!(
        resolve(&driver, "known", &lookups, Lookup::Missing).await,
        Some((true, None, None)),
        "the stale value is cached again for the TTL"
    );
    assert_eq!(lookups.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn an_uninitialized_pool_is_not_cached() {
    let driver = DbFeatureDriver::with_ttl(TTL);
    let lookups = AtomicUsize::new(0);

    assert_eq!(
        resolve(&driver, "early", &lookups, Lookup::Unavailable).await,
        None
    );
    assert!(
        resolve(&driver, "early", &lookups, enabled())
            .await
            .is_some()
    );
    assert_eq!(lookups.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_cache_holds_a_bounded_number_of_flag_names() {
    let driver = DbFeatureDriver::with_ttl(Duration::from_secs(60));
    let lookups = AtomicUsize::new(0);
    for index in 0..MAX_CACHED_FLAGS + 10 {
        resolve(&driver, &format!("flag-{index}"), &lookups, Lookup::Missing).await;
    }
    assert_eq!(driver.cache.len(), MAX_CACHED_FLAGS);
}
