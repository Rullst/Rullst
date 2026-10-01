//! Reuse of the facade's configured driver across messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::tests::{EnvironmentGuard, clear_provider_environment};
use super::*;
use crate::Message;
use crate::facade::{MAIL_ENV_LOCK, Mail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Serves exactly one managed-identity token, then stops listening, so a
/// second identity request fails.
async fn one_identity_response() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/msi/token", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let body = r#"{"access_token":"fixture_access_token","token_type":"Bearer","expires_on":"4000000000"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    (endpoint, server)
}

fn message() -> Message {
    Message::new()
        .to("member@example.com")
        .from("accounts@example.com")
        .subject("Welcome")
        .text("hello")
}

#[tokio::test]
async fn facade_sends_reuse_the_managed_identity_token() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    let (identity_endpoint, server) = one_identity_response().await;
    environment.set("MAIL_DRIVER", "azure-acs");
    environment.set(
        "AZURE_COMMUNICATION_EMAIL_ENDPOINT",
        "https://fixture.communication.azure.com/",
    );
    environment.set("IDENTITY_ENDPOINT", &identity_endpoint);
    environment.set("IDENTITY_HEADER", "fixture_identity_header");
    environment.clear("AZURE_CLIENT_ID");
    Mail::reset_driver();

    // A future schedule makes ACS fail after the token is obtained but before
    // any network request: a second identity request would fail instead.
    let scheduled = message().send_in(std::time::Duration::from_secs(3600));
    for _ in 0..3 {
        let error = Mail::send_now(scheduled.clone()).await.unwrap_err();
        assert!(
            matches!(&error, MailError::ConfigError(text) if text.contains("schedule")),
            "{error}"
        );
    }
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .expect("the single identity request was served")
        .unwrap();
    Mail::reset_driver();
}

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

    // A setting that becomes invalid fails instead of reusing the old driver.
    environment.set("MAIL_DRIVER", "mailtrap-sandbox");
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
