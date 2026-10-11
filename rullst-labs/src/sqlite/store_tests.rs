#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use sqlx::{Connection, SqliteConnection};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

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

#[tokio::test]
async fn validation_itself_rejects_a_foreign_binding_or_a_missing_time() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(&directory).await;
    let own = SqliteLabs::connect(&path, config(10), key(), SystemClock)
        .await
        .unwrap();
    own.validate().await.unwrap();
    own.close().await;

    let foreign = SqliteLabs::connect(&path, config(11), key(), SystemClock)
        .await
        .unwrap();
    assert!(matches!(
        foreign.validate().await,
        Err(Error::Configuration)
    ));
    foreign.close().await;

    tamper(
        &path,
        &[
            "PRAGMA ignore_check_constraints = ON",
            "UPDATE labs_meta SET last_now = 0",
        ],
    )
    .await;
    let untimed = SqliteLabs::connect(&path, config(10), key(), SystemClock)
        .await
        .unwrap();
    assert!(matches!(
        untimed.validate().await,
        Err(Error::Configuration)
    ));
}

/// Returns its time, then advances it by the step (zero during setup).
#[derive(Clone)]
struct Stepping(Arc<AtomicI64>, Arc<AtomicI64>);
impl Clock for Stepping {
    fn now(&self) -> Result<i64, Error> {
        Ok(self
            .0
            .fetch_add(self.1.load(Ordering::SeqCst), Ordering::SeqCst))
    }
}

const T: i64 = 1_800_000_000;

/// A store whose clock advances one second on every read once it is open.
async fn stepping(directory: &tempfile::TempDir) -> SqliteLabs<Stepping> {
    let clock = Stepping(Arc::new(AtomicI64::new(T)), Arc::new(AtomicI64::new(0)));
    let store = SqliteLabs::initialize(
        directory.path().join("labs.sqlite"),
        config(10),
        key(),
        clock.clone(),
    )
    .await
    .unwrap();
    clock.1.store(1, Ordering::SeqCst);
    store
}

#[tokio::test]
async fn a_commit_accepts_time_that_advances_within_its_deadline() {
    let directory = tempfile::tempdir().unwrap();
    let store = stepping(&directory).await;
    // Begin reads T and T + 1; the commit reads T + 2 and T + 3.
    let tx = super::super::transaction::Operation::begin(&store, None)
        .await
        .unwrap();
    assert_eq!(tx.now, T + 1);
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn a_narrowed_deadline_bounds_the_commit_and_its_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let store = stepping(&directory).await;
    let mut tx = super::super::transaction::Operation::begin(&store, None)
        .await
        .unwrap();
    assert!(matches!(tx.narrow_deadline(T + 1), Err(Error::Expired)));
    tx.narrow_deadline(T + 2).unwrap();
    // The commit reads T + 2, which reaches the deadline.
    assert!(matches!(tx.commit().await, Err(Error::Expired)));
    // Begin reads T + 3 and T + 4; the commit reads T + 5 before the
    // deadline but confirms at T + 6, which reaches it.
    let mut tx = super::super::transaction::Operation::begin(&store, None)
        .await
        .unwrap();
    tx.narrow_deadline(T + 6).unwrap();
    assert!(matches!(tx.commit().await, Err(Error::Uncertain)));
}

#[tokio::test]
async fn metadata_changed_while_open_stops_every_operation() {
    let changes: [&[&'static str]; 4] = [
        &["UPDATE labs_meta SET configuration = 'changed'"],
        &[
            "PRAGMA ignore_check_constraints = ON",
            "UPDATE labs_meta SET last_now = 0",
        ],
        // One more job or exercise row than the configuration allows.
        &[
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 11) \
             INSERT INTO labs_jobs SELECT 't', 'c', 'job' || i, 'l', 'Queued', 1, 0, 0, x'00', NULL FROM n",
        ],
        &[
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 5) \
             INSERT INTO labs_exercises SELECT 't', 'c', 'e' || i, 'v1', 'd', 0, x'00' FROM n",
        ],
    ];
    for change in changes {
        let directory = tempfile::tempdir().unwrap();
        let path = initialized(&directory).await;
        let store = SqliteLabs::open(&path, config(10), key(), SystemClock)
            .await
            .unwrap();
        tamper(&path, change).await;
        assert!(
            matches!(
                super::super::transaction::Operation::begin(&store, None).await,
                Err(Error::Configuration)
            ),
            "{change:?}"
        );
        store.close().await;
    }
}
