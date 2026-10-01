use std::fmt::Display;

#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
pub mod json_columns;
pub mod migrations;

const REQUIRE_CONTAINERS_ENV: &str = "RULLST_REQUIRE_TESTCONTAINERS";

#[allow(dead_code)]
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

#[allow(dead_code)]
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

    // Streams and event keys are case-sensitive on every backend, including
    // MySQL/MariaDB whose default collations fold case.
    let (lower, upper) = Orm::transaction(|_| {
        Box::pin(async {
            let lower = Outbox::enqueue(
                "matrix-case",
                "order:aB3x:created",
                "matrix.created",
                &json!({"id": "aB3x"}),
            )
            .await?;
            let upper = Outbox::enqueue(
                "matrix-case",
                "order:Ab3X:created",
                "matrix.created",
                &json!({"id": "Ab3X"}),
            )
            .await?;
            Ok::<_, Error>((lower, upper))
        })
    })
    .await
    .expect("keys that differ only in case should both enqueue");
    assert!(lower.inserted && upper.inserted);
    assert_ne!(lower.id, upper.id);
    assert!(
        Outbox::claim_next("MATRIX-CASE", "matrix-worker", 30, 2)
            .await
            .expect("case-distinct stream query should succeed")
            .is_none()
    );
    Outbox::install()
        .await
        .expect("reinstalling the outbox should be idempotent");
    exercise_outbox_snapshot_reuse().await;
}

/// A transaction whose read snapshot predates a concurrently committed
/// enqueue of the same key reuses that event (InnoDB REPEATABLE READ would
/// otherwise hide the row from a plain SELECT).
///
/// The scenario needs a server that commits another transaction's write while
/// this one keeps its read snapshot open. SQLite serializes writers against
/// open readers instead: the shared-cache `sqlite::memory:` pool makes the
/// other connection's INSERT wait for `late` to finish, which it never would,
/// so SQLite skips it. The timeout turns any such self-deadlock into a failure.
#[allow(dead_code)]
async fn exercise_outbox_snapshot_reuse() {
    use rullst_orm::{Error, Orm, Outbox};
    use serde_json::json;

    if Orm::driver().expect("initialized driver") == "sqlite" {
        return;
    }
    let scenario = async {
        let payload = json!({"snapshot": true});
        let mut late = Orm::begin_transaction()
            .await
            .expect("begin the late transaction");
        let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_outbox")
            .fetch_one(&mut *late)
            .await
            .expect("establish the late transaction's read snapshot");
        let committed_payload = payload.clone();
        let committed = Orm::transaction(move |_| {
            Box::pin(async move {
                Outbox::enqueue(
                    "matrix-snapshot",
                    "snapshot-event",
                    "matrix.created",
                    &committed_payload,
                )
                .await
            })
        })
        .await
        .expect("the first enqueue commits");
        assert!(committed.inserted);

        let reused = Outbox::enqueue_with_tx(
            &mut late,
            "matrix-snapshot",
            "snapshot-event",
            "matrix.created",
            &payload,
        )
        .await
        .unwrap_or_else(|error: Error| panic!("the committed key must be reused: {error}"));
        assert!(!reused.inserted);
        assert_eq!(reused.id, committed.id);
        late.commit().await.expect("commit the late transaction");
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), scenario)
        .await
        .expect("the outbox snapshot scenario must not wait on its own open transaction");
}

/// Audit payloads larger than MySQL/MariaDB's 64 KiB `TEXT` are stored
/// intact, both as values and as a revision's restore patch.
#[allow(dead_code)]
pub async fn exercise_large_audit_payload() {
    use rullst_orm::Orm;
    use rullst_orm::audit::{
        AuditContext, create_audit_table, log_audit, log_audit_diff, with_audit_context,
    };
    use serde_json::json;

    create_audit_table()
        .await
        .expect("audit table should be created or upgraded");
    let created = json!({"body": "a".repeat(70_000)}).to_string();
    let before = json!({"body": "b".repeat(40_000)}).to_string();
    let after = json!({"body": "c".repeat(40_000)}).to_string();
    let context = AuditContext::system("large-audit-matrix").expect("valid audit context");
    with_audit_context(context, async {
        log_audit("large_payloads", 1, "created", None, Some(created)).await?;
        log_audit_diff("large_payloads", 1, "updated", &before, &after).await
    })
    .await
    .expect("large audit payloads should be stored");

    let sql = if Orm::driver().expect("driver") == "postgres" {
        "SELECT new_values, restore_patch FROM rullst_audits WHERE model_type = $1 ORDER BY id"
    } else {
        "SELECT new_values, restore_patch FROM rullst_audits WHERE model_type = ? ORDER BY id"
    };
    let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(sql)
        .bind("large_payloads")
        .fetch_all(Orm::pool().expect("pool"))
        .await
        .expect("read large audit rows");
    assert_eq!(rows.len(), 2);
    let created: serde_json::Value =
        serde_json::from_str(rows[0].0.as_deref().expect("created values"))
            .expect("created values stay valid JSON");
    assert_eq!(created["body"].as_str().map(str::len), Some(70_000));
    let patch = rows[1].1.as_deref().expect("update restore patch");
    assert!(patch.len() > 80_000, "restore patch length {}", patch.len());
    serde_json::from_str::<serde_json::Value>(patch).expect("restore patch stays valid JSON");
}
