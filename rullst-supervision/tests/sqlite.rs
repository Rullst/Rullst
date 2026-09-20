#![cfg(feature = "sqlite")]

use rullst_supervision::{
    AuthorityAction, AuthorityKey, Clock, Context, Limits, OpaqueId, Operator, Scope, StoreConfig,
    SupervisionError as Error,
    exam::{Acknowledgement, ExamPolicy, SessionState, VisibilityEvent},
    parental::{AccessDecision, CoursePolicy},
    sqlite::SqliteSupervision,
};
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicUsize, Ordering},
};

#[derive(Clone)]
struct TestClock {
    time: Arc<AtomicI64>,
    calls: Arc<AtomicUsize>,
}
impl TestClock {
    fn new() -> Self {
        Self {
            time: Arc::new(AtomicI64::new(1000)),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn set(&self, value: i64) {
        self.time.store(value, Ordering::SeqCst);
    }
}
impl Clock for TestClock {
    fn now(&self) -> Result<i64, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.time.load(Ordering::SeqCst))
    }
}

fn config() -> StoreConfig {
    StoreConfig::new(
        "deployment-v1",
        Limits::new(16, 16, 128, 16).unwrap(),
        3600,
        600,
    )
    .unwrap()
}
fn context(actor: &str) -> Context {
    Context::new("school-a", actor).unwrap()
}
fn scope() -> Scope {
    Scope::new("school-a", "learner-a", "resource-a").unwrap()
}
fn operator() -> Operator {
    Operator::new(context("operator"), "checked-relationship-reference").unwrap()
}
fn acknowledgement() -> Acknowledgement {
    Acknowledgement::new("policy-v1", "notice-v1", true).unwrap()
}
fn policy() -> ExamPolicy {
    ExamPolicy::new("policy-v1", "notice-v1", 300).unwrap()
}
fn key(action: AuthorityAction) -> AuthorityKey {
    AuthorityKey::new(scope(), "delegate", action).unwrap()
}

async fn fixture() -> (tempfile::TempDir, SqliteSupervision<TestClock>, TestClock) {
    let temp = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let store = SqliteSupervision::initialize(
        temp.path().join("supervision.sqlite"),
        config(),
        clock.clone(),
    )
    .await
    .unwrap();
    (temp, store, clock)
}

#[path = "sqlite_cases/concurrency.rs"]
mod concurrency;
#[path = "sqlite_cases/exams.rs"]
mod exams;
#[path = "sqlite_cases/parental.rs"]
mod parental;
#[path = "sqlite_cases/storage.rs"]
mod storage;
