#![cfg(feature = "webhook-sql")]

mod stripe_inbox_support;

use rullst_capital::{SqlWebhookBackend, StripeInboxError, StripeInboxOutcome, StripeInboxScope};
use stripe_inbox_support::{inbox, pool, snapshot, verify};

#[tokio::test]
async fn sqlite_inbox_commits_domain_and_outcome_or_allows_retry() {
    let backend = SqlWebhookBackend::Sqlite;
    if !stripe_inbox_support::supports_backend(backend) {
        assert!(
            std::env::var_os("RULLST_REQUIRE_SQLITE_INBOX").is_none(),
            "portable gate must use an Any or SQLite pool"
        );
        eprintln!(
            "SQLite inbox uses the dedicated portable CI step when the workspace selects another native pool"
        );
        return;
    }
    let path = std::env::temp_dir().join(format!(
        "rullst-stripe-inbox-{}-{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let url = format!("sqlite:{}?mode=rwc", path.to_string_lossy());
    let db = pool(&url).await;
    stripe_inbox_support::exercise(&db, backend).await;

    // A failure writing the inbox itself must also roll back prior domain SQL.
    let store = inbox(&db, backend, "insert_failure", 2);
    store.prepare_schema().await.unwrap();
    rullst_orm::sqlx::query("CREATE TRIGGER reject_inbox BEFORE INSERT ON rullst_stripe_inbox_v1 BEGIN SELECT RAISE(ABORT, 'injected inbox failure'); END")
        .execute(&db).await.unwrap();
    let event = verify(&snapshot("evt_insert_failure"), false);
    let failed = store
        .process(&event, |tx, _| {
            Box::pin(async {
                rullst_orm::sqlx::query(
                    "INSERT INTO inbox_domain_effects (event_name) VALUES ('inbox-fault')",
                )
                .execute(&mut **tx)
                .await
                .unwrap();
                Ok(StripeInboxOutcome::Applied)
            })
        })
        .await;
    assert_eq!(failed, Err(StripeInboxError::StorageUnavailable));
    let count: i64 = rullst_orm::sqlx::query_scalar(
        "SELECT COUNT(*) FROM inbox_domain_effects WHERE event_name = 'inbox-fault'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(count, 0);
    rullst_orm::sqlx::query("DROP TRIGGER reject_inbox")
        .execute(&db)
        .await
        .unwrap();
    let retried = store
        .process(&event, |tx, _| {
            Box::pin(async {
                rullst_orm::sqlx::query(
                    "INSERT INTO inbox_domain_effects (event_name) VALUES ('inbox-fault')",
                )
                .execute(&mut **tx)
                .await
                .unwrap();
                Ok(StripeInboxOutcome::Applied)
            })
        })
        .await
        .unwrap();
    assert!(!retried.is_duplicate());
    db.close().await;
    // A fresh connection proves persistence after the original pool closes.
    let db = pool(&url).await;
    let restored = inbox(&db, backend, "insert_failure", 2);
    assert!(
        restored
            .process(&event, |_, _| Box::pin(async {
                panic!("restart must not repeat domain SQL")
            }))
            .await
            .unwrap()
            .is_duplicate()
    );
    db.close().await;
    std::fs::remove_file(path).unwrap();
}

#[test]
fn scope_rejects_unbounded_input_and_redacts_configuration() {
    for invalid in ["", "spaces here", "a/b", "bad\nname"] {
        assert!(StripeInboxScope::platform(invalid, "acct_valid", false).is_err());
    }
    for account in ["", "cus_wrong", "acct_", "acct_bad/name"] {
        assert!(StripeInboxScope::connected("valid", account, false).is_err());
    }
    assert!(StripeInboxScope::platform("x".repeat(201), "acct_valid", false).is_err());
    let scope = StripeInboxScope::platform("private_namespace", "acct_private", true).unwrap();
    let debug = format!("{scope:?}");
    assert!(!debug.contains("private"));
}
