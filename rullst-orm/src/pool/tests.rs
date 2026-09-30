use super::Orm;

fn unique_database_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rullst-orm-{label}-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ))
}

#[test]
fn named_memory_dsn_does_not_touch_a_backing_file() {
    let database_path = unique_database_path("named-memory");
    let dsn = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        database_path.display()
    );

    Orm::validate_dsn(&dsn);

    assert!(!database_path.exists());
}

#[tokio::test]
async fn every_initializer_rejects_placeholder_dsns_before_connecting() {
    let placeholder = "postgres://app:[YOUR-PASSWORD]@db.invalid/app";
    assert!(matches!(
        Orm::init_with_options(placeholder, 5, 5).await,
        Err(crate::Error::Internal(_))
    ));
    assert!(matches!(
        Orm::init_with_replicas(placeholder, Vec::new()).await,
        Err(crate::Error::Internal(_))
    ));
    assert!(matches!(
        Orm::init_with_replicas("sqlite::memory:", vec![placeholder]).await,
        Err(crate::Error::Internal(_))
    ));
    assert!(matches!(Orm::try_pool(), Err(crate::Error::NotInitialized)));
}

#[tokio::test]
async fn pool_options_outside_their_bounds_fail_without_panicking() {
    for (max_connections, acquire_timeout_secs) in [(5, u64::MAX), (5, 0), (0, 5)] {
        assert!(matches!(
            Orm::init_with_options("sqlite::memory:", max_connections, acquire_timeout_secs).await,
            Err(crate::Error::Validation(_))
        ));
    }
    assert!(matches!(Orm::try_pool(), Err(crate::Error::NotInitialized)));
}

#[test]
fn disk_dsn_still_prepares_the_backing_file() {
    for query in ["", "?mode=rwc", "?cache=shared&MODE=RWC"] {
        let database_path = unique_database_path("disk");
        let dsn = format!("sqlite:{}{query}", database_path.display());

        Orm::validate_dsn(&dsn);

        assert!(database_path.is_file(), "{query}");
        std::fs::remove_file(database_path).expect("temporary SQLite file should be removable");
    }
}

#[test]
fn read_only_and_read_write_dsns_never_create_a_missing_database() {
    for mode in ["ro", "rw", "RW"] {
        let directory = unique_database_path("missing-directory");
        let database_path = directory.join("app.db");
        let dsn = format!("sqlite://{}?mode={mode}", database_path.display());

        Orm::validate_dsn(&dsn);

        assert!(!directory.exists(), "mode={mode} created the directory");
        assert!(!database_path.exists(), "mode={mode} created the database");
    }
}
