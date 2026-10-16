#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn dotenv(value: Option<&str>) -> HashMap<String, String> {
    value
        .map(|value| HashMap::from([(AUTO_MIGRATE_VARIABLE.to_string(), value.to_string())]))
        .unwrap_or_default()
}

#[test]
fn startup_migrations_are_off_unless_enabled() {
    assert_eq!(AUTO_MIGRATE_VARIABLE, "RULLST_AUTO_MIGRATE");
    assert!(!auto_migrate_enabled(None, None, &dotenv(None)).unwrap());
    assert!(auto_migrate_enabled(Some(true), None, &dotenv(None)).unwrap());
    assert!(!auto_migrate_enabled(Some(false), None, &dotenv(None)).unwrap());
}

#[test]
fn the_variable_overrides_the_builder_and_the_process_overrides_dotenv() {
    let process = |value: &str| Some(value.to_string());
    assert!(!auto_migrate_enabled(Some(true), process("false"), &dotenv(None)).unwrap());
    assert!(auto_migrate_enabled(Some(false), process("true"), &dotenv(None)).unwrap());
    assert!(!auto_migrate_enabled(Some(true), None, &dotenv(Some("off"))).unwrap());
    assert!(auto_migrate_enabled(None, None, &dotenv(Some("on"))).unwrap());
    assert!(!auto_migrate_enabled(None, process("0"), &dotenv(Some("1"))).unwrap());
    assert!(auto_migrate_enabled(None, process("1"), &dotenv(Some("0"))).unwrap());
}

#[test]
fn switch_values_are_explicit() {
    for value in ["1", "true", "TRUE", " yes ", "On"] {
        assert!(parse_switch(value).unwrap(), "{value}");
    }
    for value in ["0", "false", "False", "no", " OFF "] {
        assert!(!parse_switch(value).unwrap(), "{value}");
    }
    for value in ["", "enabled", "2", "truee"] {
        let error = parse_switch(value).unwrap_err();
        assert!(matches!(error, ServerError::Configuration(_)), "{value}");
        assert_eq!(
            error.to_string(),
            "server configuration error: RULLST_AUTO_MIGRATE must be true or false"
        );
    }
}

#[cfg(feature = "orm")]
#[tokio::test]
async fn the_sqlite_lock_serializes_runners_and_skips_memory_databases() {
    assert!(orm::lock_sqlite_file(None).await.unwrap().is_none());
    assert!(
        orm::lock_sqlite_file(Some(String::new()))
            .await
            .unwrap()
            .is_none()
    );

    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("app.db").to_string_lossy().into_owned();
    let held = orm::lock_sqlite_file(Some(database.clone()))
        .await
        .unwrap()
        .expect("a file lock");
    assert!(dir.path().join("app.db.rullst-migrate.lock").exists());

    let waiter = tokio::spawn(orm::lock_sqlite_file(Some(database)));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(!waiter.is_finished(), "a second runner waits for the lock");
    drop(held);
    let second = tokio::time::timeout(std::time::Duration::from_secs(10), waiter)
        .await
        .expect("the lock is taken once released")
        .unwrap()
        .unwrap();
    assert!(second.is_some());

    let missing = dir.path().join("missing").join("app.db");
    let error = orm::lock_sqlite_file(Some(missing.to_string_lossy().into_owned()))
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "startup migrations failed: the SQLite migration lock could not be taken: entity not found"
    );
}
