//! A v1 audit table that cannot gain its v2 columns fails the upgrade
//! instead of being used without them.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::Orm;
use rullst_orm::audit::create_audit_table;
use sqlx::Connection;

#[tokio::test]
async fn an_upgrade_that_cannot_add_its_columns_fails() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-audit-upgrade-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut connection =
        sqlx::SqliteConnection::connect(&format!("sqlite:{}?mode=rwc", database_path.display()))
            .await
            .expect("create the database");
    sqlx::query(
        "CREATE TABLE rullst_audits (id INTEGER PRIMARY KEY AUTOINCREMENT, \
         model_type TEXT NOT NULL, model_id INTEGER NOT NULL, event TEXT NOT NULL, \
         old_values TEXT, new_values TEXT, created_at DATETIME)",
    )
    .execute(&mut connection)
    .await
    .expect("create a v1 audit table");
    connection.close().await.expect("close");

    // A read-only connection cannot run the ALTER TABLE statements.
    Orm::init(&format!("sqlite:{}?mode=ro", database_path.display()))
        .await
        .expect("open the database read-only");
    assert!(create_audit_table().await.is_err());

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
