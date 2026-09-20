use super::*;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

#[tokio::test]
async fn initialization_configuration_epoch_schema_and_clock_fail_closed() {
    let (temp, store, clock) = fixture().await;
    let path = temp.path().join("supervision.sqlite");
    assert!(matches!(
        SqliteSupervision::initialize(&path, config(), clock.clone()).await,
        Err(Error::Configuration)
    ));
    assert!(
        SqliteSupervision::open(temp.path().join("absent.sqlite"), config(), clock.clone())
            .await
            .is_err()
    );
    assert!(!temp.path().join("absent.sqlite").exists());
    let wrong_epoch = StoreConfig::new(
        "wrong-epoch",
        Limits::new(16, 16, 128, 16).unwrap(),
        3600,
        600,
    )
    .unwrap();
    assert!(matches!(
        SqliteSupervision::open(&path, wrong_epoch, clock.clone()).await,
        Err(Error::Configuration)
    ));
    let wrong_bounds = StoreConfig::new(
        "deployment-v1",
        Limits::new(15, 16, 128, 16).unwrap(),
        3600,
        600,
    )
    .unwrap();
    assert!(matches!(
        SqliteSupervision::open(&path, wrong_bounds, clock.clone()).await,
        Err(Error::Configuration)
    ));
    clock.set(999);
    assert!(matches!(
        store.enroll_parental(&operator(), &scope()).await,
        Err(Error::Clock)
    ));
    assert!(matches!(
        SqliteSupervision::open(&path, config(), clock.clone()).await,
        Err(Error::Clock)
    ));
    clock.set(1000);
    assert!(
        store
            .parental(&context("learner-a"), &scope())
            .await
            .unwrap()
            .is_none()
    );
    store.close().await;
    let mut connection =
        SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    sqlx::query("DROP TABLE rullst_supervision_courses")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    assert!(matches!(
        SqliteSupervision::open(path, config(), clock).await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn capacities_never_evict_active_state_or_reset_event_sequence() {
    let temp = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let limits = Limits::new(1, 1, 1, 1).unwrap().event_budget(1, 1).unwrap();
    let store = SqliteSupervision::initialize(
        temp.path().join("bounded.sqlite"),
        StoreConfig::new("epoch", limits, 3600, 600).unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    let session = store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    store
        .record_visibility(
            &context("learner-a"),
            &scope(),
            session.id(),
            session.revision(),
            1,
            VisibilityEvent::PageHidden,
        )
        .await
        .unwrap();
    clock.set(1001);
    assert!(matches!(
        store
            .record_visibility(
                &context("learner-a"),
                &scope(),
                session.id(),
                session.revision(),
                2,
                VisibilityEvent::PageVisible
            )
            .await,
        Err(Error::Capacity)
    ));
    let second = Scope::new("school-a", "learner-a", "resource-b").unwrap();
    assert!(matches!(
        store
            .start_exam(
                &context("learner-a"),
                &second,
                &policy(),
                &acknowledgement(),
                None
            )
            .await,
        Err(Error::Capacity)
    ));
    store.enroll_parental(&operator(), &scope()).await.unwrap();
    assert!(matches!(
        store.enroll_parental(&operator(), &second).await,
        Err(Error::Capacity)
    ));
    store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    assert!(matches!(
        store
            .provision_authority(
                &operator(),
                &key(AuthorityAction::ParentalManage),
                None,
                2000
            )
            .await,
        Err(Error::Capacity)
    ));
    assert_eq!(
        store.purge_expired(&operator(), 100).await.unwrap().events,
        0
    );
    assert_eq!(
        store
            .session(&context("learner-a"), &scope(), session.id())
            .await
            .unwrap()
            .last_sequence(),
        1
    );
    assert_eq!(
        store
            .learning_access(
                &context("learner-a"),
                &scope(),
                &OpaqueId::new("course-a").unwrap()
            )
            .await
            .unwrap(),
        AccessDecision::MissingPolicy
    );
}

#[tokio::test]
async fn corrupted_rows_are_errors_instead_of_panics_or_authorization() {
    let (temp, store, _) = fixture().await;
    let session = store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("PRAGMA ignore_check_constraints=ON")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("UPDATE rullst_supervision_sessions SET retain_until=? WHERE id=?")
        .bind(i64::MIN)
        .bind(session.id().as_str())
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(matches!(
        store
            .session(&context("learner-a"), &scope(), session.id())
            .await,
        Err(Error::Configuration)
    ));
    store.enroll_parental(&operator(), &scope()).await.unwrap();
    sqlx::query(
        "UPDATE rullst_supervision_managed SET policy_actor='delegate',not_before=?,expires_at=?",
    )
    .bind(i64::MIN)
    .bind(i64::MAX)
    .execute(&mut connection)
    .await
    .unwrap();
    assert!(matches!(
        store
            .learning_access(
                &context("learner-a"),
                &scope(),
                &OpaqueId::new("course-a").unwrap()
            )
            .await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn oversized_stored_identifiers_are_rejected_and_never_returned() {
    let (temp, store, _) = fixture().await;
    let session = store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE rullst_supervision_sessions SET notice=? WHERE id=?")
        .bind("a".repeat(100_000))
        .bind(session.id().as_str())
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(matches!(
        store
            .session(&context("learner-a"), &scope(), session.id())
            .await,
        Err(Error::Configuration)
    ));
    let grant = store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    sqlx::query("UPDATE rullst_supervision_grants SET evidence_ref=?")
        .bind("b".repeat(100_000))
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(matches!(
        store.authority(&operator(), grant.key()).await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn global_event_budget_rejects_without_advancing_another_session() {
    let temp = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let limits = Limits::new(1, 2, 1, 1).unwrap().event_budget(2, 1).unwrap();
    let store = SqliteSupervision::initialize(
        temp.path().join("global.sqlite"),
        StoreConfig::new("epoch", limits, 3600, 600).unwrap(),
        clock,
    )
    .await
    .unwrap();
    let actor = context("learner-a");
    let first = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    store
        .record_visibility(
            &actor,
            &scope(),
            first.id(),
            first.revision(),
            1,
            VisibilityEvent::PageVisible,
        )
        .await
        .unwrap();
    let second_scope = Scope::new("school-a", "learner-a", "second-resource").unwrap();
    let second = store
        .start_exam(&actor, &second_scope, &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert!(matches!(
        store
            .record_visibility(
                &actor,
                &second_scope,
                second.id(),
                second.revision(),
                1,
                VisibilityEvent::PageHidden
            )
            .await,
        Err(Error::Capacity)
    ));
    assert_eq!(
        store
            .session(&actor, &second_scope, second.id())
            .await
            .unwrap()
            .last_sequence(),
        0
    );
}

#[tokio::test]
async fn persisted_authority_and_session_are_usable_from_a_new_process() {
    let (temp, store, _) = fixture().await;
    let grant = store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    let session = store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    store.close().await;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "storage::reopen_in_child_process"])
        .env(
            "RULLST_SUPERVISION_TEST_DB",
            temp.path().join("supervision.sqlite"),
        )
        .env("RULLST_SUPERVISION_TEST_SESSION", session.id().as_str())
        .env(
            "RULLST_SUPERVISION_TEST_GRANT_REVISION",
            grant.revision().value().to_string(),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let reopened = SqliteSupervision::open(
        temp.path().join("supervision.sqlite"),
        config(),
        TestClock::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        reopened
            .session(&context("delegate"), &scope(), session.id())
            .await,
        Err(Error::Forbidden)
    ));
}

#[tokio::test]
#[ignore = "invoked only by the parent-owned process fixture"]
async fn reopen_in_child_process() {
    let path = std::env::var_os("RULLST_SUPERVISION_TEST_DB").unwrap();
    let id = OpaqueId::new(std::env::var("RULLST_SUPERVISION_TEST_SESSION").unwrap()).unwrap();
    let revision = rullst_supervision::Revision::new(
        std::env::var("RULLST_SUPERVISION_TEST_GRANT_REVISION")
            .unwrap()
            .parse()
            .unwrap(),
    )
    .unwrap();
    let store = SqliteSupervision::open(std::path::PathBuf::from(path), config(), TestClock::new())
        .await
        .unwrap();
    assert_eq!(
        store
            .session(&context("delegate"), &scope(), &id)
            .await
            .unwrap()
            .state(),
        SessionState::Active
    );
    store
        .revoke_authority(&operator(), &key(AuthorityAction::ExamReview), revision)
        .await
        .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_state_and_non_private_initial_file_are_not_created() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let (temp, store, clock) = fixture().await;
    let path = temp.path().join("supervision.sqlite");
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = temp.path().join("symlink.sqlite");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        SqliteSupervision::open(&link, config(), clock.clone()).await,
        Err(Error::Configuration)
    ));
    assert!(matches!(
        SqliteSupervision::initialize(link, config(), clock).await,
        Err(Error::Configuration)
    ));
    store.close().await;
}
