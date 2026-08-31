#![cfg(feature = "aws-ses")]

use rullst_mail::{AwsSesDriver, MailDriver, MailError, Message};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::Duration;

const ACCESS_KEY_ID: &str = "AKIAIOSFODNN7EXAMPLE";
const SECRET_ACCESS_KEY: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";

#[tokio::test]
async fn native_driver_signs_ses_v2_payload_and_preserves_message_capabilities() {
    let (endpoint, request_rx, fixture) =
        fixture_server(200, r#"{"MessageId":"ses-fixture-id"}"#, "application/json");
    let driver = AwsSesDriver::try_native(
        "us-east-1",
        ACCESS_KEY_ID,
        SECRET_ACCESS_KEY,
        Some("temporary-session-token".to_string()),
    )
    .expect("valid native credentials")
    .try_with_endpoint(endpoint)
    .expect("loopback endpoint");
    let message = Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("Native SES")
        .html("<strong>Hello</strong><img src=\"cid:logo\" alt=\"Logo\">")
        .unsubscribe_url("https://example.com/unsubscribe")
        .attach_cid("logo", "logo.txt", b"icon".to_vec(), "text/plain");

    driver.send(&message).await.expect("fixture accepts mail");
    let request = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("captured request");
    fixture.join().expect("fixture thread");

    assert!(request.starts_with("POST /v2/email/outbound-emails HTTP/1.1\r\n"));
    let authorization = header_value(&request, "authorization").expect("SigV4 authorization");
    assert!(authorization.starts_with("AWS4-HMAC-SHA256 Credential="));
    assert!(authorization.contains(ACCESS_KEY_ID));
    assert!(authorization.contains("/us-east-1/ses/aws4_request"));
    assert!(authorization.contains("SignedHeaders="));
    assert!(authorization.contains("Signature="));
    assert_eq!(
        header_value(&request, "x-amz-security-token"),
        Some("temporary-session-token")
    );
    assert!(header_value(&request, "x-amz-date").is_some());

    let payload: Value = serde_json::from_str(request_body(&request)).expect("SES JSON payload");
    assert_eq!(payload["FromEmailAddress"], "sender@example.com");
    assert_eq!(
        payload["Destination"]["ToAddresses"][0],
        "recipient@example.com"
    );
    assert_eq!(
        payload["Content"]["Simple"]["Subject"]["Data"],
        "Native SES"
    );
    assert_eq!(
        payload["Content"]["Simple"]["Attachments"][0]["RawContent"],
        "aWNvbg=="
    );
    assert_eq!(
        payload["Content"]["Simple"]["Attachments"][0]["ContentDisposition"],
        "INLINE"
    );
    let headers = payload["Content"]["Simple"]["Headers"]
        .as_array()
        .expect("unsubscribe headers");
    assert!(headers.iter().any(|header| {
        header["Name"] == "List-Unsubscribe-Post" && header["Value"] == "List-Unsubscribe=One-Click"
    }));
}

#[tokio::test]
async fn native_driver_maps_and_redacts_provider_rejections() {
    let (endpoint, request_rx, fixture) = fixture_server(
        400,
        r#"{"message":"password=provider-secret","code":"BadRequestException"}"#,
        "application/json",
    );
    let driver = AwsSesDriver::try_native("us-east-1", ACCESS_KEY_ID, SECRET_ACCESS_KEY, None)
        .expect("valid native credentials")
        .try_with_endpoint(endpoint)
        .expect("loopback endpoint");
    let message = Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("Rejected")
        .text("body");

    let error = driver
        .send(&message)
        .await
        .expect_err("fixture rejects mail");
    let _ = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("captured request");
    fixture.join().expect("fixture thread");

    assert!(matches!(
        error,
        MailError::ProviderResponse {
            provider: "aws_ses",
            status: 400,
            ..
        }
    ));
    assert!(!error.to_string().contains("provider-secret"));
    assert!(error.to_string().contains("[REDACTED]"));
}

#[tokio::test]
async fn native_driver_preserves_bounded_rate_limit_metadata() {
    let (endpoint, request_rx, fixture) = fixture_server(
        429,
        r#"{"message":"slow down","code":"TooManyRequestsException"}"#,
        "application/json",
    );
    let driver = AwsSesDriver::try_native("us-east-1", ACCESS_KEY_ID, SECRET_ACCESS_KEY, None)
        .expect("valid native credentials")
        .try_with_endpoint(endpoint)
        .expect("loopback endpoint");
    let message = Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("Rate limited")
        .text("body");

    let error = driver
        .send(&message)
        .await
        .expect_err("fixture throttles mail");
    let _ = request_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("captured request");
    fixture.join().expect("fixture thread");

    assert!(
        matches!(
            &error,
            MailError::RateLimited {
                provider: "aws_ses",
                retry_after: Some(delay),
                ..
            } if *delay == Duration::from_secs(120)
        ),
        "unexpected rate-limit mapping: {error:?}"
    );
}

#[test]
fn native_configuration_is_fail_closed_and_debug_redacted() {
    assert!(AwsSesDriver::try_native("us-east-1", "", "secret", None).is_err());
    assert!(
        AwsSesDriver::try_native(
            "us-east-1",
            ACCESS_KEY_ID,
            SECRET_ACCESS_KEY,
            Some(String::new())
        )
        .is_err()
    );
    let driver = AwsSesDriver::try_native("us-east-1", ACCESS_KEY_ID, SECRET_ACCESS_KEY, None)
        .expect("valid native credentials");
    let debug = format!("{driver:?}");
    assert!(debug.contains("native_sigv4"));
    assert!(!debug.contains(ACCESS_KEY_ID));
    assert!(!debug.contains(SECRET_ACCESS_KEY));
}

#[tokio::test]
async fn native_driver_rejects_provider_shape_and_size_limits_before_network() {
    let driver = AwsSesDriver::try_native("us-east-1", ACCESS_KEY_ID, SECRET_ACCESS_KEY, None)
        .expect("valid native credentials");
    let oversized_cid = Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("Invalid attachment")
        .text("body")
        .attach_cid("x".repeat(79), "file.txt", b"body".to_vec(), "text/plain");
    assert!(matches!(
        driver.send(&oversized_cid).await,
        Err(MailError::ValidationError(_))
    ));

    let oversized_message = Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("Oversized")
        .text("body")
        .attach_bytes(
            "large.bin",
            vec![0_u8; 30 * 1024 * 1024],
            "application/octet-stream",
        );
    assert!(matches!(
        driver.send(&oversized_message).await,
        Err(MailError::ValidationError(_))
    ));
}

fn fixture_server(
    status: u16,
    response_body: &'static str,
    content_type: &'static str,
) -> (String, mpsc::Receiver<String>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SES fixture");
    let address = listener.local_addr().expect("fixture address");
    let (request_tx, request_rx) = mpsc::channel();
    let fixture = std::thread::spawn(move || {
        let attempts = if status == 429 { 3 } else { 1 };
        for attempt in 0..attempts {
            let (mut socket, _) = listener.accept().expect("accept SES request");
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let request = read_http_request(&mut socket);
            if attempt == 0 {
                request_tx.send(request).expect("send captured request");
            }
            let reason = if status == 200 {
                "OK"
            } else if status == 429 {
                "Too Many Requests"
            } else {
                "Bad Request"
            };
            write!(
                socket,
                "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nRetry-After: 120\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            )
            .expect("write SES fixture response");
        }
    });
    (format!("http://{address}"), request_rx, fixture)
}

fn read_http_request(socket: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4_096];
    loop {
        let read = socket.read(&mut buffer).expect("read SES request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = find_header_end(&bytes) {
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("UTF-8 fixture request")
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn header_value<'a>(request: &'a str, expected: &str) -> Option<&'a str> {
    request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(expected).then(|| value.trim())
    })
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}
