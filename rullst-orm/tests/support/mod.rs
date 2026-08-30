#[cfg(not(feature = "strict-sqlite"))]
use std::fmt::Display;

#[cfg(not(feature = "strict-sqlite"))]
const REQUIRE_CONTAINERS_ENV: &str = "RULLST_REQUIRE_TESTCONTAINERS";

#[cfg(not(feature = "strict-sqlite"))]
#[track_caller]
pub fn handle_container_start_error(backend: &str, error: impl Display) {
    let required = std::env::var(REQUIRE_CONTAINERS_ENV)
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        });

    if required {
        panic!(
            "{backend} testcontainer is required by {REQUIRE_CONTAINERS_ENV}, but startup failed: {error}"
        );
    }

    eprintln!(
        "Skipping {backend} matrix test because Docker is unavailable; set {REQUIRE_CONTAINERS_ENV}=true to make this fatal: {error}"
    );
}

pub async fn exercise_outbox() {
    use rullst_orm::{Error, Orm, Outbox};
    use serde_json::json;

    Outbox::install()
        .await
        .expect("outbox DDL should install on the active backend");
    let (inserted, duplicate) = Orm::transaction(|_| {
        Box::pin(async {
            let inserted = Outbox::enqueue(
                "matrix-stream",
                "matrix-event",
                "matrix.created",
                &json!({"matrix": true}),
            )
            .await?;
            let duplicate = Outbox::enqueue(
                "matrix-stream",
                "matrix-event",
                "matrix.created",
                &json!({"matrix": true}),
            )
            .await?;
            Ok::<_, Error>((inserted, duplicate))
        })
    })
    .await
    .expect("outbox enqueue should commit atomically");
    assert!(inserted.inserted);
    assert!(!duplicate.inserted);
    assert_eq!(inserted.id, duplicate.id);

    let rollback = Orm::transaction(|_| {
        Box::pin(async {
            Outbox::enqueue(
                "matrix-rollback",
                "rolled-back-event",
                "matrix.created",
                &json!({"matrix": false}),
            )
            .await?;
            Err::<(), Error>(Error::Validation("force rollback".to_string()))
        })
    })
    .await;
    assert!(rollback.is_err());
    assert!(
        Outbox::claim_next("matrix-rollback", "matrix-worker", 30, 2)
            .await
            .expect("rolled-back stream query should succeed")
            .is_none()
    );

    let claimed = Outbox::claim_next("matrix-stream", "matrix-worker", 30, 2)
        .await
        .expect("outbox claim should execute")
        .expect("committed outbox event should be claimable");
    assert_eq!(claimed.attempts, 1);
    assert!(
        Outbox::acknowledge(claimed.id, &claimed.claim_key)
            .await
            .expect("outbox acknowledgement should execute")
    );
    assert!(
        Outbox::claim_next("matrix-stream", "matrix-worker", 30, 2)
            .await
            .expect("delivered stream query should succeed")
            .is_none()
    );
}
