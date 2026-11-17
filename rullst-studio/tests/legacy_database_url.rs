//! The legacy `resolve_db_url` helper reads `DATABASE_URL` and `./Rullst.toml`.
//! Its own test binary, with a single test, because it changes the process
//! environment and working directory.
#![cfg(feature = "local")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_studio::data_browser::db::resolve_db_url;

fn set_database_url(value: &str) {
    // SAFETY: this binary runs a single test, so nothing reads the
    // environment concurrently.
    unsafe { std::env::set_var("DATABASE_URL", value) };
}

#[test]
fn the_legacy_resolver_prefers_the_argument_then_the_environment_then_rullst_toml() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rullst-studio-legacy-url-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).expect("fresh temporary directory");
    std::env::set_current_dir(&directory).expect("enter the temporary directory");

    assert_eq!(resolve_db_url(" sqlite://given.db "), "sqlite://given.db");

    // A blank variable is not a configuration.
    set_database_url("  ");
    assert_eq!(resolve_db_url(""), "sqlite://db.sqlite");
    set_database_url(" postgres://env.example/app ");
    assert_eq!(resolve_db_url(""), "postgres://env.example/app");

    set_database_url("  ");
    std::fs::write("Rullst.toml", "[database]\nurl=\"sqlite://compact.db\"\n")
        .expect("write Rullst.toml");
    assert_eq!(resolve_db_url(""), "sqlite://compact.db");
    // An empty value is skipped in favour of a later one.
    std::fs::write(
        "Rullst.toml",
        "[database]\nurl = \"\"\nurl = 'sqlite://second.db'\n",
    )
    .expect("rewrite Rullst.toml");
    assert_eq!(resolve_db_url(""), "sqlite://second.db");

    std::env::set_current_dir(std::env::temp_dir()).expect("leave the directory");
    std::fs::remove_dir_all(&directory).expect("remove the temporary directory");
}
