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

/// Removed Capital providers, payouts and NFS-e types point to their row; the
/// providers that v13 keeps do not.
#[test]
fn removed_capital_providers_and_nfse_are_reviewed() {
    let source = r#"
use rullst::capital::{PaddleProvider, StripeProvider, WiseProvider, init_payout_provider};
use rullst::capital::fiscal::{FiscalEmitter, NfseNationalClient};
fn boot() {
    rullst::capital::init_provider(Box::new(PaddleProvider::new("key", "secret")));
    init_payout_provider(Box::new(WiseProvider::new("token", "profile")));
    let dps = invoice.to_dps("01.07.01", "3550308", 0.05);
}
"#;
    // A review rule reports its first location in each file.
    assert_eq!(
        scan_with(source, "billing.rs", &WorkspaceFacts::default()),
        vec![("V13-CAPITAL-REMOVED", 2)]
    );
    for removed in [
        "use rullst::capital::fiscal::NfseNationalClient;",
        "fn a() { init_payout_provider(Box::new(payouts)); }",
        "fn b(invoice: Invoice) { let _ = invoice.to_dps(\"01.07.01\", \"3550308\", 0.05); }",
        "fn c(event: rullst::capital::PaddleSubscriptionEvent) {}",
        "fn d() -> Result<(), FiscalError> { Ok(()) }",
    ] {
        assert_only(removed, &["V13-CAPITAL-REMOVED"]);
    }

    assert_only(
        r#"
use rullst::capital::{InfinitePayProvider, StripeProvider};
fn boot() {
    rullst::capital::init_provider(Box::new(StripeProvider::new("key", "secret")));
    let experimental = InfinitePayProvider::new("key", "secret");
}
"#,
        &[],
    );
}

/// Removed mail transports, their builder and their settings point to their
/// row; the kept transports and the in-memory `MailTrap` do not.
#[test]
fn removed_mail_transports_are_reviewed() {
    let source = r#"
use rullst::mail::{MailDriver, MailError, PostmarkDriver, ResendDriver, SendGridDriver};
fn boot() -> Result<Box<dyn MailDriver>, MailError> {
    Ok(Box::new(SendGridDriver::try_new("key")?))
}
"#;
    // A review rule reports its first location in each file.
    assert_eq!(
        scan_with(source, "mail.rs", &WorkspaceFacts::default()),
        vec![("V13-MAIL-REMOVED", 2), ("V13-MAIL-RESEND-SCHEDULE", 2)]
    );
    for removed in [
        "use rullst::mail::drivers::azure::AzureManagedIdentity;",
        "fn a() { let _ = PostmarkDriver::new(token).with_message_stream(\"outbound\"); }",
        "fn b() -> Result<MailjetDriver, Error> { MailjetDriver::try_new(key, secret) }",
        "fn c() { let _ = rullst::mail::MailtrapDriver::sandbox(token, 42); }",
        "fn d(credential: impl AzureMailCredential) {}",
        "fn e() { unsafe { std::env::set_var(\"MAIL_DRIVER\", \"azure-acs\") } }",
        "fn f() { let _ = std::env::var(\"SENDGRID_API_KEY\"); }",
        "const ENV: &str = \"MAIL_FROM=ops@example.com\\nMAIL_DRIVER=mailjet-sandbox\\n\";",
    ] {
        assert_only(removed, &["V13-MAIL-REMOVED"]);
    }

    assert_only(
        r#"
use rullst::mail::{AwsSesDriver, MailTrap, SendPulseDriver, SmtpDriver, SuppressionEvent};
fn boot() {
    unsafe { std::env::set_var("MAIL_DRIVER", "sendpulse") };
    let ses = AwsSesDriver::try_new("us-east-1", "token");
    MailTrap::assert_nothing_sent();
    // A provider label of a stored event is data, not a driver selection.
    let event = SuppressionEvent::try_new("postmark", "evt", "a@example.com", reason, now);
}
"#,
        &[],
    );
}
