#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]

use redis::AsyncCommands;
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "query_cache_live_records")]
struct QueryCacheLiveRecord {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "query_cache_live_secrets")]
struct QueryCacheLiveSecret {
    pub id: i32,
    pub name: String,
    pub cpf: rullst_orm::SecretString,
}

const LIVE_CPF: &str = "123.456.789-00";

#[tokio::test]
async fn redis_cache_is_live_bounded_and_never_replaces_transaction_state() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the opt-in live Redis contract");
        return;
    };
    // This is the only test in this binary, so no other thread reads the
    // process-wide key variables while they are set.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef");
        std::env::set_var("RULLST_ENCRYPTION_KEY_ID", "live-cache-2026");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    let namespace = format!("query-cache-live-{}", rand::random::<u64>());
    let database_path =
        std::env::temp_dir().join(format!("rullst-query-cache-live-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&database_path);
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());

    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Orm::init_redis_with_namespace(&redis_url, &namespace)
        .await
        .expect("initialize live Redis query cache");
    Schema::create("query_cache_live_records", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create live cache table");
    sqlx::query("INSERT INTO query_cache_live_records (name) VALUES (?)")
        .bind("first")
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("insert live cache fixture");

    let query = QueryCacheLiveRecord::query().where_id(1).limit(1);
    let cache_key = rullst_orm::query_cache::query_key(
        "query_cache_live_records",
        &query.to_sql(),
        &query.bindings,
    )
    .expect("derive namespaced cache key");
    assert!(cache_key.contains(&namespace));

    let first = query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("populate live Redis cache")
        .expect("fixture should exist");
    assert_eq!(first.name, "first");

    let mut redis = Orm::redis_manager().expect("Redis manager should be initialized");
    let exists: bool = redis.exists(&cache_key).await.expect("inspect cache key");
    let ttl: i64 = redis.ttl(&cache_key).await.expect("inspect cache TTL");
    assert!(exists);
    assert!((1..=30).contains(&ttl));
    // The entry is indexed under its table for commit-time invalidation,
    // scored by its expiry time, and the index lives at least as long as the
    // entry.
    let index_key = index_of(&cache_key);
    let score: Option<f64> = redis
        .zscore(&index_key, &cache_key)
        .await
        .expect("inspect cache index");
    let index_ttl: i64 = redis.ttl(&index_key).await.expect("inspect index TTL");
    let expires_at_ms = server_time_ms(&mut redis).await + 30_000.0;
    let score = score.expect("the entry key should be indexed");
    assert!(
        score > expires_at_ms - 32_000.0 && score <= expires_at_ms,
        "index score {score} should be the entry expiry"
    );
    assert!(index_ttl >= ttl);

    sqlx::query("UPDATE query_cache_live_records SET name = ? WHERE id = ?")
        .bind("second")
        .bind(1_i32)
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("update authoritative row");

    let cached = query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("read populated cache")
        .expect("cached fixture should exist");
    assert_eq!(cached.name, "first");

    let mut explicit = Orm::begin_transaction()
        .await
        .expect("begin explicit transaction");
    let explicit_row = query
        .clone()
        .remember(30)
        .first_with_tx(&mut explicit)
        .await
        .expect("transactional query must bypass cache")
        .expect("authoritative fixture should exist");
    assert_eq!(explicit_row.name, "second");
    explicit.rollback().await.expect("roll back explicit read");

    let task_scoped_name = Orm::transaction(|_| {
        Box::pin(async move {
            let row = QueryCacheLiveRecord::query()
                .where_id(1)
                .remember(30)
                .first()
                .await?
                .ok_or_else(|| {
                    rullst_orm::Error::DatabaseError("live cache fixture disappeared".to_string())
                })?;
            Ok::<String, rullst_orm::Error>(row.name)
        })
    })
    .await
    .expect("task-scoped query must bypass cache");
    assert_eq!(task_scoped_name, "second");

    let _: () = redis
        .set(&cache_key, "{corrupt-json")
        .await
        .expect("install corrupt cache fixture");
    let recovered = query
        .remember(30)
        .first()
        .await
        .expect("corrupt cache must fall back to database")
        .expect("authoritative fixture should exist");
    assert_eq!(recovered.name, "second");
    let repaired: String = redis
        .get(&cache_key)
        .await
        .expect("read repaired cache entry");
    assert!(repaired.contains("second"));

    // Invalidation follows the table index instead of scanning the keyspace:
    // a key that merely matches the table's key pattern is left to its TTL.
    let unindexed_key = format!(
        "{}:{}",
        index_key.trim_end_matches(":index"),
        "0".repeat(64)
    );
    let _: () = redis
        .set_ex(&unindexed_key, "unindexed", 30)
        .await
        .expect("install unindexed key");
    let mut updated = recovered;
    updated.name = "third".to_string();
    updated
        .save()
        .await
        .expect("model save should commit before cache invalidation");
    let exists_after_commit: bool = redis
        .exists(&cache_key)
        .await
        .expect("inspect invalidated cache key");
    assert!(!exists_after_commit);
    assert!(
        !redis
            .exists::<_, bool>(&index_key)
            .await
            .expect("inspect emptied index")
    );
    assert!(
        redis
            .exists::<_, bool>(&unindexed_key)
            .await
            .expect("inspect unindexed key"),
        "a committed write must not scan for keys outside the table index"
    );
    let _: usize = redis
        .del(&unindexed_key)
        .await
        .expect("remove unindexed key");

    let repopulated = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .expect("repopulate cache after committed update")
        .expect("updated fixture should exist");
    assert_eq!(repopulated.name, "third");

    let rollback = Orm::transaction(|_| {
        Box::pin(async move {
            let mut model = QueryCacheLiveRecord::query()
                .where_id(1)
                .first()
                .await?
                .ok_or_else(|| {
                    rullst_orm::Error::DatabaseError("live cache fixture disappeared".to_string())
                })?;
            model.name = "rolled back fourth".to_string();
            model.save().await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "force cache invalidation rollback".to_string(),
            ))
        })
    })
    .await;
    assert!(rollback.is_err());
    let exists_after_rollback: bool = redis
        .exists(&cache_key)
        .await
        .expect("cache should survive rolled-back update");
    assert!(exists_after_rollback);
    let still_cached = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .expect("read cache after rollback")
        .expect("cached fixture should exist");
    assert_eq!(still_cached.name, "third");

    let _: usize = redis
        .del(&cache_key)
        .await
        .expect("remove isolated live cache key");
    exercise_secret_string_cache(&mut redis).await;
    exercise_expired_index_members(&mut redis).await;
    let _ = std::fs::remove_file(database_path);
}

