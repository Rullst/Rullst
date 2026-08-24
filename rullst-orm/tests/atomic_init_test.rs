#![cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]

use rullst_orm::{Error, Orm};

#[tokio::test]
async fn replica_failure_does_not_publish_partial_global_state() {
    let failure = Orm::init_with_replicas(
        "sqlite::memory:",
        vec!["unsupported-replica-scheme://invalid"],
    )
    .await;

    assert!(failure.is_err(), "the invalid replica must fail startup");
    assert!(matches!(Orm::pool(), Err(Error::NotInitialized)));
    assert!(matches!(Orm::read_pool(), Err(Error::NotInitialized)));
    assert!(matches!(Orm::driver(), Err(Error::NotInitialized)));

    Orm::init("sqlite::memory:")
        .await
        .expect("a complete retry must succeed after replica preparation fails");

    assert!(Orm::pool().is_ok());
    assert!(Orm::read_pool().is_ok());
    assert_eq!(Orm::driver().expect("initialized driver"), "sqlite");
}
