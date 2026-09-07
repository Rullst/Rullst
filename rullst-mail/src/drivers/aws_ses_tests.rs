#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;
use std::time::Duration;

fn spawn_response(
    status: u16,
    extra_headers: &[(&str, &str)],
    body: &str,
) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_string();
    let headers = extra_headers
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect::<Vec<_>>();
    let handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "fixture accept timed out"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("fixture accept failed: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4_096];
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "fixture request timed out"
            );
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            assert!(request.len() <= 1024 * 1024, "fixture request too large");
            let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }

        let reason = if status == 200 {
            "OK"
        } else if status == 429 {
            "Too Many Requests"
        } else {
            "Error"
        };
        let mut response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        for (name, value) in headers {
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str("\r\n");
        response.push_str(&body);
        stream.write_all(response.as_bytes()).unwrap();
        request
    });
    (format!("http://{address}/send"), handle)
}

fn message() -> Message {
    Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("proxy contract")
        .html("<strong>HTML</strong>")
        .text("plain text")
        .unsubscribe_email("leave@example.com")
        .unsubscribe_url("https://example.com/unsubscribe")
}

#[tokio::test]
async fn bearer_proxy_does_not_redirect_private_message_payloads() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let destination = format!("http://{}/capture", listener.local_addr().unwrap());
    let capture = tokio::spawn(async move {
        let Ok(Ok((mut stream, _))) =
            tokio::time::timeout(Duration::from_secs(1), listener.accept()).await
        else {
            return false;
        };
        let mut buffer = [0u8; 8192];
        let count = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
            .await
            .unwrap();
        count > 0
    });
    let (endpoint, origin) = spawn_response(307, &[("Location", &destination)], "");
    let driver = AwsSesDriver::try_new("us-east-1", "fixture-proxy-secret")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(3), driver.send(&message()))
        .await
        .unwrap();
    assert!(!origin.join().unwrap().is_empty());
    let forwarded = capture.await.unwrap();
    assert!(
        !forwarded,
        "private mail payload was redirected outside the configured proxy"
    );
    assert!(outcome.is_err());
}

#[tokio::test]
async fn bearer_proxy_sends_the_complete_ses_payload_and_maps_provider_failures() {
    let (endpoint, capture) = spawn_response(200, &[], "{}");
    let driver = AwsSesDriver::try_new("sa-east-1", "proxy-secret")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    driver.send(&message()).await.unwrap();
    let request = String::from_utf8(capture.join().unwrap()).unwrap();
    assert!(request.starts_with("POST /send HTTP/1.1"));
    assert!(request.contains("authorization: Bearer proxy-secret"));
    let body = request.split_once("\r\n\r\n").unwrap().1;
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["FromEmailAddress"], "sender@example.com");
    assert_eq!(
        body["Destination"]["ToAddresses"][0],
        "recipient@example.com"
    );
    assert_eq!(
        body["Content"]["Simple"]["Body"]["Html"]["Data"],
        "<strong>HTML</strong>"
    );
    assert_eq!(
        body["Content"]["Simple"]["Body"]["Text"]["Data"],
        "plain text"
    );
    assert_eq!(
        body["Content"]["Simple"]["Headers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let (endpoint, capture) = spawn_response(
        429,
        &[("Retry-After", "7")],
        r#"{"message":"token=should-not-leak"}"#,
    );
    let driver = AwsSesDriver::try_new("us-east-1", "proxy-secret")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    let error = driver.send(&message()).await.unwrap_err();
    assert!(matches!(error, MailError::RateLimited { .. }));
    assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
    assert!(!error.to_string().contains("should-not-leak"));
    assert!(!capture.join().unwrap().is_empty());
}

#[tokio::test]
async fn bearer_proxy_rejects_transport_schedule_and_deprecated_invalid_configuration() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/send", listener.local_addr().unwrap());
    drop(listener);
    let driver = AwsSesDriver::try_new("us-east-1", "proxy-secret")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    assert!(matches!(
        driver.send(&message()).await,
        Err(MailError::TransportError {
            provider: "aws_ses_proxy",
            ..
        })
    ));

    let scheduled = message().send_in(Duration::from_secs(60));
    assert!(matches!(
        driver.send(&scheduled).await,
        Err(MailError::ConfigError(_))
    ));

    #[allow(deprecated)]
    let invalid = AwsSesDriver::new("bad_region", "proxy-secret")
        .with_endpoint("http://not-loopback.invalid/send");
    assert!(matches!(
        invalid.send(&message()).await,
        Err(MailError::ConfigError(_))
    ));
}

#[test]
fn configuration_payload_and_debug_paths_are_bounded_and_secret_free() {
    for region in ["", "bad_region", &"x".repeat(65)] {
        assert!(AwsSesDriver::try_new(region, "mock_token").is_err());
    }
    for endpoint in [
        "not-a-url",
        "ftp://example.com/send",
        "http://example.com/send",
        "https://user:secret@example.com/send",
    ] {
        assert!(validate_endpoint(endpoint).is_err());
    }

    let mock = AwsSesDriver::try_new("us-east-1", "mock_ses").unwrap();
    let debug = format!("{mock:?}");
    assert!(debug.contains("offline_mock"));
    assert!(!debug.contains("mock_ses"));

    let payload = proxy_payload(
        &Message::new()
            .to("recipient@example.com")
            .subject("subject"),
    );
    assert_eq!(payload["FromEmailAddress"], "noreply@rullst.dev");
    assert!(payload["Content"]["Simple"].get("Headers").is_none());
}

#[cfg(feature = "aws-ses")]
#[test]
fn native_constructors_require_regions_and_non_empty_credentials() {
    let no_region = aws_sdk_sesv2::Config::builder()
        .behavior_version_latest()
        .build();
    assert!(matches!(
        AwsSesDriver::from_native_config(no_region),
        Err(MailError::ConfigError(message)) if message.contains("region")
    ));
    assert!(AwsSesDriver::try_native("us-east-1", "", "secret", None).is_err());
    assert!(AwsSesDriver::try_native("us-east-1", "access", "", None).is_err());
    assert!(
        AwsSesDriver::try_native("us-east-1", "access", "secret", Some(String::new())).is_err()
    );

    let credentials =
        aws_sdk_sesv2::config::Credentials::new("access", "secret", None, None, "test-provider");
    let config = native::config_with_provider("us-east-1", credentials);
    let driver = AwsSesDriver::from_native_config(config).unwrap();
    assert_eq!(driver.region(), "us-east-1");
    assert_eq!(driver.delivery_mode(), DeliveryMode::Real);
    assert!(format!("{driver:?}").contains("native_sigv4"));
}
