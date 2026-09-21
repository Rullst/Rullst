#![cfg(feature = "sqlite")]
use rullst_labs::{sqlite::*, *};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

const NOW: i64 = 1_800_000_000;
#[derive(Clone)]
struct TestClock(Arc<AtomicI64>);
impl Clock for TestClock {
    fn now(&self) -> Result<i64, LabError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct Policy;
impl Authorization for Policy {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, LabError> {
        if actor.as_str() != "teacher"
            || scope != &Scope::new("school", "rust")?
            || action != Action::ManageExercises
        {
            return Err(LabError::Denied);
        }
        Permission::until(NOW + 1000)
    }
}
fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
fn exercise(expected: i64) -> Exercise {
    Exercise::new(
        Scope::new("school", "rust").unwrap(),
        reference("sum"),
        reference("v1"),
        vec![GraderCase {
            id: reference("private-grader-case"),
            input: [1111111111, 2222222222],
            expected,
        }],
        ExecutionLimits::new(30, 100000, 64).unwrap(),
    )
    .unwrap()
}
fn config() -> StoreConfig {
    StoreConfig::new(
        reference("isolated-test"),
        10,
        1,
        ExecutionProfile::Simulation,
    )
    .unwrap()
}

#[tokio::test]
async fn authorized_registry_encrypts_grader_and_preserves_withdrawal_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.sqlite");
    let clock = TestClock(Arc::new(AtomicI64::new(NOW)));
    let store = SqliteLabs::initialize(
        &path,
        config(),
        ContentKey::new([7; 32]).unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    let teacher = reference("teacher");
    let learner = reference("learner");
    let exercise = exercise(9876543210);
    assert_eq!(
        store
            .register_exercise(&Policy, &learner, &exercise)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    store
        .register_exercise(&Policy, &teacher, &exercise)
        .await
        .unwrap();
    assert_eq!(
        store
            .get_exercise(
                &Policy,
                &learner,
                exercise.scope(),
                exercise.id(),
                exercise.revision()
            )
            .await
            .unwrap_err(),
        LabError::Denied
    );
    let other = Scope::new("other-school", "rust").unwrap();
    assert_eq!(
        store
            .get_exercise(
                &Policy,
                &teacher,
                &other,
                exercise.id(),
                exercise.revision()
            )
            .await
            .unwrap_err(),
        LabError::Denied
    );
    let changed = Exercise::new(
        exercise.scope().clone(),
        exercise.id().clone(),
        exercise.revision().clone(),
        vec![GraderCase {
            id: reference("different"),
            input: [0, 0],
            expected: 0,
        }],
        exercise.limits().clone(),
    )
    .unwrap();
    assert_eq!(
        store
            .register_exercise(&Policy, &teacher, &changed)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    store
        .set_exercise_enabled(
            &Policy,
            &teacher,
            exercise.scope(),
            exercise.id(),
            exercise.revision(),
            false,
        )
        .await
        .unwrap();
    store
        .register_exercise(&Policy, &teacher, &exercise)
        .await
        .unwrap();
    assert!(
        !store
            .get_exercise(
                &Policy,
                &teacher,
                exercise.scope(),
                exercise.id(),
                exercise.revision()
            )
            .await
            .unwrap()
            .1
    );
    store.close().await;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for sensitive in [
            b"private-grader-case".as_slice(),
            b"9876543210",
            b"1111111111",
        ] {
            assert!(!bytes.windows(sensitive.len()).any(|w| w == sensitive));
        }
    }
    let store = SqliteLabs::open(&path, config(), ContentKey::new([7; 32]).unwrap(), clock)
        .await
        .unwrap();
    let (restored, enabled) = store
        .get_exercise(
            &Policy,
            &teacher,
            exercise.scope(),
            exercise.id(),
            exercise.revision(),
        )
        .await
        .unwrap();
    assert_eq!(restored, exercise);
    assert!(!enabled);
    store.close().await;
}

#[tokio::test]
async fn wrong_key_ciphertext_tampering_and_clock_rollback_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.sqlite");
    let clock = TestClock(Arc::new(AtomicI64::new(NOW)));
    let store = SqliteLabs::initialize(
        &path,
        config(),
        ContentKey::new([7; 32]).unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    let teacher = reference("teacher");
    let exercise = exercise(42);
    store
        .register_exercise(&Policy, &teacher, &exercise)
        .await
        .unwrap();
    assert!(matches!(
        SqliteLabs::open(
            &path,
            config(),
            ContentKey::new([8; 32]).unwrap(),
            clock.clone()
        )
        .await,
        Err(LabError::Configuration)
    ));
    let mut db = SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&path))
        .await
        .unwrap();
    let mut encrypted: Vec<u8> = sqlx::query_scalar("SELECT content FROM labs_exercises")
        .fetch_one(&mut db)
        .await
        .unwrap();
    encrypted[15] ^= 1;
    sqlx::query("UPDATE labs_exercises SET content=?")
        .bind(encrypted)
        .execute(&mut db)
        .await
        .unwrap();
    assert_eq!(
        store
            .get_exercise(
                &Policy,
                &teacher,
                exercise.scope(),
                exercise.id(),
                exercise.revision()
            )
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    store.close().await;
    clock.0.store(NOW - 1, Ordering::SeqCst);
    assert!(matches!(
        SqliteLabs::open(&path, config(), ContentKey::new([7; 32]).unwrap(), clock).await,
        Err(LabError::Clock)
    ));
    db.close().await.unwrap();
}
