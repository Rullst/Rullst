#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

#[tokio::test]
async fn sqlite_data_browser_mutations_are_typed_scoped_and_fail_closed() {
    support::exercise_mutations("sqlite::memory:", "sqlite", "studio_mutation_sqlite").await;
}
