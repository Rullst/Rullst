// Final outbox-delivery assertions and closure for the materialized Academy test.

#[path = "academy_rollback_tests.rs"]
mod academy_rollback_tests;
pub(crate) use academy_rollback_tests::GENERATED_ROLLBACK_TESTS_SUFFIX;

pub const GENERATED_COMPLETION_TESTS_SUFFIX: &str = r##"
        let expected_completion_key = format!(
            "course-completed:22:{}",
            scheduler_draft.id,
        );
        let expected_revocation_key =
            "certificate-revoked:certificate-revoke-22".to_string();
        let mut completion_delivered = false;
        let mut revocation_delivered = false;
        for attempt in 0_i64..20 {
            let pending = rullst::db::sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM academy_outbox WHERE status = ?",
            )
            .bind("pending")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("pending domain events");
            if pending == 0 {
                break;
            }
            let outcome = run_once_at(
                "worker-completion",
                &format!("claim-completion-{attempt}"),
                80_000 + attempt,
                30,
                3,
                5,
            )
            .await
            .expect("completion journey worker");
            let AutomationWorkerOutcome::Delivered { event_key, .. } = outcome else {
                panic!("completion journey event was not delivered");
            };
            completion_delivered |= event_key == expected_completion_key;
            revocation_delivered |= event_key == expected_revocation_key;
        }
        assert!(completion_delivered);
        assert!(revocation_delivered);
        let remaining = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE status = ?",
        )
        .bind("pending")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("all domain events delivered");
        assert_eq!(remaining, 0);
    }
}
"##;
