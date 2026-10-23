//! `TursoOrm::init_from_env` persists the offline store of a mock URL.
#![cfg(feature = "turso")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::polyglot::TursoOrm;

#[tokio::test]
async fn a_mock_url_persists_the_offline_store_at_its_path() {
    let path = std::env::temp_dir().join(format!(
        "rullst-turso-env-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    // This is the only test in its process, so no other thread reads these
    // variables while they change.
    unsafe {
        std::env::set_var("TURSO_DATABASE_URL", "mock_turso");
        std::env::set_var("TURSO_OFFLINE_PATH", &path);
        std::env::remove_var("TURSO_AUTH_TOKEN");
    }
    TursoOrm::init_from_env()
        .await
        .expect("offline initialization");
    assert!(TursoOrm::store().unwrap().is_offline());
    assert!(
        path.exists(),
        "the offline database is a file at TURSO_OFFLINE_PATH"
    );
    let _ = std::fs::remove_file(path);
}
