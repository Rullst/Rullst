//! Reads and bulk deletes bind the tenant context as the tenant field's type,
//! failing closed on a mismatch exactly like `save()`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm, with_tenant};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "typed_tenant_orders", tenant_column = "org")]
struct TypedTenantOrder {
    id: i32,
    org: String,
}

fn is_type_mismatch<T: std::fmt::Debug>(result: &Result<T, Error>) -> bool {
    matches!(result, Err(Error::Validation(message)) if message.contains("tenant context type does not match"))
}

#[tokio::test]
async fn mistyped_tenant_contexts_fail_closed_for_reads_and_bulk_deletes() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE typed_tenant_orders (id INTEGER PRIMARY KEY, org TEXT NOT NULL)",
        "INSERT INTO typed_tenant_orders (id, org) VALUES (1, '5'), (2, '5-acme'), (3, 'globex')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }

    // An integer context against a text tenant column used to be bound as
    // `org = 5`, which matches '5' here and also '5-acme' on MySQL.
    assert!(is_type_mismatch(
        &with_tenant(5_i32, TypedTenantOrder::all()).await
    ));
    assert!(is_type_mismatch(
        &with_tenant(5_i32, async { TypedTenantOrder::query().count().await }).await
    ));
    assert!(is_type_mismatch(
        &with_tenant(5_i32, async {
            TypedTenantOrder::query().delete_all().await
        })
        .await
    ));
    let mut order = TypedTenantOrder {
        id: 0,
        org: "5".into(),
    };
    assert!(is_type_mismatch(&with_tenant(5_i32, order.save()).await));
    assert_eq!(
        TypedTenantOrder::unscoped().count().await.expect("count"),
        3
    );

    let matching = with_tenant("5", TypedTenantOrder::all())
        .await
        .expect("correctly typed tenant");
    assert_eq!(
        matching.iter().map(|order| order.id).collect::<Vec<_>>(),
        [1]
    );
}
