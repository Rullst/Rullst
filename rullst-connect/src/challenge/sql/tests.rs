use super::*;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

const STATE: &str = "s1234567890123456789012345678901";
const VERIFIER: &str = "v123456789012345678901234567890123456789012";
const NOW: u64 = 1_800_000_000;

async fn memory_pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap()
}

fn challenge() -> LoginChallenge {
    LoginChallenge::new(
        "google",
        STATE,
        None,
        VERIFIER,
        Some(ReturnPath::parse("/home").unwrap()),
        NOW,
        NOW + 600,
    )
    .unwrap()
}

#[tokio::test]
async fn rows_are_keyed_by_the_id_digest_never_the_id() {
    let pool = memory_pool().await;
    let store = SqlChallengeStore::open(pool.clone(), 8).await.unwrap();
    let id = store.save(&challenge()).await.unwrap();
    let (key, provider, return_to): (String, String, Option<String>) =
        sqlx::query_as("SELECT id_key, provider, return_to FROM rullst_connect_login_challenges")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(key, id.storage_key());
    assert_ne!(key, id.as_str());
    assert_eq!(provider, "google");
    assert_eq!(return_to.as_deref(), Some("/home"));
    let clone = store.clone();
    assert_eq!(clone.count().await.unwrap(), 1);
}

#[tokio::test]
async fn corrupt_rows_are_reported_and_still_consumed() {
    let pool = memory_pool().await;
    let store = SqlChallengeStore::open(pool.clone(), 8).await.unwrap();
    for (column, value) in [
        ("created_at", "-1"),
        ("expires_at", "-1"),
        ("state", "'s123456789012345678901234567890!'"),
        ("return_to", "'//evil.example'"),
        ("expires_at", "created_at + 601"),
    ] {
        let id = store.save(&challenge()).await.unwrap();
        let update = format!("UPDATE rullst_connect_login_challenges SET {column} = {value}");
        sqlx::query(sqlx::AssertSqlSafe(update))
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            store.take(&id, NOW).await,
            Err(ChallengeError::CorruptStorage("stored challenge")),
            "{column}"
        );
        assert_eq!(store.count().await.unwrap(), 0, "{column}");
    }
}

#[tokio::test]
async fn storage_failures_are_typed() {
    let pool = memory_pool().await;
    let store = SqlChallengeStore::open(pool.clone(), 8).await.unwrap();
    let id = store.save(&challenge()).await.unwrap();
    sqlx::query("DROP TABLE rullst_connect_login_challenges")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        store.save(&challenge()).await,
        Err(ChallengeError::StorageUnavailable("count challenges"))
    );
    assert_eq!(
        store.take(&id, NOW).await,
        Err(ChallengeError::StorageUnavailable("consume challenge"))
    );
    assert_eq!(
        store.purge_expired(NOW, 1).await,
        Err(ChallengeError::StorageUnavailable(
            "purge expired challenges"
        ))
    );
    assert_eq!(
        store.count().await,
        Err(ChallengeError::StorageUnavailable("count challenges"))
    );
    pool.close().await;
    assert_eq!(
        SqlChallengeStore::open(pool, 8).await.unwrap_err(),
        ChallengeError::StorageUnavailable("begin challenge schema")
    );
}

#[test]
fn signed_seconds_saturate_and_counts_stay_unsigned() {
    assert_eq!(seconds(0), 0);
    assert_eq!(seconds(NOW), 1_800_000_000);
    assert_eq!(seconds(i64::MAX as u64), i64::MAX);
    assert_eq!(seconds(u64::MAX), i64::MAX);
    assert_eq!(count_from(7), Ok(7));
    assert_eq!(
        count_from(-1),
        Err(ChallengeError::CorruptStorage("entry count"))
    );
}
