use std::collections::BTreeMap;

use super::*;

struct EnvironmentGuard {
    original: BTreeMap<&'static str, Option<String>>,
}

impl EnvironmentGuard {
    fn new() -> Self {
        Self {
            original: BTreeMap::new(),
        }
    }

    fn remember(&mut self, key: &'static str) {
        self.original
            .entry(key)
            .or_insert_with(|| std::env::var(key).ok());
    }

    fn set(&mut self, key: &'static str, value: &str) {
        self.remember(key);
        unsafe { std::env::set_var(key, value) };
    }

    fn clear(&mut self, key: &'static str) {
        self.remember(key);
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (key, value) in &self.original {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn clear_provider_environment(environment: &mut EnvironmentGuard) {
    for key in [
        "MAIL_HOST",
        "MAIL_PORT",
        "MAIL_USERNAME",
        "MAIL_PASSWORD",
        "RESEND_API_KEY",
        "SENDGRID_API_KEY",
        "POSTMARK_SERVER_TOKEN",
        "POSTMARK_API_KEY",
        "POSTMARK_MESSAGE_STREAM",
        "AWS_REGION",
        "AWS_SES_ENDPOINT",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_SES_TOKEN",
        "AWS_SES_BEARER_TOKEN",
    ] {
        environment.clear(key);
    }
}

#[tokio::test]
async fn resolves_every_offline_provider_and_rejects_unknown_drivers() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    let message = Message::new()
        .to("offline@example.com")
        .from("team@rullst.dev")
        .subject("resolved provider")
        .text("offline fixture");

    for name in [
        "log", "memory", "resend", "sendgrid", "postmark", "ses", "aws_ses",
    ] {
        environment.set("MAIL_DRIVER", name);
        let driver = Mail::resolve_driver().await.unwrap();
        driver.send(&message).await.unwrap();
    }

    environment.set("MAIL_DRIVER", "smtp");
    environment.set("MAIL_HOST", "127.0.0.1");
    environment.set("MAIL_PORT", "2525");
    let smtp = Mail::resolve_driver().await.unwrap();
    #[cfg(feature = "mail-smtp")]
    smtp.send(&message).await.unwrap();
    #[cfg(not(feature = "mail-smtp"))]
    assert!(smtp.send(&message).await.is_err());

    environment.set("MAIL_DRIVER", "unknown-provider");
    assert!(matches!(
        Mail::resolve_driver().await,
        Err(MailError::ConfigError(message)) if message.contains("unknown-provider")
    ));
}

#[tokio::test]
async fn provider_resolution_fails_closed_for_partial_or_unsafe_ses_configuration() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("MAIL_DRIVER", "ses");

    environment.set("AWS_ACCESS_KEY_ID", "configured-access-key");
    assert!(matches!(
        Mail::resolve_driver().await,
        Err(MailError::ConfigError(message)) if message.contains("requires both")
    ));

    environment.clear("AWS_ACCESS_KEY_ID");
    environment.set("AWS_SECRET_ACCESS_KEY", "configured-secret-key");
    assert!(matches!(
        Mail::resolve_driver().await,
        Err(MailError::ConfigError(message)) if message.contains("requires both")
    ));

    environment.clear("AWS_SECRET_ACCESS_KEY");
    environment.set("AWS_SES_ENDPOINT", "http://mail.example.com/send");
    assert!(Mail::resolve_driver().await.is_err());
}

#[tokio::test]
async fn optional_provider_settings_are_consumed_without_live_requests() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);

    environment.set("MAIL_DRIVER", "postmark");
    environment.set("POSTMARK_MESSAGE_STREAM", "broadcast");
    assert!(Mail::resolve_driver().await.is_ok());

    environment.set("MAIL_DRIVER", "ses");
    environment.set("AWS_REGION", "sa-east-1");
    environment.set("AWS_SES_TOKEN", "mock_ses");
    environment.set(
        "AWS_SES_ENDPOINT",
        "http://127.0.0.1:4566/v2/email/outbound-emails",
    );
    let driver = Mail::resolve_driver().await.unwrap();
    let message = Message::new()
        .to("offline@example.com")
        .subject("endpoint fixture");
    driver.send(&message).await.unwrap();
}

#[test]
fn schedule_conversion_rejects_pre_epoch_values() {
    let epoch = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 123).unwrap();
    assert_eq!(
        datetime_to_system_time(&epoch).unwrap(),
        UNIX_EPOCH + Duration::from_nanos(123)
    );

    let before_epoch = chrono::DateTime::<chrono::Utc>::from_timestamp(-1, 0).unwrap();
    assert!(matches!(
        datetime_to_system_time(&before_epoch),
        Err(MailError::ValidationError(message)) if message.contains("predates")
    ));
}
