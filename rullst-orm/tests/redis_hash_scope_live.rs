//! Generated Redis model hashes are bound to the application namespace and
//! the active tenant. Opt-in: set `RULLST_TEST_REDIS_URL`.
#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use redis::AsyncCommands;
use rullst_orm::{Error, FromRow, Orm, RullstValue, with_tenant};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRow, rullst_orm::Orm)]
#[orm(table = "redis_scope_invoices", tenant_column = "organization_id")]
struct ScopedInvoice {
    id: i32,
    organization_id: String,
    total: i64,
}

fn is_validation(result: &Result<impl std::fmt::Debug, Error>, needle: &str) -> bool {
    matches!(result, Err(Error::Validation(message)) if message.contains(needle))
}

#[tokio::test]
async fn redis_hashes_are_namespaced_and_tenant_scoped() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the opt-in live Redis contract");
        return;
    };
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let namespace = format!("redis-hash-scope-{}", rand::random::<u64>());
    Orm::init_redis_with_namespace(&redis_url, &namespace)
        .await
        .expect("initialize live Redis");
    let legacy_key = "orm:redis_scope_invoices:7";
    let mut redis = Orm::redis_manager().expect("Redis manager");
    let _: () = redis.del(legacy_key).await.expect("reset legacy key");

    let invoice = ScopedInvoice {
        id: 7,
        organization_id: "acme".to_string(),
        total: 10,
    };
    assert!(is_validation(
        &invoice.save_to_redis().await,
        "tenant context is required"
    ));
    with_tenant("acme", invoice.save_to_redis())
        .await
        .expect("save in the owning tenant");

    let fetched = with_tenant("acme", ScopedInvoice::get_from_redis(7))
        .await
        .expect("read in the owning tenant")
        .expect("cached invoice");
    assert_eq!(fetched.total, 10);
    assert!(
        with_tenant("globex", ScopedInvoice::get_from_redis(7))
            .await
            .expect("read in another tenant")
            .is_none(),
        "another tenant must not read the hash"
    );
    assert!(is_validation(
        &ScopedInvoice::get_from_redis(7).await,
        "tenant context is required"
    ));
    assert!(is_validation(
        &with_tenant(7_i32, ScopedInvoice::get_from_redis(7)).await,
        "tenant context type does not match"
    ));

    // A handle of one tenant cannot overwrite that tenant's hash from another.
    let forged = ScopedInvoice {
        total: 999,
        ..invoice.clone()
    };
    assert!(is_validation(
        &with_tenant("globex", forged.save_to_redis()).await,
        "outside the active tenant scope"
    ));
    assert_eq!(
        with_tenant(
            "acme",
            ScopedInvoice::increment_redis_field(7, ScopedInvoiceColumn::Total, 5)
        )
        .await
        .expect("increment in the owning tenant"),
        15
    );
    assert!(is_validation(
        &with_tenant(
            "acme",
            ScopedInvoice::increment_redis_field(7, ScopedInvoiceColumn::OrganizationId, 1),
        )
        .await,
        "cannot be incremented"
    ));

    // The hash lives under the namespaced, tenant-bound key only.
    let key = rullst_orm::query_cache::model_hash_key(
        "redis_scope_invoices",
        Some(&RullstValue::String("acme".to_string())),
        "7",
    )
    .expect("hash key");
    assert!(key.contains(&namespace));
    assert!(!key.contains("acme"));
    assert!(redis.exists::<_, bool>(&key).await.expect("EXISTS"));
    assert!(
        !redis
            .exists::<_, bool>(legacy_key)
            .await
            .expect("EXISTS legacy key")
    );
    let _: () = redis.del(&key).await.expect("clean up hash");
}
