#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use sqlx::{Connection, SqliteConnection};

fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}

fn config(max_jobs: u32) -> StoreConfig {
    StoreConfig::new(
        reference("store-tests"),
        max_jobs,
        4,
        ExecutionProfile::Simulation,
    )
    .unwrap()
}

fn key() -> ContentKey {
    ContentKey::new([3; 32]).unwrap()
}

/// The number of values bound into a configuration.
fn binding_len(config: &StoreConfig) -> usize {
    let binding: serde_json::Value =
        serde_json::from_str(&config.binding(&key()).unwrap()).unwrap();
    binding.as_array().unwrap().len()
}

async fn initialized(directory: &tempfile::TempDir) -> PathBuf {
    let path = directory.path().join("labs.sqlite");
    let store = SqliteLabs::initialize(&path, config(10), key(), SystemClock)
        .await
        .unwrap();
    store.close().await;
    path
}

async fn tamper(path: &Path, statements: &[&'static str]) {
    let options = sqlx::sqlite::SqliteConnectOptions::new().filename(path);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    for statement in statements.iter().copied() {
        sqlx::query(statement)
            .execute(&mut connection)
            .await
            .unwrap();
    }
    connection.close().await.unwrap();
}

#[test]
fn the_tenant_quota_may_equal_the_store_wide_exercise_limit() {
    assert_eq!(
        config(10).tenant_exercises(4).unwrap().tenant_exercises,
        Some(4)
    );
    assert!(matches!(
        config(10).tenant_exercises(5),
        Err(Error::Configuration)
    ));
    assert!(matches!(
        config(10).tenant_exercises(0),
        Err(Error::Configuration)
    ));
}

#[test]
fn only_an_explicit_quota_extends_the_persisted_binding() {
    // Stores initialized before the quotas existed keep their binding.
    assert_eq!(binding_len(&config(10)), 6);
    assert_eq!(binding_len(&config(500)), 6);
    assert_eq!(
        binding_len(&config(500).learner_jobs(LEARNER_JOBS).unwrap()),
        6
    );
    assert_eq!(binding_len(&config(10).learner_jobs(5).unwrap()), 7);
    assert_eq!(binding_len(&config(10).tenant_exercises(2).unwrap()), 8);
}

#[tokio::test]
async fn a_store_reopens_only_with_its_own_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(&directory).await;
    let store = SqliteLabs::open(&path, config(10), key(), SystemClock)
        .await
        .unwrap();
    store.close().await;
    assert!(matches!(
        SqliteLabs::open(&path, config(11), key(), SystemClock).await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn a_changed_schema_definition_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(&directory).await;
    tamper(
        &path,
        &[
            "DROP INDEX labs_ready",
            "CREATE INDEX labs_ready ON labs_jobs(state)",
        ],
    )
    .await;
    assert!(matches!(
        SqliteLabs::open(&path, config(10), key(), SystemClock).await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn a_metadata_row_without_a_recorded_time_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(&directory).await;
    // Bypass the CHECK constraint, as an external writer could.
    tamper(
        &path,
        &[
            "PRAGMA ignore_check_constraints = ON",
            "UPDATE labs_meta SET last_now = 0",
        ],
    )
    .await;
    assert!(matches!(
        SqliteLabs::open(&path, config(10), key(), SystemClock).await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn a_closed_store_refuses_further_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(&directory).await;
    let store = SqliteLabs::open(&path, config(10), key(), SystemClock)
        .await
        .unwrap();
    store.validate().await.unwrap();
    store.close().await;
    assert!(matches!(store.validate().await, Err(Error::Storage)));
}

#[test]
fn only_verbatim_drive_paths_lose_their_prefix() {
    assert_eq!(
        verbatim_drive_path(r"\\?\C:\Users\labs"),
        Some(r"C:\Users\labs")
    );
    assert_eq!(verbatim_drive_path(r"\\?\UNC\server\share"), None);
    assert_eq!(verbatim_drive_path(r"C:\Users\labs"), None);
    assert_eq!(verbatim_drive_path("/var/lib/labs"), None);
}
