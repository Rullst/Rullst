//! Reuse of the facade's configured driver across messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::tests::{EnvironmentGuard, clear_provider_environment};
use super::*;
use crate::error::MailError;
use crate::facade::{MAIL_ENV_LOCK, Mail};
use crate::message::Message;

#[tokio::test]
async fn a_changed_setting_builds_a_new_driver() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("MAIL_DRIVER", "resend");
    environment.set("RESEND_API_KEY", "mock_first");
    Mail::reset_driver();
    let settings = || async { MailSettings::load().await.unwrap() };

    let first = configured_driver(&settings().await).unwrap();
    let again = configured_driver(&settings().await).unwrap();
    assert!(Arc::ptr_eq(&first, &again));

    environment.set("RESEND_API_KEY", "mock_second");
    let rotated = configured_driver(&settings().await).unwrap();
    assert!(!Arc::ptr_eq(&first, &rotated));

    // A setting that becomes invalid fails instead of reusing the old driver,
    // and so does a driver that v13 removed.
    environment.set("MAIL_DRIVER", "ses");
    environment.set("AWS_ACCESS_KEY_ID", "configured-access-key");
    assert!(configured_driver(&settings().await).is_err());
    environment.set("MAIL_DRIVER", "sendgrid");
    assert!(configured_driver(&settings().await).is_err());

    // Memory drivers are never shared between messages.
    environment.set("MAIL_DRIVER", "memory");
    let memory = configured_driver(&settings().await).unwrap();
    assert!(!Arc::ptr_eq(
        &memory,
        &configured_driver(&settings().await).unwrap()
    ));

    environment.set("MAIL_DRIVER", "resend");
    let before_reset = configured_driver(&settings().await).unwrap();
    Mail::reset_driver();
    assert!(!Arc::ptr_eq(
        &before_reset,
        &configured_driver(&settings().await).unwrap()
    ));
    Mail::reset_driver();
}

#[tokio::test]
async fn both_aws_keys_select_native_ses_credentials() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("MAIL_DRIVER", "ses");
    environment.set("AWS_ACCESS_KEY_ID", "configured-access-key");
    environment.set("AWS_SECRET_ACCESS_KEY", "configured-secret-key");
    let spec = DriverSpec::from_settings(&MailSettings::load().await.unwrap());
    #[cfg(feature = "aws-ses")]
    assert!(matches!(
        spec,
        Ok(DriverSpec::Ses {
            credentials: SesCredentials::Native { .. },
            ..
        })
    ));
    #[cfg(not(feature = "aws-ses"))]
    assert!(spec.is_err());

    // Without any AWS key the HTTPS proxy token is used.
    environment.clear("AWS_ACCESS_KEY_ID");
    environment.clear("AWS_SECRET_ACCESS_KEY");
    environment.set("AWS_SES_TOKEN", "mock_proxy_token");
    let spec = DriverSpec::from_settings(&MailSettings::load().await.unwrap());
    assert!(matches!(
        spec,
        Ok(DriverSpec::Ses {
            credentials: SesCredentials::Proxy(_),
            ..
        })
    ));
}

/// The settings a facade send would read now.
async fn spec() -> Result<DriverSpec, MailError> {
    DriverSpec::from_settings(&MailSettings::load().await.unwrap())
}

fn names_the_setting(
    spec: Result<DriverSpec, MailError>,
    setting: &str,
    environment: &str,
) -> bool {
    matches!(spec, Err(MailError::ConfigError(text))
        if text.contains(setting) && text.contains(environment) && text.contains("mock_"))
}

#[tokio::test]
async fn staging_and_production_refuse_an_empty_provider_credential() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    for (driver, setting) in [
        ("resend", "RESEND_API_KEY"),
        ("sendpulse", "SENDPULSE_API_KEY"),
        ("ses", "AWS_SES_BEARER_TOKEN"),
    ] {
        environment.set("MAIL_DRIVER", driver);
        for name in ["production", "staging"] {
            environment.set("RULLST_ENV", name);
            environment.clear(setting);
            assert!(
                names_the_setting(spec().await, setting, name),
                "{driver} {name}"
            );
            environment.set(setting, " ");
            assert!(
                names_the_setting(spec().await, setting, name),
                "{driver} {name}"
            );
            // An explicit mock is deliberate, and a real secret is used.
            environment.set(setting, "mock_fixture");
            assert!(spec().await.is_ok(), "{driver} {name}");
            environment.set(setting, "configured-secret");
            assert!(spec().await.is_ok(), "{driver} {name}");
        }
        // Development and test keep the offline fallback.
        for name in ["development", "test"] {
            environment.set("RULLST_ENV", name);
            environment.set(setting, "");
            assert!(spec().await.is_ok(), "{driver} {name}");
            environment.clear(setting);
            assert!(spec().await.is_ok(), "{driver} {name}");
        }
    }
    // The SES token read first decides, so an empty one is refused too.
    environment.set("MAIL_DRIVER", "ses");
    environment.set("RULLST_ENV", "production");
    environment.set("AWS_SES_TOKEN", "");
    environment.set("AWS_SES_BEARER_TOKEN", "configured-secret");
    assert!(names_the_setting(
        spec().await,
        "AWS_SES_TOKEN",
        "production"
    ));
    // An environment that cannot be resolved fails closed as well.
    environment.set("RULLST_ENV", "not-an-environment");
    assert!(matches!(spec().await, Err(MailError::ConfigError(_))));
}

#[cfg(feature = "mail-smtp")]
#[tokio::test]
async fn production_refuses_an_empty_smtp_host_but_keeps_the_local_default() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("MAIL_DRIVER", "smtp");
    environment.set("RULLST_ENV", "production");
    environment.set("MAIL_HOST", "");
    assert!(names_the_setting(spec().await, "MAIL_HOST", "production"));
    environment.set("MAIL_HOST", "mock_smtp");
    assert!(spec().await.is_ok());
    // Unset, the host is 127.0.0.1, a real relay rather than the mock.
    environment.clear("MAIL_HOST");
    assert!(matches!(spec().await, Ok(DriverSpec::Smtp { host, .. }) if host == "127.0.0.1"));
    environment.set("RULLST_ENV", "development");
    environment.set("MAIL_HOST", "");
    assert!(spec().await.is_ok());
}

/// The end-to-end case of the defect: a production send with no API key
/// fails instead of reporting success for mail the mock captured.
#[tokio::test]
async fn a_production_send_without_an_api_key_is_a_configuration_error() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    environment.set("RULLST_ENV", "production");
    environment.set("MAIL_DRIVER", "resend");
    Mail::reset_driver();
    let message = Message::new()
        .to("member@example.com")
        .from("team@acme.example")
        .subject("production send without a key")
        .text("must not be captured");
    let error = Mail::send_now(message).await.unwrap_err();
    assert!(matches!(&error, MailError::ConfigError(text) if text.contains("RESEND_API_KEY")));
    assert!(
        !crate::drivers::OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|delivery| delivery.message.subject == "production send without a key")
    );
    Mail::reset_driver();
}
