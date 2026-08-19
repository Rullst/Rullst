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
}
