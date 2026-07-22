#![cfg(feature = "mailer")]

use rullst::mail::{Mail, MailDriver, MailError, Message, ResendDriver, SendGridDriver};

#[tokio::test]
async fn test_mail_resolution_flow() {
    // 1. Test log driver resolution and send
    unsafe {
        std::env::set_var("MAIL_DRIVER", "log");
    }
    let msg = Message::new()
        .to("test@example.com")
        .subject("Test Facade")
        .text("hello");
    let res = Mail::send(msg.clone()).await;
    assert!(res.is_ok());

    // 2. Test unknown driver
    unsafe {
        std::env::set_var("MAIL_DRIVER", "unknown_xyz");
    }
    let res_unknown = Mail::send(Message::new()).await;
    assert!(res_unknown.is_err());
    if let Err(MailError::ConfigError(e)) = res_unknown {
        assert!(e.contains("Unknown mail driver"));
    }

    // 3. Test SMTP driver resolution (feature not enabled)
    unsafe {
        std::env::set_var("MAIL_DRIVER", "smtp");
    }
    let smtp_res = Mail::send(Message::new()).await;
    assert!(smtp_res.is_err());

    // 4. Test Resend driver config error (missing env var)
    unsafe {
        std::env::set_var("MAIL_DRIVER", "resend");
        std::env::remove_var("RESEND_API_KEY");
    }
    let resend_res = Mail::send(Message::new()).await;
    assert!(resend_res.is_err());

    // 5. Test SendGrid driver config error (missing env var)
    unsafe {
        std::env::set_var("MAIL_DRIVER", "sendgrid");
        std::env::remove_var("SENDGRID_API_KEY");
    }
    let sendgrid_res = Mail::send(Message::new()).await;
    assert!(sendgrid_res.is_err());

    // 6. Test resolve driver from TOML
    unsafe {
        std::env::remove_var("MAIL_DRIVER");
    }
    let has_original_toml = std::path::Path::new("Rullst.toml").exists();
    if has_original_toml {
        let _ = std::fs::rename("Rullst.toml", "Rullst.toml.bak");
    }

    // Write a dummy Rullst.toml with driver = "log"
    let dummy_toml = "[mail]\ndriver = \"log\"\n";
    let _ = std::fs::write("Rullst.toml", dummy_toml);

    let res_toml = Mail::send(msg).await;
    assert!(res_toml.is_ok());

    // Clean up
    let _ = std::fs::remove_file("Rullst.toml");
    if has_original_toml {
        let _ = std::fs::rename("Rullst.toml.bak", "Rullst.toml");
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_resend_driver_send_mock() {
    let driver = ResendDriver {
        api_key: "dummy_key".to_string(),
    };
    let msg = Message::new()
        .to("recipient@example.com")
        .subject("Hi")
        .text("plain text")
        .html("<p>html</p>");

    // This should fail due to invalid key, but it will execute the request logic
    let res = driver.send(&msg).await;
    assert!(res.is_err());
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_sendgrid_driver_send_mock() {
    let driver = SendGridDriver {
        api_key: "dummy_key".to_string(),
    };
    let msg = Message::new()
        .to("recipient@example.com")
        .subject("Hi")
        .text("plain text")
        .html("<p>html</p>");

    // This should fail due to invalid key, but it will execute the request logic
    let res = driver.send(&msg).await;
    assert!(res.is_err());
}
