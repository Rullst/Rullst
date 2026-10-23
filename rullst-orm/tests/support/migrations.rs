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

const ATOMIC_MIGRATION: &str = "m20261001_000001_atomic_record";
const ATOMIC_TABLE: &str = "rullst_atomic_migration_probe";

/// A transactional migration that creates a table through `Schema`. It can
/// make the runner's tracking INSERT fail after `up()` succeeded (by
/// recording its own name first, as a lost connection would leave the record
/// missing) or fail its `down()` after dropping the table.
struct AtomicMigration {
    break_record: bool,
    fail_down: bool,
}

#[rullst_orm::async_trait]
impl Migration for AtomicMigration {
    fn name(&self) -> &'static str {
        ATOMIC_MIGRATION
    }

    fn within_transaction(&self) -> bool {
        true
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::schema::Schema::create(ATOMIC_TABLE, |table| {
            table.id();
        })
        .await?;
        if self.break_record {
            let sql = if Orm::driver()? == "postgres" {
                "INSERT INTO migrations (migration, batch) VALUES ($1, 0)"
            } else {
                "INSERT INTO migrations (migration, batch) VALUES (?, 0)"
            };
            let record = sqlx::query(sql).bind(ATOMIC_MIGRATION);
            rullst_orm::execute_query!(record, execute, pool)?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::schema::Schema::drop_if_exists(ATOMIC_TABLE).await?;
        if self.fail_down {
            return Err(rullst_orm::Error::Internal(
                "intentional rollback failure".to_string(),
            ));
        }
        Ok(())
    }
}

fn atomic(break_record: bool, fail_down: bool) -> Vec<Box<dyn Migration>> {
    vec![Box::new(AtomicMigration {
        break_record,
        fail_down,
    })]
}

async fn atomic_state() -> (i64, i64) {
    let driver = Orm::driver().expect("driver");
    let (table_sql, record_sql) = match driver {
        "postgres" => (
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = $1",
            "SELECT COUNT(*) FROM migrations WHERE migration = $1",
        ),
        "mysql" => (
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
            "SELECT COUNT(*) FROM migrations WHERE migration = ?",
        ),
        _ => (
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?",
            "SELECT COUNT(*) FROM migrations WHERE migration = ?",
        ),
    };
    let pool = Orm::pool().expect("pool");
    let (tables,): (i64,) = sqlx::query_as(table_sql)
        .bind(ATOMIC_TABLE)
        .fetch_one(pool)
        .await
        .expect("count probe tables");
    let (records,): (i64,) = sqlx::query_as(record_sql)
        .bind(ATOMIC_MIGRATION)
        .fetch_one(pool)
        .await
        .expect("count probe records");
    (tables, records)
}

/// A migration with `within_transaction()` commits its change and its record
/// together. PostgreSQL and SQLite roll the DDL back when the record (or a
/// `down()`) fails; MySQL/MariaDB commit DDL implicitly, so there only the
/// successful apply/rollback round trip is asserted.
#[allow(dead_code)]
pub async fn exercise_transactional_migrations() {
    let transactional_ddl = Orm::driver().expect("driver") != "mysql";
    if transactional_ddl {
        run_artisan_with_args(&artisan("migrate"), atomic(true, false), Vec::new())
            .await
            .expect_err("a failed tracking INSERT must fail the run");
        assert_eq!(
            atomic_state().await,
            (0, 0),
            "the applied change must roll back with its failed record"
        );
    }

    run_artisan_with_args(&artisan("migrate"), atomic(false, false), Vec::new())
        .await
        .expect("apply the transactional migration");
    assert_eq!(atomic_state().await, (1, 1));

    if transactional_ddl {
        run_artisan_with_args(
            &artisan("migrate:rollback"),
            atomic(false, true),
            Vec::new(),
        )
        .await
        .expect_err("a failed down() must fail the rollback");
        assert_eq!(
            atomic_state().await,
            (1, 1),
            "a failed down() must leave the table and its record in place"
        );
    }

    run_artisan_with_args(
        &artisan("migrate:rollback"),
        atomic(false, false),
        Vec::new(),
    )
    .await
    .expect("roll the transactional migration back");
    assert_eq!(atomic_state().await, (0, 0));
}

const POOL_MIGRATION: &str = "m20261022_000001_pool_in_transaction";
const POOL_TABLE: &str = "rullst_pool_migration_probe";

/// Which pool accessor a transactional migration reaches for after its
/// `Schema` change, as `up()` or `down()`.
#[derive(Clone, Copy)]
enum PoolUse {
    None,
    PoolInUp,
    ReadPoolInDown,
    BeginInDown,
}

struct PoolMigration(PoolUse);

#[rullst_orm::async_trait]
impl Migration for PoolMigration {
    fn name(&self) -> &'static str {
        POOL_MIGRATION
    }

    fn within_transaction(&self) -> bool {
        true
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::schema::Schema::create(POOL_TABLE, |table| {
            table.id();
        })
        .await?;
        if let PoolUse::PoolInUp = self.0 {
            sqlx::query("SELECT 1").execute(Orm::pool()?).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::schema::Schema::drop_if_exists(POOL_TABLE).await?;
        match self.0 {
            PoolUse::ReadPoolInDown => {
                Orm::read_pool()?;
            }
            PoolUse::BeginInDown => {
                Orm::begin_transaction().await?;
            }
            PoolUse::None | PoolUse::PoolInUp => {}
        }
        Ok(())
    }
}

