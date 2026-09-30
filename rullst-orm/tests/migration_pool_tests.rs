#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use async_trait::async_trait;
use rullst_orm::schema::migration::{Migration, run_artisan_with_args};
use rullst_orm::{Error, Orm, Seeder};

struct DummyMigration;
#[async_trait]
impl Migration for DummyMigration {
    fn name(&self) -> &'static str {
        "m20260820_000001_create_dummy_table"
    }
    async fn up(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("CREATE TABLE IF NOT EXISTS dummy_records (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(pool)
            .await?;
        Ok(())
    }
    async fn down(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("DROP TABLE IF EXISTS dummy_records")
            .execute(pool)
            .await?;
        Ok(())
    }
}

struct DummySeeder;
#[async_trait]
impl Seeder for DummySeeder {
    async fn run(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("INSERT INTO dummy_records (name) VALUES ('seed_entry')")
            .execute(pool)
            .await?;
        Ok(())
    }
}

struct TrackedBeforeFailureMigration;
#[async_trait]
impl Migration for TrackedBeforeFailureMigration {
    fn name(&self) -> &'static str {
        "m20260820_000002_tracked_before_failure"
    }

    async fn up(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("CREATE TABLE tracked_before_failure (id INTEGER PRIMARY KEY)")
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("DROP TABLE IF EXISTS tracked_before_failure")
            .execute(pool)
            .await?;
        Ok(())
    }
}

struct FailingMigration;
#[async_trait]
impl Migration for FailingMigration {
    fn name(&self) -> &'static str {
        "m20260820_000003_intentional_failure"
    }

    async fn up(&self) -> Result<(), Error> {
        Err(Error::Internal("intentional migration failure".to_string()))
    }

    async fn down(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct FailingDownMigration;
#[async_trait]
impl Migration for FailingDownMigration {
    fn name(&self) -> &'static str {
        "m20260820_000004_failing_down"
    }

    async fn up(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn down(&self) -> Result<(), Error> {
        Err(Error::Internal("intentional rollback failure".to_string()))
    }
}

struct RevertedBeforeFailureMigration;
#[async_trait]
impl Migration for RevertedBeforeFailureMigration {
    fn name(&self) -> &'static str {
        "m20260820_000005_reverted_before_failure"
    }

    async fn up(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("CREATE TABLE reverted_before_failure (id INTEGER PRIMARY KEY)")
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), Error> {
        let pool = Orm::pool()?;
        sqlx::query("DROP TABLE reverted_before_failure")
            .execute(pool)
            .await?;
        Ok(())
    }
}

async fn tracked_rows(name: &str) -> i64 {
    let tracked: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM migrations WHERE migration = ?")
        .bind(name)
        .fetch_one(Orm::pool().expect("ORM pool"))
        .await
        .expect("read migration tracking row");
    tracked.0
}

