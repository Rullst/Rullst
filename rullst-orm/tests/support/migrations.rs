//! Built-in migration runner contracts shared by the server matrices.

#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rullst_orm::Orm;
use rullst_orm::schema::{Migration, run_artisan_with_args};

const SLOW_MIGRATION: &str = "m20260930_000001_concurrent_runner";
static SLOW_MIGRATION_UPS: AtomicUsize = AtomicUsize::new(0);

/// A pending migration that stays in `up()` long enough for a concurrent
/// runner to observe it as pending.
struct SlowMigration;

#[rullst_orm::async_trait]
impl Migration for SlowMigration {
    fn name(&self) -> &'static str {
        SLOW_MIGRATION
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        SLOW_MIGRATION_UPS.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Ok(())
    }
}

fn slow_migration() -> Vec<Box<dyn Migration>> {
    vec![Box::new(SlowMigration)]
}

fn artisan(command: &str) -> Vec<String> {
    vec!["artisan".to_string(), command.to_string()]
}

/// `status` and `migrate:rollback` on a database without its own
/// `migrations` table ignore a same-named table in another schema/database.
#[allow(dead_code)]
pub async fn exercise_foreign_migrations_table() {
    let pool = Orm::pool().expect("pool");
    let (create_namespace, create_table, drop_namespace) = match Orm::driver().expect("driver") {
        "postgres" => (
            "CREATE SCHEMA rullst_foreign_app",
            "CREATE TABLE rullst_foreign_app.migrations (id SERIAL PRIMARY KEY, migration VARCHAR(255) NOT NULL, batch INTEGER NOT NULL)",
            "DROP SCHEMA rullst_foreign_app CASCADE",
        ),
        "mysql" => (
            "CREATE DATABASE rullst_foreign_app",
            "CREATE TABLE rullst_foreign_app.migrations (id INT AUTO_INCREMENT PRIMARY KEY, migration VARCHAR(255) NOT NULL, batch INT NOT NULL)",
            "DROP DATABASE rullst_foreign_app",
        ),
        _ => return,
    };
    sqlx::query("DROP TABLE IF EXISTS migrations")
        .execute(pool)
        .await
        .expect("start without an own migrations table");
    sqlx::query(create_namespace)
        .execute(pool)
        .await
        .expect("create foreign schema/database");
    sqlx::query(create_table)
        .execute(pool)
        .await
        .expect("create foreign migrations table");
    sqlx::query(
        "INSERT INTO rullst_foreign_app.migrations (migration, batch) VALUES ('m1_foreign', 1)",
    )
    .execute(pool)
    .await
    .expect("record a foreign migration");

    run_artisan_with_args(&artisan("status"), Vec::new(), Vec::new())
        .await
        .expect("status must ignore another schema's migrations table");
    run_artisan_with_args(&artisan("migrate:rollback"), Vec::new(), Vec::new())
        .await
        .expect("rollback must ignore another schema's migrations table");

    sqlx::query(drop_namespace)
        .execute(pool)
        .await
        .expect("drop foreign schema/database");
}

/// Two runners started together apply a pending migration exactly once and
/// record it once. SQLite has no cross-process runner lock (and concurrent
/// transactions on a shared-cache in-memory database deadlock), so it is
/// skipped there.
#[allow(dead_code)]
pub async fn exercise_concurrent_migration_runners() {
    let driver = Orm::driver().expect("driver");
    if !matches!(driver, "postgres" | "mysql") {
        return;
    }
    let arguments = artisan("migrate");
    let migrate = || run_artisan_with_args(&arguments, slow_migration(), Vec::new());
    let scenario = async {
        let (left, right) = tokio::join!(migrate(), migrate());
        left.expect("first concurrent runner");
        right.expect("second concurrent runner");
    };
    tokio::time::timeout(Duration::from_secs(60), scenario)
        .await
        .expect("concurrent migration runners must not deadlock");
    assert_eq!(
        SLOW_MIGRATION_UPS.load(Ordering::SeqCst),
        1,
        "a pending migration must run once across concurrent runners"
    );
    let sql = if driver == "postgres" {
        "SELECT COUNT(*) FROM migrations WHERE migration = $1"
    } else {
        "SELECT COUNT(*) FROM migrations WHERE migration = ?"
    };
    let (recorded,): (i64,) = sqlx::query_as(sql)
        .bind(SLOW_MIGRATION)
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("count tracking rows");
    assert_eq!(recorded, 1);

    run_artisan_with_args(&artisan("migrate:rollback"), slow_migration(), Vec::new())
        .await
        .expect("roll the concurrent migration back");
}
