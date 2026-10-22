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

/// SQLx percent-decodes the DSN path, so the prepared file must be the
/// decoded path SQLx opens, never a stray file named after the encoding.
#[test]
fn percent_encoded_dsn_prepares_the_decoded_path() {
    let directory = unique_database_path("encoded dir");
    let database_path = directory.join("app db.sqlite");
    let encoded = format!("{}/app%20db.sqlite", directory.display()).replace(' ', "%20");
    let dsn = format!("sqlite://{encoded}");

    Orm::validate_dsn(&dsn);

    let stray = std::path::PathBuf::from(&encoded);
    assert!(database_path.is_file(), "the decoded path must be prepared");
    assert!(
        !stray.exists(),
        "no file may be created under the encoded name"
    );
    std::fs::remove_dir_all(directory).expect("temporary SQLite directory should be removable");
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

#[test]
fn in_memory_sqlite_pools_keep_their_database_connection() {
    let short = || {
        Orm::pool_options()
            .idle_timeout(Some(std::time::Duration::from_secs(1)))
            .max_lifetime(Some(std::time::Duration::from_secs(2)))
    };
    for dsn in [
        "sqlite::memory:",
        "sqlite://:memory:",
        "sqlite:file:shared?mode=memory&cache=shared",
    ] {
        let options = Orm::retain_memory_database(short(), dsn);
        assert_eq!(options.get_min_connections(), 1, "{dsn}");
        assert_eq!(options.get_idle_timeout(), None, "{dsn}");
        assert_eq!(options.get_max_lifetime(), None, "{dsn}");
    }
    let file = Orm::retain_memory_database(short(), "sqlite:data/app.db");
    assert_eq!(file.get_min_connections(), 0);
    assert_eq!(
        file.get_idle_timeout(),
        Some(std::time::Duration::from_secs(1))
    );
}

/// With the idle reaper active, an in-memory database would be dropped with
/// its last connection; the retained pool keeps its schema and rows.
#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
#[tokio::test]
async fn in_memory_sqlite_schema_survives_the_idle_reaper() {
    sqlx::any::install_default_drivers();
    let options = Orm::pool_options()
        .idle_timeout(Some(std::time::Duration::from_millis(500)))
        .max_lifetime(Some(std::time::Duration::from_secs(1)));
    let pool = Orm::retain_memory_database(options, "sqlite::memory:")
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory pool");
    sqlx::query("CREATE TABLE retained (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .expect("create table");
    tokio::time::sleep(std::time::Duration::from_millis(2_500)).await;
    let (rows,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM retained")
        .fetch_one(&pool)
        .await
        .expect("the in-memory schema must survive idle periods");
    assert_eq!(rows, 0);
    pool.close().await;
}

/// A pool of the compiled driver that never connects.
fn lazy_pool() -> crate::RullstPool {
    #[cfg(not(any(
        feature = "strict-postgres",
        feature = "strict-mysql",
        feature = "strict-sqlite"
    )))]
    sqlx::any::install_default_drivers();
    let url = if cfg!(feature = "strict-postgres") {
        "postgres://rullst@127.0.0.1:9/never"
    } else if cfg!(feature = "strict-mysql") {
        "mysql://rullst@127.0.0.1:9/never"
    } else {
        "sqlite::memory:"
    };
    Orm::pool_options()
        .connect_lazy(url)
        .expect("a lazy pool never connects")
}

#[tokio::test]
async fn read_pool_rotates_over_the_replicas_and_falls_back_to_the_primary() {
    let state = super::OrmState::new(lazy_pool(), "sqlite", vec![lazy_pool(), lazy_pool()]);
    let picks: Vec<*const crate::RullstPool> = (0..5)
        .map(|_| std::ptr::from_ref(state.read_pool()))
        .collect();
    let first = std::ptr::from_ref(&state.replicas[0]);
    let second = std::ptr::from_ref(&state.replicas[1]);
    assert_eq!(picks, [first, second, first, second, first]);

    let primary_only = super::OrmState::new(lazy_pool(), "sqlite", Vec::new());
    assert!(std::ptr::eq(
        primary_only.read_pool(),
        &primary_only.primary
    ));
}

#[test]
fn pool_options_report_slow_acquisitions_after_half_a_second() {
    assert_eq!(
        Orm::pool_options().get_acquire_slow_threshold(),
        std::time::Duration::from_millis(500)
    );
}
