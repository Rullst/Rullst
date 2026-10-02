//! Rules for changes to stored data and to rolling deployments from 12.x.

use super::*;

#[test]
fn stored_data_and_rolling_upgrade_apis_are_reviewed() {
    assert_only(
        r#"
use rullst::mail::{Mail, register_mail_handler};
async fn boot(queue: Queue, worker: &mut Worker) -> Result<(), Error> {
    Mail::init_queue(queue.clone());
    register_mail_handler(worker);
    Mail::enqueue_for_tenant(&queue, "acme", message).await?;
    let doc = Doc::get_from_redis(7).await?;
    Doc::increment_redis_field(7, DocColumn::Views, 1).await?;
    let quota = SqlQuotaStore::from_pool(pool, SqlQuotaBackend::Mysql);
    rullst_orm::Outbox::install().await?;
    let memory = SqlChatMemory::connect(url).await?;
    rullst_orm::audit::create_audit_table().await?;
    let chain = AuditChain::new(Arc::new(StdoutAuditLogger));
    Ok(())
}
"#,
        &[
            "V13-MAIL-FACADE-CONFIG",
            "V13-MAIL-QUEUED-ATTACHMENTS",
            "V13-ORM-REDIS-HASHES",
            "V13-CAPITAL-QUOTA-KEYS",
            "V13-OUTBOX-MYSQL-KEYS",
            "V13-AI-CHAT-MEMORY-KEYS",
            "V13-ORM-AUDIT-PAYLOADS",
            "V13-SECURITY-AUDIT-LOG-LINES",
        ],
    );
    for (source, code) in [
        (
            "async fn a() { doc.save_to_redis().await; }",
            "V13-ORM-REDIS-HASHES",
        ),
        (
            "async fn a() { Mail::enqueue(&queue, message).await; }",
            "V13-MAIL-QUEUED-ATTACHMENTS",
        ),
        (
            "fn a(store: rullst::capital::SqlQuotaStore) {}",
            "V13-CAPITAL-QUOTA-KEYS",
        ),
        (
            "fn a() -> Box<dyn Migration> { Box::new(OutboxMigration) }",
            "V13-OUTBOX-MYSQL-KEYS",
        ),
    ] {
        assert!(codes(source).contains(&code), "{source}");
    }
}

#[test]
fn direct_mail_similar_names_comments_and_strings_are_not_data_findings() {
    assert_only(
        r#"
// save_to_redis, Mail::enqueue, SqlQuotaStore and Outbox::install in a comment
const NOTE: &str = "Mail::enqueue SqlChatMemory StdoutAuditLogger orm:posts:1";
async fn send(cache: Cache) {
    Mail::send(message).await;
    Mail::send_now(message).await;
    let outbox = MyOutbox::default();
    queue.enqueue(job).await;
    cache.save("key").await;
}
"#,
        &["V13-MAIL-FACADE-CONFIG"],
    );
}

#[test]
fn audited_models_and_protected_values_are_reviewed() {
    let audited = r#"
#[derive(Orm)]
#[orm(table = "projects", auditable)]
pub struct Project {
    pub id: i32,
    #[orm(masked)]
    pub api_token: String,
}
"#;
    assert_eq!(
        scan_with(audited, "project.rs", &WorkspaceFacts::default()),
        vec![
            ("V13-ORM-AUDIT-PAYLOADS", 3),
            ("V13-ORM-PROTECTED-VALUES", 7)
        ]
    );
    assert_only(
        "#[derive(Serialize)]\npub struct Job {\n    pub token: SecretString,\n}\n",
        &["V13-ORM-PROTECTED-VALUES"],
    );
    assert_only(
        r#"
#[derive(Orm)]
#[orm(table = "projects")]
pub struct Project {
    pub id: i32,
    #[orm(hidden)]
    pub password_hash: String,
}
#[derive(Debug)]
pub struct Holder {
    pub token: SecretString,
}
"#,
        &[],
    );
}
