// Realtime-notification preference regression appended to the Academy journey.

pub const GENERATED_NOTIFICATION_REALTIME_TESTS_SUFFIX: &str = r##"
        let realtime_opt_out = set_preference(&learner, 7, "realtime", false, "en")
            .await
            .expect("realtime notification opt-out");
        assert!(realtime_opt_out.applied);
        let mut muted_realtime = subscribe_in_app(&learner, 7)
            .await
            .expect("muted tenant-scoped subscription");
        rullst::db::sqlx::query(
            "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, 0, 100000, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(1_i32)
        .bind("achievement:realtime-muted-1")
        .bind("achievement_awarded")
        .bind(7_i32)
        .bind(serde_json::json!({
            "schema_version": 1,
            "actor_user_id": 7,
            "subject_user_id": 7,
            "achievement_code": "memory-guardian",
            "execution_key": "automation:realtime-muted-1",
        }).to_string())
        .bind("processing")
        .bind(1_i32)
        .bind("worker-realtime-muted")
        .bind("claim-realtime-muted")
        .bind("")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("muted realtime outbox fixture");
        let muted_receipt =
            crate::services::notification_service::deliver_claimed_achievement(
                "achievement:realtime-muted-1",
                "claim-realtime-muted",
            )
            .await
            .expect("database notification survives realtime opt-out");
        assert!(muted_receipt.applied);
        let muted_event_id = rullst::db::sqlx::query_scalar::<_, i32>(
            "SELECT id FROM academy_outbox WHERE event_key = ?",
        )
        .bind("achievement:realtime-muted-1")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("muted realtime event id");
        assert!(acknowledge(muted_event_id, "claim-realtime-muted")
            .await
            .expect("muted realtime event ACK"));
        assert!(matches!(
            muted_realtime.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
"##;