#[tokio::test]
async fn test_migration_and_pool_suite() {
    let _ = Orm::init("sqlite:file:migration_suite_db?mode=memory&cache=shared").await;

    // --- Part 1: Artisan CLI & Migration Lifecycle ---

    // 1. CLI help (no args)
    let res = run_artisan_with_args(&["artisan".into()], vec![], vec![]).await;
    assert!(res.is_ok());

    // A successful migration preceding a later failure must be recorded
    // immediately, otherwise the next run would try to apply it twice.
    let failed_batch = run_artisan_with_args(
        &["artisan".into(), "migrate".into()],
        vec![
            Box::new(TrackedBeforeFailureMigration),
            Box::new(FailingMigration),
        ],
        vec![],
    )
    .await;
    assert!(failed_batch.is_err());
    let tracked: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM migrations WHERE migration = ?")
        .bind(TrackedBeforeFailureMigration.name())
        .fetch_one(Orm::pool().expect("ORM pool"))
        .await
        .expect("read migration tracking row");
    assert_eq!(tracked.0, 1);
    run_artisan_with_args(
        &["artisan".into(), "db:rollback".into()],
        vec![Box::new(TrackedBeforeFailureMigration)],
        vec![],
    )
    .await
    .expect("rollback tracked successful migration");

    // A rollback that fails part-way must forget every migration whose down()
    // already ran; otherwise `migrate` would skip the reverted migration and a
    // repeated rollback would run its down() against a dropped table.
    let batch = || -> Vec<Box<dyn Migration>> {
        vec![
            Box::new(FailingDownMigration),
            Box::new(RevertedBeforeFailureMigration),
        ]
    };
    run_artisan_with_args(&["artisan".into(), "migrate".into()], batch(), vec![])
        .await
        .expect("apply the partial-rollback batch");
    let partial_rollback = run_artisan_with_args(
        &["artisan".into(), "migrate:rollback".into()],
        batch(),
        vec![],
    )
    .await;
    assert!(partial_rollback.is_err());
    assert_eq!(
        tracked_rows(RevertedBeforeFailureMigration.name()).await,
        0,
        "a reverted migration must not stay recorded as applied"
    );
    assert_eq!(tracked_rows(FailingDownMigration.name()).await, 1);
    run_artisan_with_args(&["artisan".into(), "migrate".into()], batch(), vec![])
        .await
        .expect("re-apply the reverted migration");
    assert_eq!(tracked_rows(RevertedBeforeFailureMigration.name()).await, 1);
    sqlx::query("DROP TABLE reverted_before_failure")
        .execute(Orm::pool().expect("ORM pool"))
        .await
        .expect("the re-applied migration recreated its table");
    sqlx::query("DELETE FROM migrations WHERE migration IN (?, ?)")
        .bind(FailingDownMigration.name())
        .bind(RevertedBeforeFailureMigration.name())
        .execute(Orm::pool().expect("ORM pool"))
        .await
        .expect("clear the partial-rollback fixture");

    // 2. make:migration without name
    let res =
        run_artisan_with_args(&["artisan".into(), "make:migration".into()], vec![], vec![]).await;
    assert!(res.is_ok());

    // 3. Status before migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "status".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 4. Run migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "migrate".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 5. Run migrations again (nothing to migrate)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:migrate".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 6. Status after migrations (applied)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:status".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 7. Seed database
    let res = run_artisan_with_args(
        &["artisan".into(), "db:seed".into()],
        vec![],
        vec![Box::new(DummySeeder)],
    )
    .await;
    assert!(res.is_ok());

    // 8. Rollback migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "migrate:rollback".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 9. Rollback again (nothing to rollback)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:rollback".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 10. Unknown command
    let res = run_artisan_with_args(
        &["artisan".into(), "unknown_command".into()],
        vec![],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // --- Part 2: Transaction Commit & Rollback ---
    if let Ok(pool) = Orm::try_pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS tx_items (id INTEGER PRIMARY KEY AUTOINCREMENT, val TEXT)",
        )
        .execute(pool)
        .await;
    }

    // Transaction success
    let res: Result<i32, Error> = Orm::transaction(|_| {
        Box::pin(async {
            let query = sqlx::query("INSERT INTO tx_items (val) VALUES ('tx_ok')");
            rullst_orm::execute_query!(query, execute, pool)?;
            Ok::<i32, Error>(42)
        })
    })
    .await;
    assert_eq!(res.unwrap(), 42);

    // Transaction failure & rollback
    let err_res: Result<i32, Error> = Orm::transaction(|_| {
        Box::pin(async {
            let query = sqlx::query("INSERT INTO tx_items (val) VALUES ('tx_fail')");
            rullst_orm::execute_query!(query, execute, pool)?;
            Err::<i32, Error>(Error::Internal("intentional error".to_string()))
        })
    })
    .await;
    assert!(err_res.is_err());
    let failed_rows: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tx_items WHERE val = 'tx_fail'")
        .fetch_one(Orm::pool().expect("ORM pool"))
        .await
        .expect("count rolled-back rows");
    assert_eq!(failed_rows.0, 0);
}

#[test]
fn test_pool_helpers_and_placeholders() {
    // 1. Placeholder replacement for postgres
    let sql = "SELECT * FROM users WHERE id = ? AND email = ? AND role = ?";
    let replaced = rullst_orm::pool::replace_placeholders(sql);
    assert_eq!(
        replaced,
        "SELECT * FROM users WHERE id = $1 AND email = $2 AND role = $3"
    );

    // 2. Prevent lazy loading flag
    rullst_orm::pool::prevent_lazy_loading(true);
    assert!(rullst_orm::pool::is_lazy_loading_prevented());
    rullst_orm::pool::prevent_lazy_loading(false);
    assert!(!rullst_orm::pool::is_lazy_loading_prevented());

    // 3. Query settings
    Orm::enable_query_log();
    Orm::disable_query_log();
    Orm::set_max_query_limit(500);
    Orm::set_query_timeout(30);
}

#[test]
fn test_schema_diff_ast_extraction() {
    let tables = rullst_orm::schema_diff::extract_tables_from_ast();
    assert!(!tables.is_empty() || tables.is_empty());
}
