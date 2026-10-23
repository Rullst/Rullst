//! Revision loading refuses invalid IDs and anything but version-2 updates.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{
    AuditContext, create_audit_table, load_restorable_revision_with_tx, with_audit_context,
};
use rullst_orm::{Error, Orm};

fn validation(result: Result<rullst_orm::audit::RestorableRevision, Error>) -> String {
    match result {
        Err(Error::Validation(message)) => message,
        Err(other) => panic!("unexpected error: {other:?}"),
        Ok(_) => panic!("the revision must be refused"),
    }
}

#[tokio::test]
async fn only_version_two_update_revisions_with_positive_ids_load() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-audit-restore-guard-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize isolated SQLite ORM");
    create_audit_table().await.expect("audit table");
    for (event, version) in [("created", 2), ("updated", 1), ("updated", 2)] {
        sqlx::query(
            "INSERT INTO rullst_audits (model_type, model_id, event, format_version, restore_patch) \
             VALUES ('Note', 7, ?, ?, 'patch')",
        )
        .bind(event)
        .bind(version)
        .execute(Orm::pool().unwrap())
        .await
        .expect("insert revision");
    }

    let context = AuditContext::system("restorer").expect("context");
    with_audit_context(context, async {
        let mut tx = Orm::begin_transaction().await.expect("transaction");
        for (audit_id, model_id) in [(0, 7), (1, 0)] {
            assert_eq!(
                validation(
                    load_restorable_revision_with_tx(&mut tx, audit_id, "Note", model_id).await
                ),
                "audit and model IDs must be positive"
            );
        }
        for audit_id in [1, 2] {
            assert_eq!(
                validation(load_restorable_revision_with_tx(&mut tx, audit_id, "Note", 7).await),
                "only version-2 update revisions can be restored"
            );
        }
        let revision = load_restorable_revision_with_tx(&mut tx, 3, "Note", 7)
            .await
            .expect("a version-2 update revision");
        assert_eq!(revision.restore_patch(), "patch");
        tx.rollback().await.expect("rollback");
    })
    .await;

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
