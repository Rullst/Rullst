//! `Server::auto_migrate` against a real SQLite file. The ORM pool is
//! process-global, so every scenario runs in this one test, in order.
//!
//! `--all-features` selects the strict PostgreSQL pool type, so this SQLite
//! contract runs with `--features orm`, as CI's strict-sqlite job does.

#![cfg(all(
    feature = "orm",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::{Server, ServerError, routes};
use rullst_orm::schema::Migration;

struct CreateNotes;

#[async_trait::async_trait]
impl Migration for CreateNotes {
    fn name(&self) -> &'static str {
        "m20261015000000_create_notes"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::Orm::raw("CREATE TABLE notes (id INTEGER PRIMARY KEY)")
            .execute()
            .await
            .map(|_| ())
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        rullst_orm::Orm::raw("DROP TABLE notes")
            .execute()
            .await
            .map(|_| ())
    }
}

struct Broken;

#[async_trait::async_trait]
impl Migration for Broken {
    fn name(&self) -> &'static str {
        "m20261015000001_broken"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Err(rullst_orm::Error::Validation(
            "deliberately broken".to_string(),
        ))
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Ok(())
    }
}

async fn start(
    url: &str,
    migrations: Option<Vec<Box<dyn Migration>>>,
    auto_migrate: Option<bool>,
) -> Result<(), ServerError> {
    let mut server = Server::new(routes![get("/" => || async { "ok" })]).with_db(url);
    if let Some(migrations) = migrations {
        server = server.with_migrations(migrations);
    }
    if let Some(enabled) = auto_migrate {
        server = server.auto_migrate(enabled);
    }
    // Bind an ephemeral port and stop at once.
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        server.run_with_shutdown(0, async {}),
    )
    .await
    .expect("startup finishes within the deadline")
}

async fn applied() -> Vec<String> {
    let pool = rullst_orm::Orm::try_pool().unwrap();
    let tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'migrations'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    if tables == 0 {
        return Vec::new();
    }
    sqlx::query_scalar("SELECT migration FROM migrations ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn startup_migrations_run_only_when_enabled_and_failures_stop_startup() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("app.db");
    let url = format!("sqlite://{}?mode=rwc", database.display());

    // Registered but not enabled: nothing runs.
    start(&url, Some(vec![Box::new(CreateNotes)]), None)
        .await
        .unwrap();
    assert!(applied().await.is_empty());
    start(&url, Some(vec![Box::new(CreateNotes)]), Some(false))
        .await
        .unwrap();
    assert!(applied().await.is_empty());

    // Enabled: applied before the server binds, under the SQLite file lock.
    start(&url, Some(vec![Box::new(CreateNotes)]), Some(true))
        .await
        .unwrap();
    assert_eq!(applied().await, vec!["m20261015000000_create_notes"]);
    assert!(dir.path().join("app.db.rullst-migrate.lock").exists());

    // A later start finds nothing pending.
    start(&url, Some(vec![Box::new(CreateNotes)]), Some(true))
        .await
        .unwrap();
    assert_eq!(applied().await, vec!["m20261015000000_create_notes"]);

    // A failing migration stops startup with a typed error.
    let error = start(
        &url,
        Some(vec![Box::new(CreateNotes), Box::new(Broken)]),
        Some(true),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, ServerError::Migration(_)), "{error}");
    assert!(error.to_string().contains("deliberately broken"), "{error}");
    assert_eq!(applied().await, vec!["m20261015000000_create_notes"]);

    // Enabled without a registry fails closed.
    let error = start(&url, None, Some(true)).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "startup migrations failed: no migrations are registered; call Server::with_migrations"
    );
}
