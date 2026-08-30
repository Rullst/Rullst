#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use rullst_core::security::TenantMembership;

struct AlwaysFailDriver;

#[async_trait::async_trait]
impl MailDriver for AlwaysFailDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(MailError::DriverError(
            "intentional test failure".to_string(),
        ))
    }
}

#[test]
fn test_message_subject() {
    let msg = Message::new().subject("Test Subject");
    assert_eq!(msg.subject, "Test Subject");

    let msg2 = Message::new().subject(String::from("Another Subject"));
    assert_eq!(msg2.subject, "Another Subject");
}

#[tokio::test]
async fn test_mail_html() {
    let msg = Message::new().html("h");
    assert_eq!(msg.body_html.unwrap(), "h");
}

#[tokio::test]
async fn test_mail_subject() {
    let msg = Message::new().subject("sub");
    assert_eq!(msg.subject, "sub");
}

#[tokio::test]
async fn test_mail_to() {
    let msg = Message::new().to("to");
    assert_eq!(msg.to, "to");
}

#[tokio::test]
async fn test_mail_from() {
    let msg = Message::new().from("from");
    assert_eq!(msg.from.unwrap(), "from");
}

#[tokio::test]
async fn test_mail_text() {
    let msg = Message::new().text("txt");
    assert_eq!(msg.body_text.unwrap(), "txt");
}

