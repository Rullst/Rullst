//! Runs the driver-neutral contracts on SQLite. The live PostgreSQL, MySQL
//! and MariaDB matrices run the same `driver_contract` module.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod driver_contract;

use rullst_orm::Orm;

#[tokio::test]
async fn driver_contract_holds_on_sqlite() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-driver-contract-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize SQLite contract database");

    driver_contract::exercise().await;

    Orm::pool().expect("SQLite pool").close().await;
    let _ = std::fs::remove_file(database_path);
}
