//! Reuse of the facade's configured driver across messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::tests::{EnvironmentGuard, clear_provider_environment};
use super::*;
use crate::facade::{MAIL_ENV_LOCK, Mail};

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
