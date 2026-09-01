#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::*;
use async_trait::async_trait;
use rullst_core::queue::{QueueDriver, QueueError, QueuedJob};

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

#[derive(Default)]
struct CapturedQueue {
    jobs: Arc<Mutex<Vec<CapturedJob>>>,
    failure: Option<QueueError>,
}

type CapturedJob = (String, String, Option<SystemTime>);

#[async_trait]
impl QueueDriver for CapturedQueue {
    async fn push(&self, _id: &str, job_name: &str, payload: &str) -> Result<(), QueueError> {
        if let Some(error) = &self.failure {
            return Err(QueueError::Driver(error.to_string()));
        }
        self.jobs
            .lock()
            .unwrap()
            .push((job_name.to_string(), payload.to_string(), None));
        Ok(())
    }

    async fn push_at(
        &self,
        _id: &str,
        job_name: &str,
        payload: &str,
        available_at: SystemTime,
    ) -> Result<(), QueueError> {
        if let Some(error) = &self.failure {
            return Err(QueueError::Driver(error.to_string()));
        }
        self.jobs.lock().unwrap().push((
            job_name.to_string(),
            payload.to_string(),
            Some(available_at),
        ));
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.jobs.lock().unwrap().len() as u64)
    }
}

fn valid_message() -> Message {
    Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("facade contract")
        .text("bounded body")
}

#[tokio::test]
async fn explicit_queue_preserves_tenant_and_schedule_and_maps_driver_errors() {
    let jobs = Arc::new(Mutex::new(Vec::new()));
    let queue = Queue::custom(Box::new(CapturedQueue {
        jobs: jobs.clone(),
        failure: None,
    }));
    Mail::enqueue(&queue, valid_message()).await.unwrap();

    let scheduled_at = chrono::Utc::now() + chrono::Duration::seconds(30);
    Mail::enqueue_for_tenant(&queue, "tenant_acme", valid_message().send_at(scheduled_at))
        .await
        .unwrap();

    {
        let captured = jobs.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].0, "rullst_mail_send");
        assert!(captured[0].2.is_none());
        let unscoped: QueuedMail = serde_json::from_str(&captured[0].1).unwrap();
        assert_eq!(unscoped.schema_version, MAIL_JOB_SCHEMA_VERSION);
        assert!(unscoped.tenant_id.is_none());

        assert!(captured[1].2.is_some());
        let scoped: QueuedMail = serde_json::from_str(&captured[1].1).unwrap();
        assert_eq!(scoped.tenant_id.as_deref(), Some("tenant_acme"));
    }

    let failing = Queue::custom(Box::new(CapturedQueue {
        jobs: Arc::new(Mutex::new(Vec::new())),
        failure: Some(QueueError::Driver("offline failure".to_string())),
    }));
    assert!(matches!(
        Mail::enqueue(&failing, valid_message()).await,
        Err(MailError::SendError(message)) if message.contains("offline failure")
    ));
}

#[tokio::test]
async fn synchronous_tenant_facade_uses_custom_and_resolved_offline_drivers() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let (driver, store) = MemoryDriver::isolated();
    Mail::set_driver(Box::new(driver));
    Mail::send_now_for_tenant("tenant_acme", valid_message())
        .await
        .unwrap();
    assert_eq!(store.lock().unwrap().len(), 1);
    Mail::reset_driver();

    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("MAIL_DRIVER", "memory");
    Mail::send_now_for_tenant("tenant_globex", valid_message())
        .await
        .unwrap();

    assert!(
        Mail::send_now_for_tenant("../invalid", valid_message())
            .await
            .is_err()
    );
}
