//! A build without a SQLx driver feature (the default since 13.0.0-alpha.3)
//! fails `Orm::init` with an error that names the feature to enable. The
//! SQLite driver is not checked: the SeaORM and Diesel comparison
//! dev-dependencies compile SQLx's SQLite driver into every test build.
#![cfg(not(any(
    feature = "drivers-all",
    feature = "strict-sqlite",
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "turso"
)))]

use rullst_orm::{Error, Orm};

#[tokio::test]
async fn init_without_a_driver_names_the_feature_to_enable() {
    for (url, scheme, feature) in [
        ("postgres://app@127.0.0.1:1/app", "postgres", "strict-postgres"),
        ("mysql://app@127.0.0.1:1/app", "mysql", "strict-mysql"),
        ("MariaDB://app@127.0.0.1:1/app", "MariaDB", "strict-mysql"),
    ] {
        let error = Orm::init(url).await.expect_err("no driver is compiled in");
        assert!(
            matches!(&error, Error::Validation(message)
                if message.contains(&format!("`{scheme}:` URLs"))
                    && message.contains(&format!("`{feature}`"))),
            "{url}: {error}"
        );
    }
    // A failed initialization leaves the ORM uninitialized.
    assert!(matches!(Orm::driver(), Err(Error::NotInitialized)));
}
