use rullst_labs::{sqlite::*, *};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

pub const NOW: i64 = 1_800_000_000;
pub const SOURCE: &str = "pub fn solve(a:i64,b:i64)->i64 { a+b } // learner-secret-source";
#[derive(Clone)]
pub struct TestClock(pub Arc<AtomicI64>);
impl Clock for TestClock {
    fn now(&self) -> Result<i64, LabError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
#[derive(Clone)]
pub struct Policy(pub Arc<AtomicI64>);
impl Authorization for Policy {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, LabError> {
        if scope != &self::scope() || !matches!(actor.as_str(), "teacher" | "alice" | "bob") {
            return Err(LabError::Denied);
        }
        if matches!(action, Action::ManageJobs | Action::ManageExercises)
            && actor.as_str() != "teacher"
        {
            return Err(LabError::Denied);
        }
        Permission::until(self.0.load(Ordering::SeqCst))
    }
}
pub fn id(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
pub fn scope() -> Scope {
    Scope::new("school", "rust").unwrap()
}
pub fn exercise() -> Exercise {
    Exercise::new(
        scope(),
        id("sum"),
        id("v1"),
        vec![GraderCase {
            id: id("private-case"),
            input: [123, 456],
            expected: 579,
        }],
        ExecutionLimits::new(10, 100000, 64).unwrap(),
    )
    .unwrap()
}
pub fn submission(name: &str) -> Submission {
    Submission::new(
        id(name),
        ExerciseRef::new("sum", "v1").unwrap(),
        RustSource::new(SOURCE).unwrap(),
        300,
    )
    .unwrap()
}
pub fn config(max_jobs: u32) -> StoreConfig {
    StoreConfig::new(id("queue-tests"), max_jobs, 4, ExecutionProfile::Simulation).unwrap()
}
pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub store: SqliteLabs<TestClock>,
    pub clock: TestClock,
    pub policy: Policy,
}
impl Fixture {
    pub async fn new(max_jobs: u32) -> Self {
        Self::with_profile(max_jobs, ExecutionProfile::Simulation).await
    }
    pub async fn with_profile(max_jobs: u32, profile: ExecutionProfile) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let clock = TestClock(Arc::new(AtomicI64::new(NOW)));
        let policy = Policy(Arc::new(AtomicI64::new(NOW + 1000)));
        let store = SqliteLabs::initialize(
            dir.path().join("jobs.sqlite"),
            if profile == ExecutionProfile::Simulation {
                config(max_jobs)
            } else {
                StoreConfig::new(id("queue-tests"), max_jobs, 4, profile).unwrap()
            },
            ContentKey::new([9; 32]).unwrap(),
            clock.clone(),
        )
        .await
        .unwrap();
        store
            .register_exercise(&policy, &id("teacher"), &exercise())
            .await
            .unwrap();
        Self {
            dir,
            store,
            clock,
            policy,
        }
    }
    pub async fn database(&self) -> SqliteConnection {
        SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(self.dir.path().join("jobs.sqlite")),
        )
        .await
        .unwrap()
    }
}