fn pool_migration(pool_use: PoolUse) -> Vec<Box<dyn Migration>> {
    vec![Box::new(PoolMigration(pool_use))]
}

/// The probe table and its `migrations` record, as `(tables, records)`.
async fn pool_probe_state() -> (i64, i64) {
    let driver = Orm::driver().expect("driver");
    let (table_sql, record_sql) = match driver {
        "postgres" => (
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = $1",
            "SELECT COUNT(*) FROM migrations WHERE migration = $1",
        ),
        "mysql" => (
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
            "SELECT COUNT(*) FROM migrations WHERE migration = ?",
        ),
        _ => (
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?",
            "SELECT COUNT(*) FROM migrations WHERE migration = ?",
        ),
    };
    let pool = Orm::pool().expect("the pool is available outside migrations");
    let (tables,): (i64,) = sqlx::query_as(table_sql)
        .bind(POOL_TABLE)
        .fetch_one(pool)
        .await
        .expect("count pool probe tables");
    let (records,): (i64,) = sqlx::query_as(record_sql)
        .bind(POOL_MIGRATION)
        .fetch_one(pool)
        .await
        .expect("count pool probe records");
    (tables, records)
}

/// `Orm::pool()`, `Orm::read_pool()` and `Orm::begin_transaction()` inside
/// the `up()`/`down()` of a migration with `within_transaction()` fail with
/// a message naming the migration, instead of running outside its
/// transaction; the migration is then neither recorded nor forgotten.
/// PostgreSQL and SQLite also roll its `Schema` change back; MySQL/MariaDB
/// commit DDL implicitly, so there only the record is asserted.
#[allow(dead_code)]
pub async fn exercise_pool_access_in_transactional_migrations() {
    let transactional_ddl = Orm::driver().expect("driver") != "mysql";
    let refusal = format!(
        "Validation error: migration {POOL_MIGRATION} returns true from within_transaction(), so Orm::pool() and Orm::read_pool() are unavailable in its up()/down()"
    );

    let error = run_artisan_with_args(
        &artisan("migrate"),
        pool_migration(PoolUse::PoolInUp),
        Vec::new(),
    )
    .await
    .expect_err("Orm::pool() in a transactional up() must fail the run");
    assert!(error.to_string().contains(&refusal), "{error}");
    let (tables, records) = pool_probe_state().await;
    assert_eq!(records, 0, "the refused migration must stay unrecorded");
    if transactional_ddl {
        assert_eq!(tables, 0, "its Schema change must roll back");
    } else {
        rullst_orm::schema::Schema::drop_if_exists(POOL_TABLE)
            .await
            .expect("drop the implicitly committed MySQL table");
    }

    run_artisan_with_args(
        &artisan("migrate"),
        pool_migration(PoolUse::None),
        Vec::new(),
    )
    .await
    .expect("apply the transactional migration");
    assert_eq!(pool_probe_state().await, (1, 1));

    for pool_use in [PoolUse::ReadPoolInDown, PoolUse::BeginInDown] {
        let error = run_artisan_with_args(
            &artisan("migrate:rollback"),
            pool_migration(pool_use),
            Vec::new(),
        )
        .await
        .expect_err("pool access in a transactional down() must fail the rollback");
        assert!(error.to_string().contains(&refusal), "{error}");
        let (tables, records) = pool_probe_state().await;
        assert_eq!(records, 1, "the refused rollback must keep the record");
        if transactional_ddl {
            assert_eq!(tables, 1, "its Schema change must roll back");
        }
    }

    run_artisan_with_args(
        &artisan("migrate:rollback"),
        pool_migration(PoolUse::None),
        Vec::new(),
    )
    .await
    .expect("roll the transactional migration back");
    assert_eq!(pool_probe_state().await, (0, 0));
}

/// The MySQL/MariaDB runner lock name of the current database.
const MYSQL_LOCK_NAME: &str = "CONCAT('rullst_orm_migrations:', SHA1(COALESCE(DATABASE(), '')))";

/// A runner keeps waiting while another session holds the MySQL/MariaDB lock
/// for longer than one 10-second `GET_LOCK` attempt, then migrates.
pub async fn exercise_runner_waits_past_one_lock_attempt() {
    if Orm::driver().expect("driver") != "mysql" {
        return;
    }
    let mut holder = Orm::pool()
        .expect("pool")
        .acquire()
        .await
        .expect("lock holder connection");
    let take = format!("SELECT CAST(GET_LOCK({MYSQL_LOCK_NAME}, 0) AS SIGNED)");
    let taken: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(take.as_str()))
        .fetch_one(&mut *holder)
        .await
        .expect("take the runner lock");
    assert_eq!(taken, Some(1));

    let arguments = artisan("migrate");
    let migrate = run_artisan_with_args(&arguments, Vec::new(), Vec::new());
    let release = async {
        tokio::time::sleep(Duration::from_secs(12)).await;
        let release = format!("SELECT CAST(RELEASE_LOCK({MYSQL_LOCK_NAME}) AS SIGNED)");
        let released: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(release.as_str()))
            .fetch_one(&mut *holder)
            .await
            .expect("release the runner lock");
        released
    };
    let (migrated, released) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(migrate, release)
    })
    .await
    .expect("the runner finishes once the lock is free");
    assert_eq!(released, Some(1));
    migrated.expect("the runner waits for the lock instead of failing");
}