#[tokio::test]
async fn test_log_driver() {
    let log_path = "storage/logs/mail.log";
    let log_dir = std::path::Path::new(log_path).parent().unwrap();
    let _ = tokio::fs::create_dir_all(log_dir).await;

    let msg = Message::new()
        .to("test@rullst.dev")
        .subject("Hello Test")
        .text("Testing 1 2 3")
        .html("<h1>Testing 1 2 3</h1>");

    let driver = LogDriver;
    let res = driver.send(&msg).await;
    assert!(res.is_ok());

    let mut content = String::new();
    for _ in 0..10 {
        content = std::fs::read_to_string(log_path).unwrap_or_default();
        if content.contains("To: test@rullst.dev") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(content.contains("To: test@rullst.dev"));
    assert!(content.contains("Subject: Hello Test"));
    assert!(content.contains("Testing 1 2 3"));
}

#[tokio::test]
async fn test_mail_send_facade() {
    unsafe {
        std::env::set_var("MAIL_DRIVER", "log");
    }
    let log_path = "storage/logs/mail.log";
    let msg = Message::new().to("facade@rullst.dev").subject("Facade");

    let res = Mail::send(msg).await;
    assert!(res.is_ok());

    let mut content = String::new();
    for _ in 0..10 {
        content = std::fs::read_to_string(log_path).unwrap_or_default();
        if content.contains("facade@rullst.dev") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(content.contains("facade@rullst.dev"));
}

#[test]
fn test_rfc8058_unsubscribe_headers() {
    let msg_both = Message::new()
        .unsubscribe_url("https://example.com/unsub/123")
        .unsubscribe_email("unsub@example.com");
    assert_eq!(
        msg_both.list_unsubscribe_header().unwrap(),
        "<mailto:unsub@example.com>, <https://example.com/unsub/123>"
    );

    let msg_url = Message::new().unsubscribe_url("https://example.com/unsub/123");
    assert_eq!(
        msg_url.list_unsubscribe_header().unwrap(),
        "<https://example.com/unsub/123>"
    );

    let msg_email = Message::new().unsubscribe_email("unsub@example.com");
    assert_eq!(
        msg_email.list_unsubscribe_header().unwrap(),
        "<mailto:unsub@example.com>"
    );

    let msg_none = Message::new();
    assert!(msg_none.list_unsubscribe_header().is_none());
}

#[test]
fn test_strip_html_to_plain_text() {
    let html = "<h1>Welcome</h1><p>Hello <b>Alice</b>! Click <a href=\"https://rullst.dev\">here</a>.</p><ul><li>Item 1</li><li>Item 2</li></ul><style>.secret { display: none; }</style>";
    let plain = strip_html_to_plain_text(html);
    assert!(plain.contains("Welcome"));
    assert!(plain.contains("Hello Alice!"));
    assert!(plain.contains("Item 1"));
    assert!(plain.contains("Item 2"));
    assert!(!plain.contains("style"));
    assert!(!plain.contains("<h1>"));

    let empty = strip_html_to_plain_text("");
    assert_eq!(empty, "");
}

#[test]
fn test_redact_email_secrets() {
    let raw = "Config: password=MySecret123&api_key=key_456 with AWS AKIA1234567890123456 and Bearer jwt_secret_token_12345";
    let clean = redact_email_secrets(raw);
    assert!(clean.contains("password=[REDACTED]"));
    assert!(clean.contains("api_key=[REDACTED]"));
    assert!(clean.contains("AKIA****************"));
    assert!(clean.contains("Bearer [REDACTED]"));
    assert!(!clean.contains("MySecret123"));

    let repeated = redact_email_secrets(
        "password=first password=second AKIA1234567890123456 AKIAABCDEFGHIJKLMNOP",
    );
    assert_eq!(repeated.matches("password=[REDACTED]").count(), 2);
    assert!(!repeated.contains("AKIA1234567890123456"));
    assert!(!repeated.contains("AKIAABCDEFGHIJKLMNOP"));

    let private_key = redact_email_secrets(
        "-----BEGIN PRIVATE KEY-----\nsecret-material\n-----END PRIVATE KEY-----",
    );
    assert_eq!(private_key, "[REDACTED PRIVATE KEY]");

    let msg = Message::new()
        .subject("Alert for key=my_secret_token")
        .html("<p>Your password=super_secret_pwd</p>")
        .sanitize_secrets();
    assert!(msg.subject.contains("key=[REDACTED]"));
    assert!(msg.body_html.unwrap().contains("password=[REDACTED]"));
}

#[tokio::test]
async fn test_mail_trap_and_assertions() {
    MailTrap::clear();
    MailTrap::assert_nothing_sent();

    let (isolated_driver, store) = MemoryDriver::isolated();
    let msg = Message::new()
        .to("bob@example.com")
        .from("team@rullst.dev")
        .subject("Welcome to Rullst!")
        .html("<p>Thanks for joining!</p>")
        .unsubscribe_url("https://rullst.dev/unsub/bob");

    let res = isolated_driver.send(&msg).await;
    assert!(res.is_ok());
    assert_eq!(store.lock().unwrap().len(), 1);

    // Test Global MailTrap
    Mail::set_driver(Box::new(MailTrap::driver()));
    let sent_res = Mail::send_now(msg.clone()).await;
    assert!(sent_res.is_ok());

    assert_eq!(MailTrap::count(), 1);
    MailTrap::assert_sent_to("bob@example.com")
        .with_subject("Welcome to Rullst!")
        .with_subject_contains("Welcome")
        .with_body_contains("Thanks for joining")
        .with_from("team@rullst.dev")
        .with_unsubscribe_url("https://rullst.dev/unsub/bob");

    assert!(MailTrap::last_message().is_some());
    MailTrap::clear();
    assert_eq!(MailTrap::count(), 0);
    Mail::reset_driver();
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_resend_driver() {
    let driver = ResendDriver::try_new("mock_resend").unwrap();
    let msg = Message::new()
        .to("test@rullst.dev")
        .subject("offline-resend");
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
    assert_eq!(driver.delivery_mode(), DeliveryMode::OfflineMock);
    assert!(
        OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|item| { item.provider == "resend" && item.message.subject == "offline-resend" })
    );
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_sendgrid_driver() {
    let driver = SendGridDriver::try_new("").unwrap();
    let msg = Message::new()
        .to("test@rullst.dev")
        .subject("offline-sendgrid");
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
    assert_eq!(driver.delivery_mode(), DeliveryMode::OfflineMock);
    assert!(
        OfflineMailMock::deliveries().unwrap().iter().any(|item| {
            item.provider == "sendgrid" && item.message.subject == "offline-sendgrid"
        })
    );
}

#[cfg(feature = "mail-smtp")]
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_smtp_driver() {
    let driver = SmtpDriver::try_new("smtp.example.com", 587, None, None).unwrap();
    let msg = Message::new().to("test@rullst.dev").subject("offline-smtp");
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
    assert_eq!(driver.delivery_mode(), DeliveryMode::OfflineMock);
    assert!(
        OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|item| { item.provider == "smtp" && item.message.subject == "offline-smtp" })
    );
}

#[cfg(not(feature = "mail-smtp"))]
#[tokio::test]
async fn test_smtp_driver_disabled() {
    let driver = SmtpDriver;
    let msg = Message::new().to("test@rullst.dev").subject("Test");
    let res = driver.send(&msg).await;
    assert!(res.is_err());
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_postmark_driver() {
    let driver = PostmarkDriver::try_new("mock_postmark")
        .unwrap()
        .with_message_stream("outbound");
    assert_eq!(driver.server_token, "mock_postmark");
    assert_eq!(driver.message_stream.as_deref(), Some("outbound"));

    let msg = Message::new()
        .to("test@rullst.dev")
        .subject("Hello Postmark")
        .html("<p>Postmark Test</p>")
        .unsubscribe_url("https://rullst.dev/unsub");
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
    assert!(
        OfflineMailMock::deliveries().unwrap().iter().any(|item| {
            item.provider == "postmark" && item.message.subject == "Hello Postmark"
        })
    );
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_aws_ses_driver() {
    let driver = AwsSesDriver::try_new("sa-east-1", "mock_ses")
        .unwrap()
        .try_with_endpoint("http://127.0.0.1:4566/v2/email/outbound-emails")
        .unwrap();
    assert_eq!(driver.region, "sa-east-1");
    assert_eq!(
        driver.endpoint(),
        "http://127.0.0.1:4566/v2/email/outbound-emails"
    );

    let default_driver = AwsSesDriver::try_new("us-east-1", "mock_default").unwrap();
    assert_eq!(
        default_driver.endpoint(),
        "https://email.us-east-1.amazonaws.com/v2/email/outbound-emails"
    );

    let msg = Message::new()
        .to("test@rullst.dev")
        .subject("Hello SES")
        .text("SES Plain Text")
        .unsubscribe_email("unsub@rullst.dev");
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
    assert!(
        OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|item| { item.provider == "aws_ses" && item.message.subject == "Hello SES" })
    );
}

#[tokio::test]
async fn test_failover_driver_success_primary() {
    let (primary_driver, primary_store) = MemoryDriver::isolated();
    let (fallback_driver, fallback_store) = MemoryDriver::isolated();

    let failover = FailoverDriver::new(primary_driver)
        .with_fallback(fallback_driver)
        .with_threshold(2);

    assert_eq!(failover.fallback_count(), 1);
    assert_eq!(failover.failure_count(), 0);
    assert!(!failover.is_tripped());

    let msg = Message::new()
        .to("user@example.com")
        .subject("Failover Test");
    let res = failover.send(&msg).await;
    assert!(res.is_ok());

    assert_eq!(primary_store.lock().unwrap().len(), 1);
    assert_eq!(fallback_store.lock().unwrap().len(), 0);
    assert_eq!(failover.failure_count(), 0);
}

#[tokio::test]
async fn test_failover_driver_fallback_on_primary_failure() {
    let primary = AlwaysFailDriver;
    let (fallback_driver, fallback_store) = MemoryDriver::isolated();

    let failover = FailoverDriver::new(primary)
        .with_fallback(fallback_driver)
        .with_threshold(3);

    let msg = Message::new()
        .to("fallback-user@example.com")
        .subject("Fallback Verification");
    let res = failover.send(&msg).await;

    assert!(res.is_ok());
    assert_eq!(failover.failure_count(), 1);
    assert_eq!(fallback_store.lock().unwrap().len(), 1);
    assert_eq!(
        fallback_store.lock().unwrap()[0].to,
        "fallback-user@example.com"
    );
}

#[tokio::test]
async fn test_failover_driver_circuit_breaker_tripping() {
    let failing_primary = AlwaysFailDriver;
    let (fallback_driver, fallback_store) = MemoryDriver::isolated();

    let failover = FailoverDriver::new(failing_primary)
        .with_fallback(fallback_driver)
        .with_threshold(2)
        .with_cooldown(std::time::Duration::from_secs(10));

    let msg = Message::new()
        .to("user@example.com")
        .subject("Circuit Breaker");

    // First failure -> count = 1, not tripped yet
    let _ = failover.send(&msg).await;
    assert_eq!(failover.failure_count(), 1);
    assert!(!failover.is_tripped());

    // Second failure -> count = 2, threshold reached -> tripped!
    let _ = failover.send(&msg).await;
    assert_eq!(failover.failure_count(), 2);
    assert!(failover.is_tripped());

    // Third dispatch -> skips primary directly because circuit is tripped
    let res = failover.send(&msg).await;
    assert!(res.is_ok());
    assert_eq!(fallback_store.lock().unwrap().len(), 3);

    // Manual reset
    failover.reset_circuit();
    assert!(!failover.is_tripped());
    assert_eq!(failover.failure_count(), 0);
}

#[tokio::test]
async fn test_failover_driver_all_fail() {
    let failing_primary = AlwaysFailDriver;
    let failing_fallback = AlwaysFailDriver;

    let failover = FailoverDriver::new(failing_primary).with_fallback(failing_fallback);

    let msg = Message::new().to("user@example.com");
    let res = failover.send(&msg).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("All mail drivers in failover chain failed"));
}

#[tokio::test]
async fn test_tenant_mail_resolver() {
    let (tenant_a_driver, tenant_a_store) = MemoryDriver::isolated();
    let (tenant_b_driver, tenant_b_store) = MemoryDriver::isolated();
    let (default_driver, default_store) = MemoryDriver::isolated();

    let resolver = TenantMailResolver::with_default(default_driver);
    resolver
        .register("tenant_acme", tenant_a_driver)
        .expect("tenant A registration");
    resolver
        .register("tenant_globex", tenant_b_driver)
        .expect("tenant B registration");

    assert_eq!(resolver.tenant_count().expect("tenant count"), 2);
    assert!(resolver.has_tenant("tenant_acme").expect("tenant A lookup"));
    assert!(
        resolver
            .has_tenant("tenant_globex")
            .expect("tenant B lookup")
    );
    assert!(
        !resolver
            .has_tenant("tenant_unknown")
            .expect("unknown tenant lookup")
    );

    // 1. Send for tenant A
    let msg_a = Message::new().to("admin@acme.com").subject("Acme Invoice");
    let res_a = resolver.send_for_tenant("tenant_acme", &msg_a).await;
    assert!(res_a.is_ok());
    assert_eq!(tenant_a_store.lock().unwrap().len(), 1);
    assert_eq!(tenant_b_store.lock().unwrap().len(), 0);

    // 2. Send for tenant B
    let msg_b = Message::new()
        .to("admin@globex.com")
        .subject("Globex Alert");
    let res_b = resolver.send_for_tenant("tenant_globex", &msg_b).await;
    assert!(res_b.is_ok());
    assert_eq!(tenant_a_store.lock().unwrap().len(), 1);
    assert_eq!(tenant_b_store.lock().unwrap().len(), 1);

    // 3. Send for unknown tenant -> routes to default driver
    let msg_unknown = Message::new().to("user@other.com").subject("Default Route");
    let res_unk = resolver
        .send_for_tenant("tenant_unknown", &msg_unknown)
        .await;
    assert!(res_unk.is_ok());
    assert_eq!(default_store.lock().unwrap().len(), 1);

    // 4. Send via generic MailDriver trait implementation -> dispatches via default
    let generic_msg = Message::new().to("system@rullst.dev").subject("Generic");
    let res_gen = resolver.send(&generic_msg).await;
    assert!(res_gen.is_ok());
    assert_eq!(default_store.lock().unwrap().len(), 2);

    // 5. The trait-object path used by Mail::send_for_tenant preserves tenant routing.
    let dynamic_driver: &dyn MailDriver = &resolver;
    let tenant_msg = Message::new()
        .to("dynamic@globex.com")
        .subject("Dynamic tenant route");
    dynamic_driver
        .send_for_tenant("tenant_globex", &tenant_msg)
        .await
        .unwrap();
    assert_eq!(tenant_b_store.lock().unwrap().len(), 2);
    assert!(
        dynamic_driver
            .send_for_tenant("../spoofed", &tenant_msg)
            .await
            .is_err()
    );

    // 6. Remove tenant
    let removed = resolver.remove("tenant_acme").expect("tenant removal");
    assert!(removed.is_some());
    assert_eq!(resolver.tenant_count().expect("tenant count"), 1);
    assert!(!resolver.has_tenant("tenant_acme").expect("tenant lookup"));
}

#[tokio::test]
async fn tenant_context_selects_credentials_without_cross_tenant_delivery() {
    let membership =
        TenantMembership::try_new(["acme:prod", "globex.eu"]).expect("authenticated membership");
    let acme_context = membership.select("acme:prod").expect("Acme membership");
    let globex_context = membership.select("globex.eu").expect("Globex membership");
    let (acme_driver, acme_store) = MemoryDriver::isolated();
    let (globex_driver, globex_store) = MemoryDriver::isolated();
    let resolver = TenantMailResolver::new();

    resolver
        .register_for_context(&acme_context, acme_driver)
        .expect("Acme registration");
    resolver
        .register_for_context(&globex_context, globex_driver)
        .expect("Globex registration");

    let acme_message = Message::new().to("owner@acme.example").subject("Acme only");
    let globex_message = Message::new()
        .to("owner@globex.example")
        .subject("Globex only");
    let (acme_result, globex_result) = tokio::join!(
        resolver.send_for_context(&acme_context, &acme_message),
        resolver.send_for_context(&globex_context, &globex_message),
    );

    acme_result.expect("Acme delivery");
    globex_result.expect("Globex delivery");
    assert_eq!(acme_store.lock().expect("Acme store").len(), 1);
    assert_eq!(globex_store.lock().expect("Globex store").len(), 1);

    assert!(resolver.register("../spoofed", LogDriver).is_err());
}

#[path = "lib_feature_tests.rs"]
mod feature_tests;
