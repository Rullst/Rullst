#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{Orm, with_tenant};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "tenant_records", tenant_column = "tenant_id")]
struct TenantRecord {
    id: i32,
    tenant_id: String,
    name: String,
}

#[tokio::test]
async fn tenant_models_fail_closed_and_require_an_explicit_global_escape_hatch() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize isolated SQLite pool");
    let pool = Orm::pool().expect("ORM pool should be available");
    rullst_orm::_sqlx::query(
        "CREATE TABLE tenant_records (\
            id INTEGER PRIMARY KEY AUTOINCREMENT, \
            tenant_id TEXT NOT NULL, \
            name TEXT NOT NULL\
        )",
    )
    .execute(pool)
    .await
    .expect("create tenant table");
    rullst_orm::_sqlx::query("INSERT INTO tenant_records (tenant_id, name) VALUES (?, ?), (?, ?)")
        .bind("tenant-a")
        .bind("alpha")
        .bind("tenant-b")
        .bind("beta")
        .execute(pool)
        .await
        .expect("seed tenant rows");

    let missing_context = TenantRecord::query().get().await;
    assert!(matches!(
        missing_context,
        Err(rullst_orm::Error::Validation(message)) if message.contains("tenant context is required")
    ));

    let global_rows = TenantRecord::unscoped()
        .order_by("id")
        .get()
        .await
        .expect("explicit unscoped query");
    assert_eq!(global_rows.len(), 2);

    let tenant_a_rows = with_tenant("tenant-a", TenantRecord::all())
        .await
        .expect("tenant-scoped query");
    assert_eq!(tenant_a_rows.len(), 1);
    assert_eq!(tenant_a_rows[0].name, "alpha");

    let invalid_page = TenantRecord::unscoped().paginate(1, 0).await;
    assert!(matches!(
        invalid_page,
        Err(rullst_orm::Error::Validation(_))
    ));
    let overflowing_page = TenantRecord::unscoped().paginate(usize::MAX, 2).await;
    assert!(
        matches!(
            &overflowing_page,
            Err(rullst_orm::Error::Validation(message)) if message.contains("offset exceeds")
        ),
        "unexpected pagination result: {overflowing_page:?}"
    );
    let invalid_chunk = TenantRecord::unscoped().chunk(0, |_| async {}).await;
    assert!(matches!(
        invalid_chunk,
        Err(rullst_orm::Error::Validation(_))
    ));

    let mut no_context_insert = TenantRecord {
        id: 0,
        tenant_id: "forged".to_string(),
        name: "missing context".to_string(),
    };
    assert!(matches!(
        no_context_insert.save().await,
        Err(rullst_orm::Error::Validation(message)) if message.contains("tenant context is required")
    ));

    let mut tenant_insert = TenantRecord {
        id: 0,
        tenant_id: "forged".to_string(),
        name: "scoped".to_string(),
    };
    with_tenant("tenant-a", tenant_insert.save())
        .await
        .expect("scoped insert");
    assert_eq!(tenant_insert.tenant_id, "tenant-a");

    let mut forged_update = global_rows[1].clone();
    forged_update.name = "cross-tenant overwrite".to_string();
    let update_result = with_tenant("tenant-a", forged_update.save()).await;
    assert!(matches!(
        update_result,
        Err(rullst_orm::Error::Validation(message)) if message.contains("outside the active tenant scope")
    ));

    let mut forged_partial = global_rows[1].clone();
    let partial_result = with_tenant(
        "tenant-a",
        forged_partial
            .update_partial()
            .name("partial overwrite".to_string())
            .save(),
    )
    .await;
    assert!(matches!(
        partial_result,
        Err(rullst_orm::Error::Validation(message)) if message.contains("outside the active tenant scope")
    ));

    let delete_result = with_tenant("tenant-a", global_rows[1].delete()).await;
    assert!(matches!(
        delete_result,
        Err(rullst_orm::Error::Validation(message)) if message.contains("outside the active tenant scope")
    ));

    let unchanged = TenantRecord::unscoped()
        .where_eq("id", global_rows[1].id)
        .first()
        .await
        .expect("read row after rejected mutations")
        .expect("tenant-b row should remain");
    assert_eq!(unchanged.tenant_id, "tenant-b");
    assert_eq!(unchanged.name, "beta");

    let invalid_keyset = TenantRecord::unscoped()
        .chunk_by_id(0, |_| async { Ok(()) })
        .await;
    assert!(matches!(
        invalid_keyset,
        Err(rullst_orm::Error::Validation(message)) if message.contains("greater than zero")
    ));

    let callback_error = TenantRecord::unscoped()
        .chunk_by_id(1, |_| async {
            Err(rullst_orm::Error::Validation(
                "stop keyset traversal".to_string(),
            ))
        })
        .await;
    assert!(matches!(
        callback_error,
        Err(rullst_orm::Error::Validation(message)) if message == "stop keyset traversal"
    ));

    let observed_ids = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let delete_pool = Orm::pool()
        .expect("ORM pool should remain available")
        .clone();
    TenantRecord::unscoped()
        .chunk_by_id(1, {
            let observed_ids = Arc::clone(&observed_ids);
            move |rows| {
                let observed_ids = Arc::clone(&observed_ids);
                let delete_pool = delete_pool.clone();
                async move {
                    for row in rows {
                        observed_ids.lock().await.push(row.id);
                        rullst_orm::_sqlx::query("DELETE FROM tenant_records WHERE id = ?")
                            .bind(row.id)
                            .execute(&delete_pool)
                            .await?;
                    }
                    Ok(())
                }
            }
        })
        .await
        .expect("keyset traversal remains stable while processed rows are deleted");
    assert_eq!(*observed_ids.lock().await, vec![1, 2, 3]);
}
