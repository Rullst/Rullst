//! # `rullst-mail` — High-Performance Transactional Email & Mailables Engine
//!
//! Provides zero-cost abstraction for transactional emails with built-in:
//! - **RFC 8058 One-Click List-Unsubscribe** headers
//! - **Automatic Plain-Text Fallback** derivation
//! - **In-Memory MailTrap & Fluent Assertions**
//! - **Outbound DLP Secret Scanner** (AWS keys, passwords, bearer tokens)
//! - Multiple delivery drivers (**SMTP**, **Resend**, **SendGrid**, **Log**, **Memory**)

pub mod drivers;
pub mod facade;
pub mod message;
pub mod worker;

pub use drivers::*;
pub use facade::*;
pub use message::*;
pub use worker::*;

/// Backwards compatibility alias for `rullst_mail::mail::*`
pub mod mail {
    pub use crate::*;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_message_subject() {
        let msg = Message::new().subject("Test Subject");
        assert_eq!(msg.subject, "Test Subject");

        let msg2 = Message::new().subject(String::from("Another Subject"));
        assert_eq!(msg2.subject, "Another Subject");
    }

    #[test]
    fn test_message_to() {
        let msg = Message::new().to("user@example.com");
        assert_eq!(msg.to, "user@example.com");
    }

    #[tokio::test]
    async fn test_mail_custom() {
        let msg = Message::new()
            .to("a")
            .from("b")
            .subject("c")
            .text("d")
            .html("e");
        assert_eq!(msg.to, "a");
        assert_eq!(msg.from.unwrap(), "b");
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
        assert!(clean.contains("Bearer jwt_..."));
        assert!(!clean.contains("MySecret123"));

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
        let driver = ResendDriver {
            api_key: "test".to_string(),
        };
        let msg = Message::new().to("test@rullst.dev");
        let res = driver.send(&msg).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_sendgrid_driver() {
        let driver = SendGridDriver {
            api_key: "test".to_string(),
        };
        let msg = Message::new().to("test@rullst.dev");
        let res = driver.send(&msg).await;
        assert!(res.is_err());
    }

    #[cfg(feature = "mail-smtp")]
    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_smtp_driver() {
        let driver = SmtpDriver {
            host: "invalid.local".to_string(),
            port: 25,
            username: None,
            password: None,
        };
        let msg = Message::new().to("test@rullst.dev").subject("Test");
        let res = driver.send(&msg).await;
        assert!(res.is_err());
    }

    #[cfg(not(feature = "mail-smtp"))]
    #[tokio::test]
    async fn test_smtp_driver_disabled() {
        let driver = SmtpDriver;
        let msg = Message::new().to("test@rullst.dev").subject("Test");
        let res = driver.send(&msg).await;
        assert!(res.is_err());
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_message_builder_recipient() {
        let msg = Message::new().to("user@rullst.dev").subject("Hello");
        assert!(!msg.to.is_empty());
        assert!(!msg.subject.is_empty());
    }
}
