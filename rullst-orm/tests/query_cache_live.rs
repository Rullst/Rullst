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
    // The entry is indexed under its table for commit-time invalidation, and
    // the index lives at least as long as the entry.
    let index_key = format!(
        "{}:keys",
        cache_key.rsplit_once(':').expect("generated entry key").0
    );
    let indexed: bool = redis
        .sismember(&index_key, &cache_key)
        .await
        .expect("inspect cache index");
    let index_ttl: i64 = redis.ttl(&index_key).await.expect("inspect index TTL");
    assert!(indexed);
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
    let unindexed_key = format!("{}:{}", index_key.trim_end_matches(":keys"), "0".repeat(64));
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

    let mut partial = still_cached;
    partial
        .update_partial()
        .name("partial fourth".into())
        .save()
        .await
        .unwrap();
    assert!(!redis.exists::<_, bool>(&cache_key).await.unwrap());
    let refreshed = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refreshed.name, "partial fourth");
    let partial_rollback = Orm::transaction(|_| {
        Box::pin(async {
            let mut row = QueryCacheLiveRecord::find(1).await?.unwrap();
            row.update_partial()
                .name("partial rolled back".into())
                .save()
                .await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "rollback partial cache update".into(),
            ))
        })
    })
    .await;
    assert!(partial_rollback.is_err());
    assert!(redis.exists::<_, bool>(&cache_key).await.unwrap());
    let preserved = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preserved.name, "partial fourth");

    let _: usize = redis
        .del(&cache_key)
        .await
        .expect("remove isolated live cache key");
    exercise_tenant_scoped_invalidation(&mut redis).await;
    exercise_secret_string_cache(&mut redis).await;
    let _ = std::fs::remove_file(database_path);
}

/// The post-commit invalidation runs after the transaction closure returns,
/// outside a `with_tenant` scope entered inside it, and must still remove the
/// keys of the tenant that performed the write.
async fn exercise_tenant_scoped_invalidation(
    redis: &mut rullst_orm::_redis::aio::ConnectionManager,
) {
    let tenant_query = || QueryCacheLiveRecord::query().where_id(1).limit(1);
    let tenant_key = rullst_orm::with_tenant("acme", async {
        let query = tenant_query();
        rullst_orm::query_cache::query_key(
            "query_cache_live_records",
            &query.to_sql(),
            &query.bindings,
        )
    })
    .await
    .expect("derive tenant cache key");
    rullst_orm::with_tenant("acme", tenant_query().remember(30).first())
        .await
        .expect("populate tenant cache")
        .expect("fixture should exist");
    assert!(redis.exists::<_, bool>(&tenant_key).await.unwrap());

    Orm::transaction(|_| {
        Box::pin(async move {
            rullst_orm::with_tenant("acme", async move {
                let mut row = QueryCacheLiveRecord::find(1).await?.ok_or_else(|| {
                    rullst_orm::Error::DatabaseError("live cache fixture disappeared".to_string())
                })?;
                row.name = "tenant scoped".to_string();
                row.save().await
            })
            .await
        })
    })
    .await
    .expect("commit tenant-scoped write");
    assert!(
        !redis.exists::<_, bool>(&tenant_key).await.unwrap(),
        "the commit must invalidate the writing tenant's keys"
    );
    let fresh = rullst_orm::with_tenant("acme", tenant_query().remember(30).first())
        .await
        .expect("read tenant cache after commit")
        .expect("fixture should exist");
    assert_eq!(fresh.name, "tenant scoped");
    let _: usize = redis
        .del(&tenant_key)
        .await
        .expect("remove isolated tenant cache key");
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
