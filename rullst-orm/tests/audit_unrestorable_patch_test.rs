#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::audit::{AuditContext, create_audit_table, with_audit_context};
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "audit_large_documents", auditable)]
struct Document {
    pub id: i32,
    pub body: String,
}

/// A revision whose reverse patch exceeds its bound is recorded without a
/// restore patch; the audited update itself still commits.
#[tokio::test]
async fn oversized_restore_patches_do_not_reject_the_audited_update() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-audit-unrestorable-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Schema::create("audit_large_documents", |table: &mut Blueprint| {
        table.id();
        // SQLite does not enforce the VARCHAR length.
        table.string("body").not_null();
    })
    .await
    .expect("create audited document table");
    create_audit_table().await.expect("create audit table");

    let context = AuditContext::system("large-revision-test").expect("valid context");
    with_audit_context(context, async {
        let mut document = Document {
            id: 0,
            body: "a".repeat(3 * 1024 * 1024),
        };
        document.save().await.expect("create large document");
        // Each side stays under the 5 MiB payload bound; the reverse patch,
        // which holds both, does not.
        document.body = "b".repeat(3 * 1024 * 1024);
        document
            .save()
            .await
            .expect("an unrestorable revision must not reject the update");

        let pool = Orm::pool().expect("ORM pool");
        let stored: (String,) = sqlx::query_as("SELECT body FROM audit_large_documents WHERE id = ?")
            .bind(document.id)
            .fetch_one(pool)
            .await
            .expect("read updated document");
        assert!(stored.0.starts_with('b'));
        let (audit_id, new_values, restore_patch): (i32, Option<String>, Option<String>) =
            sqlx::query_as(
                "SELECT id, new_values, restore_patch FROM rullst_audits WHERE model_type = ? AND event = 'updated'",
            )
            .bind("audit_large_documents")
            .fetch_one(pool)
            .await
            .expect("read update audit");
        assert!(new_values.is_some_and(|values| values.contains("bbbb")));
        assert_eq!(restore_patch, None);

        let refused = document
            .restore_revision(audit_id, "restore an unrestorable revision")
            .await;
        assert!(matches!(refused, Err(rullst_orm::Error::Validation(_))));
    })
    .await;

    let _ = std::fs::remove_file(database_path);
}
