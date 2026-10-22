//! The login-challenge store contract shared by every SQL backend.
//!
//! Each caller passes a pool to an empty database (no challenge table).

use rullst_connect::challenge::{
    ChallengeDatabase, ChallengeError, ChallengeId, ChallengeStore, LoginChallenge, ReturnPath,
    SqlChallengeStore,
};
use std::sync::Arc;
use std::time::Duration;

const STATE: &str = "s1234567890123456789012345678901";
const NONCE: &str = "n1234567890123456789012345678901";
const VERIFIER: &str = "v123456789012345678901234567890123456789012";
const NOW: u64 = 1_800_000_000;

fn challenge(created_at: u64, expires_at: u64) -> LoginChallenge {
    LoginChallenge::new(
        "google",
        STATE,
        Some(NONCE.to_string()),
        VERIFIER,
        Some(ReturnPath::parse("/courses/7?tab=lessons").unwrap()),
        created_at,
        expires_at,
    )
    .unwrap()
}

pub async fn exercise<DB: ChallengeDatabase>(pool: sqlx::Pool<DB>) {
    for limit in [0, 1_000_001] {
        assert_eq!(
            SqlChallengeStore::open(pool.clone(), limit)
                .await
                .unwrap_err(),
            ChallengeError::Invalid("entry limit")
        );
    }
    let store = SqlChallengeStore::open(pool.clone(), 3).await.unwrap();
    // Opening again finds the existing table and index.
    let again = SqlChallengeStore::open(pool.clone(), 3).await.unwrap();
    assert_eq!(store.count().await.unwrap(), 0);
    assert_eq!(
        format!("{store:?}"),
        "SqlChallengeStore { database: \"[REDACTED]\", max_entries: 3 }"
    );

    // Round trip of every field, then a replay finds nothing.
    let stored = challenge(NOW, NOW + 600);
    let id = store.save(&stored).await.unwrap();
    assert_eq!(again.count().await.unwrap(), 1);
    assert_eq!(again.take(&id, NOW + 599).await.unwrap(), stored);
    assert_eq!(store.take(&id, NOW).await, Err(ChallengeError::NotFound));

    // An OAuth challenge without nonce or return path round-trips too.
    let plain = LoginChallenge::new("github", STATE, None, VERIFIER, None, NOW, NOW + 1).unwrap();
    let id = store.save(&plain).await.unwrap();
    assert_eq!(store.remove(&id).await.unwrap(), Some(plain));
    assert_eq!(store.remove(&id).await.unwrap(), None);

    // Expiry consumes the row; unknown ids find nothing.
    let id = store.save(&challenge(NOW, NOW + 60)).await.unwrap();
    assert_eq!(
        store.take(&id, NOW + 60).await,
        Err(ChallengeError::Expired)
    );
    assert_eq!(store.take(&id, NOW).await, Err(ChallengeError::NotFound));
    assert_eq!(
        store.take(&ChallengeId::generate(), NOW).await,
        Err(ChallengeError::NotFound)
    );
    assert_eq!(store.count().await.unwrap(), 0);

    // At its limit a save first drops expired rows, then refuses.
    let early = store.save(&challenge(NOW, NOW + 10)).await.unwrap();
    let live = store.save(&challenge(NOW, NOW + 600)).await.unwrap();
    store.save(&challenge(NOW, NOW + 600)).await.unwrap();
    assert_eq!(
        store.save(&challenge(NOW + 9, NOW + 600)).await,
        Err(ChallengeError::CapacityExceeded)
    );
    let replacement = store.save(&challenge(NOW + 10, NOW + 600)).await.unwrap();
    assert_eq!(store.count().await.unwrap(), 3);
    assert_eq!(store.remove(&early).await.unwrap(), None);
    // A larger limit on another handle shares the same rows.
    let wider = SqlChallengeStore::open(pool.clone(), 4).await.unwrap();
    wider.save(&challenge(NOW, NOW + 600)).await.unwrap();
    assert_eq!(wider.count().await.unwrap(), 4);

    // Purging is bounded and keeps live rows.
    let purge = SqlChallengeStore::open(pool.clone(), 1_000_000)
        .await
        .unwrap();
    for offset in 0..3 {
        purge
            .save(&challenge(NOW + offset, NOW + 20 + offset))
            .await
            .unwrap();
    }
    assert_eq!(purge.purge_expired(NOW + 21, 10).await, Ok(2));
    assert_eq!(purge.purge_expired(NOW + 30, 0).await, Ok(0));
    assert_eq!(purge.purge_expired(NOW + 30, 1).await, Ok(1));
    assert_eq!(purge.purge_expired(NOW + 30, 1).await, Ok(0));
    assert_eq!(purge.count().await.unwrap(), 4);
    assert!(purge.take(&live, NOW + 30).await.is_ok());
    assert!(purge.take(&replacement, NOW + 30).await.is_ok());
    // A `now` beyond the signed column range purges everything expired.
    assert_eq!(purge.purge_expired(u64::MAX, 10).await, Ok(2));
    assert_eq!(purge.count().await.unwrap(), 0);

    concurrent_takes_have_one_winner(purge).await;
}

async fn concurrent_takes_have_one_winner<DB: ChallengeDatabase>(store: SqlChallengeStore<DB>) {
    let store = Arc::new(store);
    for round in 0..4 {
        let id = store.save(&challenge(NOW, NOW + 600)).await.unwrap();
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let id = id.clone();
            tasks.push(tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(30), store.take(&id, NOW)).await
            }));
        }
        let mut winners = 0;
        for task in tasks {
            match task.await.unwrap().expect("bounded take") {
                Ok(_) => winners += 1,
                Err(error) => assert_eq!(error, ChallengeError::NotFound, "round {round}"),
            }
        }
        assert_eq!(winners, 1, "round {round}");
    }
    assert_eq!(store.count().await.unwrap(), 0);
}