/// Cached `SecretString` values are ciphertext in Redis and still decrypt to
/// the real value on a cache hit.
async fn exercise_secret_string_cache(redis: &mut rullst_orm::_redis::aio::ConnectionManager) {
    Schema::create("query_cache_live_secrets", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("cpf").not_null();
    })
    .await
    .expect("create live secret cache table");
    let mut secret = QueryCacheLiveSecret {
        id: 0,
        name: "cached".to_string(),
        cpf: rullst_orm::SecretString::new(LIVE_CPF),
    };
    secret
        .save()
        .await
        .expect("insert encrypted secret fixture");

    let query = QueryCacheLiveSecret::query().where_id(secret.id).limit(1);
    let cache_key = rullst_orm::query_cache::query_key(
        "query_cache_live_secrets",
        &query.to_sql(),
        &query.bindings,
    )
    .expect("derive secret cache key");
    let first = query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("populate secret cache")
        .expect("secret fixture should exist");
    assert_eq!(first.cpf.reveal_audited(), LIVE_CPF);

    let raw: String = redis.get(&cache_key).await.expect("read raw secret cache");
    assert!(!raw.contains(LIVE_CPF), "cache holds plaintext: {raw}");
    assert!(raw.contains("RULLST:v2:live-cache-2026:"));

    sqlx::query("UPDATE query_cache_live_secrets SET name = ? WHERE id = ?")
        .bind("database changed")
        .bind(secret.id)
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("change authoritative secret row");
    let cached = query
        .remember(30)
        .first()
        .await
        .expect("read secret cache")
        .expect("cached secret should exist");
    assert_eq!(
        cached.name, "cached",
        "the second read must come from Redis"
    );
    assert_eq!(cached.cpf.reveal_audited(), LIVE_CPF);
    let _: usize = redis
        .del(&cache_key)
        .await
        .expect("remove isolated secret cache key");
}

fn index_of(cache_key: &str) -> String {
    format!(
        "{}:index",
        cache_key.rsplit_once(':').expect("generated entry key").0
    )
}

async fn server_time_ms(redis: &mut rullst_orm::_redis::aio::ConnectionManager) -> f64 {
    let (seconds, micros): (u64, u64) = rullst_orm::_redis::cmd("TIME")
        .query_async(redis)
        .await
        .expect("read Redis server time");
    (seconds * 1_000 + micros / 1_000) as f64
}

/// Index members of entries that already expired are pruned when the next
/// entry is stored and never count toward the per-write invalidation cap, so
/// steady reads of many distinct queries cannot grow the index or make the
/// next write fail with `PostCommit`.
async fn exercise_expired_index_members(redis: &mut rullst_orm::_redis::aio::ConnectionManager) {
    let live_query = QueryCacheLiveRecord::query().where_id(1).limit(1);
    let live_key = rullst_orm::query_cache::query_key(
        "query_cache_live_records",
        &live_query.to_sql(),
        &live_query.bindings,
    )
    .expect("derive live cache key");
    let index_key = index_of(&live_key);
    let prefix = index_key.trim_end_matches(":index").to_string();
    let expired = |round: u32| -> Vec<(f64, String)> {
        (0..10_001_u32)
            .map(|member| {
                (
                    1.0,
                    format!(
                        "{prefix}:{:064x}",
                        u64::from(round) << 32 | u64::from(member)
                    ),
                )
            })
            .collect()
    };

    for chunk in expired(1).chunks(1_000) {
        let _: usize = redis
            .zadd_multiple(&index_key, chunk)
            .await
            .expect("index members of expired entries");
    }
    live_query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("store a live entry")
        .expect("fixture should exist");
    let stale: usize = redis
        .zcount(&index_key, 0, 2)
        .await
        .expect("count expired index members");
    assert_eq!(stale, 0, "storing an entry prunes expired index members");
    assert!(
        redis
            .zscore::<_, _, Option<f64>>(&index_key, &live_key)
            .await
            .unwrap()
            .is_some()
    );

    for chunk in expired(2).chunks(1_000) {
        let _: usize = redis
            .zadd_multiple(&index_key, chunk)
            .await
            .expect("index members of expired entries");
    }
    let mut row = QueryCacheLiveRecord::find(1)
        .await
        .expect("load fixture")
        .expect("fixture should exist");
    row.name = "after expired members".to_string();
    row.save()
        .await
        .expect("expired index members must not count toward the invalidation cap");
    assert!(!redis.exists::<_, bool>(&live_key).await.unwrap());
    assert!(
        !redis.exists::<_, bool>(&index_key).await.unwrap(),
        "invalidation removes live and expired members alike"
    );
}
