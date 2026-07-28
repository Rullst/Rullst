#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::Orm;

#[tokio::test]
async fn test_init_with_replicas() {
    let db_primary = "it_replica_primary.db";
    let db_replica1 = "it_replica_1.db";
    let db_replica2 = "it_replica_2.db";

    let _ = std::fs::remove_file(db_primary);
    let _ = std::fs::remove_file(db_replica1);
    let _ = std::fs::remove_file(db_replica2);

    let replica1_url = format!("sqlite:{}?mode=rwc", db_replica1);
    let replica2_url = format!("sqlite:{}?mode=rwc", db_replica2);
    let replicas = vec![replica1_url.as_str(), replica2_url.as_str()];

    Orm::init_with_replicas(&format!("sqlite:{}?mode=rwc", db_primary), replicas)
        .await
        .expect("Orm::init_with_replicas should succeed");

    // The primary pool should be accessible
    let _pool = Orm::pool();

    // The read_pool should alternate between the replicas
    let read_pool1 = Orm::read_pool();
    let read_pool2 = Orm::read_pool();

    // We can execute queries on them
    sqlx::query("SELECT 1")
        .execute(read_pool1)
        .await
        .expect("query replica 1");

    sqlx::query("SELECT 1")
        .execute(read_pool2)
        .await
        .expect("query replica 2");

    let _ = std::fs::remove_file(db_primary);
    let _ = std::fs::remove_file(db_replica1);
    let _ = std::fs::remove_file(db_replica2);
}
