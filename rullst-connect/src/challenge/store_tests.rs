use super::*;
use std::sync::Arc;

const STATE: &str = "s1234567890123456789012345678901";
const VERIFIER: &str = "v123456789012345678901234567890123456789012";

fn challenge(created_at: u64, expires_at: u64) -> LoginChallenge {
    LoginChallenge::new(
        "google",
        STATE,
        Some(STATE.to_string()),
        VERIFIER,
        None,
        created_at,
        expires_at,
    )
    .unwrap()
}

#[test]
fn entry_limits_are_bounded() {
    assert_eq!(
        MemoryChallengeStore::new(0).unwrap_err(),
        ChallengeError::Invalid("entry limit")
    );
    assert_eq!(
        MemoryChallengeStore::new(1_000_001).unwrap_err(),
        ChallengeError::Invalid("entry limit")
    );
    assert!(MemoryChallengeStore::new(1).is_ok());
    assert!(MemoryChallengeStore::new(1_000_000).is_ok());
}

#[tokio::test]
async fn a_challenge_is_taken_exactly_once() {
    let store = MemoryChallengeStore::new(4).unwrap();
    assert!(store.is_empty().unwrap());
    let stored = challenge(100, 700);
    let id = store.save(&stored).await.unwrap();
    assert_eq!(store.len().unwrap(), 1);
    assert!(!store.is_empty().unwrap());
    assert_eq!(store.take(&id, 699).await.unwrap(), stored);
    assert_eq!(store.take(&id, 699).await, Err(ChallengeError::NotFound));
    assert_eq!(store.len().unwrap(), 0);
}

#[tokio::test]
async fn an_expired_challenge_is_refused_and_consumed() {
    let store = MemoryChallengeStore::new(4).unwrap();
    let id = store.save(&challenge(100, 700)).await.unwrap();
    assert_eq!(store.take(&id, 700).await, Err(ChallengeError::Expired));
    assert_eq!(store.take(&id, 100).await, Err(ChallengeError::NotFound));
}

#[tokio::test]
async fn unknown_ids_find_nothing() {
    let store = MemoryChallengeStore::new(4).unwrap();
    store.save(&challenge(100, 700)).await.unwrap();
    let unknown = ChallengeId::generate();
    assert_eq!(
        store.take(&unknown, 100).await,
        Err(ChallengeError::NotFound)
    );
    assert_eq!(store.remove(&unknown).await, Ok(None));
    assert_eq!(store.len().unwrap(), 1);
}

#[tokio::test]
async fn a_full_store_drops_expired_challenges_before_refusing() {
    let store = MemoryChallengeStore::new(2).unwrap();
    let old = store.save(&challenge(100, 200)).await.unwrap();
    let live = store.save(&challenge(150, 400)).await.unwrap();
    // At capacity, a challenge created at 200 drops the one expired at 200.
    let newest = store.save(&challenge(200, 500)).await.unwrap();
    assert_eq!(store.len().unwrap(), 2);
    assert_eq!(store.remove(&old).await, Ok(None));
    // Nothing has expired at 250: the store refuses and keeps both.
    assert_eq!(
        store.save(&challenge(250, 500)).await.unwrap_err(),
        ChallengeError::CapacityExceeded
    );
    assert_eq!(store.take(&live, 250).await.unwrap(), challenge(150, 400));
    assert_eq!(store.take(&newest, 250).await.unwrap(), challenge(200, 500));
}

#[tokio::test]
async fn a_store_below_capacity_keeps_expired_challenges_until_purged() {
    let store = MemoryChallengeStore::new(3).unwrap();
    let expired = store.save(&challenge(100, 200)).await.unwrap();
    store.save(&challenge(300, 400)).await.unwrap();
    assert_eq!(store.len().unwrap(), 2);
    // The expired challenge is still there to be reported as expired.
    assert_eq!(
        store.take(&expired, 300).await,
        Err(ChallengeError::Expired)
    );
}

#[tokio::test]
async fn purging_is_bounded_and_keeps_live_challenges() {
    let store = MemoryChallengeStore::new(8).unwrap();
    for created in [100, 110, 120] {
        store.save(&challenge(created, created + 10)).await.unwrap();
    }
    let live = store.save(&challenge(125, 500)).await.unwrap();
    assert_eq!(store.purge_expired(129, 10).await, Ok(2));
    assert_eq!(store.len().unwrap(), 2);
    assert_eq!(store.purge_expired(130, 0).await, Ok(0));
    assert_eq!(store.purge_expired(130, 1).await, Ok(1));
    assert_eq!(store.purge_expired(130, 1).await, Ok(0));
    assert_eq!(store.len().unwrap(), 1);
    assert!(store.take(&live, 130).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_takes_have_exactly_one_winner() {
    let store = Arc::new(MemoryChallengeStore::new(4).unwrap());
    let id = store.save(&challenge(100, 700)).await.unwrap();
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let store = Arc::clone(&store);
        let id = id.clone();
        tasks.push(tokio::spawn(async move { store.take(&id, 200).await }));
    }
    let mut winners = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => winners += 1,
            Err(error) => assert_eq!(error, ChallengeError::NotFound),
        }
    }
    assert_eq!(winners, 1);
}

#[tokio::test]
async fn a_poisoned_store_fails_closed() {
    let store = Arc::new(MemoryChallengeStore::new(4).unwrap());
    let id = store.save(&challenge(100, 700)).await.unwrap();
    let poisoner = Arc::clone(&store);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.entries.lock().unwrap();
        panic!("poison the challenge store");
    })
    .join();
    let unavailable = ChallengeError::StorageUnavailable("lock memory store");
    assert_eq!(store.len().unwrap_err(), unavailable);
    assert_eq!(store.is_empty().unwrap_err(), unavailable);
    assert_eq!(store.take(&id, 200).await.unwrap_err(), unavailable);
    assert_eq!(
        store.save(&challenge(100, 700)).await.unwrap_err(),
        unavailable
    );
    assert_eq!(store.purge_expired(800, 1).await.unwrap_err(), unavailable);
}
