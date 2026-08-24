use rullst_mail::mail::*;

#[tokio::test]
async fn test_message_builder_and_log_driver() {
    let msg = Message::new()
        .to("recipient@example.com")
        .from("team@rullst.dev")
        .subject("Welcome to Rullst!")
        .html("<h1>Welcome</h1><p>Enjoy blazing fast apps.</p>")
        .text("Welcome! Enjoy blazing fast apps.");

    assert_eq!(msg.to, "recipient@example.com");
    assert_eq!(msg.from.as_deref(), Some("team@rullst.dev"));
    assert_eq!(msg.subject, "Welcome to Rullst!");
    assert!(msg.body_html.as_ref().unwrap().contains("<h1>Welcome</h1>"));
    assert!(msg.body_text.as_ref().unwrap().contains("Enjoy"));

    let driver = LogDriver;
    let res = driver.send(&msg).await;
    assert!(res.is_ok());
}

#[test]
fn test_mail_error_formatting() {
    let err1 = MailError::ConfigError("Missing API key".to_string());
    assert!(format!("{}", err1).contains("Configuration error: Missing API key"));

    let err2 = MailError::SendError("Connection dropped".to_string());
    assert!(format!("{}", err2).contains("Send error: Connection dropped"));

    let err3 = MailError::DriverError("DNS resolution failed".to_string());
    assert!(format!("{}", err3).contains("Driver error: DNS resolution failed"));
}

#[test]
fn test_driver_constructors() {
    let resend = ResendDriver {
        api_key: "re_123456789".to_string(),
    };
    assert_eq!(resend.api_key, "re_123456789");

    let sendgrid = SendGridDriver {
        api_key: "SG.123456".to_string(),
    };
    assert_eq!(sendgrid.api_key, "SG.123456");

    let postmark = PostmarkDriver {
        server_token: "pm_tok_123".to_string(),
        message_stream: Some("outbound".to_string()),
    };
    assert_eq!(postmark.server_token, "pm_tok_123");

    let ses = AwsSesDriver::try_new("us-east-1", "mock_ses_token").unwrap();
    assert_eq!(ses.region, "us-east-1");
    assert_eq!(ses.auth_token, "mock_ses_token");
    assert!(ses.endpoint().contains("us-east-1.amazonaws.com"));
}

#[tokio::test]
async fn test_memory_driver_and_message_features() {
    MailTrap::clear();
    let mem = MemoryDriver::new();

    let msg = Message::new()
        .to("customer@acme.com")
        .from("billing@rullst.com")
        .subject("Your Monthly Invoice")
        .html("<p>Please find invoice attached.</p>")
        .attach_bytes(
            "invoice.pdf",
            b"PDF_CONTENT_BYTES".to_vec(),
            "application/pdf",
        )
        .unsubscribe_url("https://rullst.com/unsubscribe/123")
        .unsubscribe_email("unsub@rullst.com");

    let res = mem.send(&msg).await;
    assert!(res.is_ok());

    let header = msg.list_unsubscribe_header();
    assert!(header.is_some());
    assert!(
        header
            .unwrap()
            .contains("https://rullst.com/unsubscribe/123")
    );

    MailTrap::assert_sent_to("customer@acme.com");
    let sent = MailTrap::sent_messages();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].subject, "Your Monthly Invoice");
    assert_eq!(sent[0].attachments.len(), 1);
    assert_eq!(sent[0].attachments[0].filename, "invoice.pdf");
}

#[test]
fn test_email_security_and_validator() {
    let clean_html = "<a href=\"https://rullst.dev/docs\">Documentation</a>";
    assert!(scan_content_security(clean_html).is_ok());

    let dangerous_html = "<a href=\"javascript:alert('pwned')\">Click</a>";
    assert!(scan_content_security(dangerous_html).is_err());

    assert!(validate_email_syntax("valid.user@example.com").is_ok());
    assert!(validate_email_syntax("invalid-email-string").is_err());
    assert!(is_disposable_email("temp@10minutemail.com"));
    assert!(!is_disposable_email("alice@gmail.com"));
}
