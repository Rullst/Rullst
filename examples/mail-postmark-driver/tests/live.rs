//! The live path against a loopback HTTP fixture that plays Postmark.

use rullst_mail::{FailoverDriver, MailDriver, MailError, MailFailureClass, MemoryDriver, Message};
use rullst_mail_postmark_example::PostmarkDriver;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;

const DEADLINE: Duration = Duration::from_secs(10);
const RECEIPT: &str = r#"{"To":"ada@example.com","SubmittedAt":"2026-11-15T07:25:01.4178645-05:00","MessageID":"0a129aee-e1cd-480d-b08d-4f48548ff48d","ErrorCode":0,"Message":"OK"}"#;

fn welcome() -> Message {
    Message::new()
        .to("ada@example.com")
        .from("Acme <hello@acme.example>")
        .subject("Welcome to Acme")
        .text("Thanks for signing up.")
        .reply_to("support@acme.example")
}

fn response(status: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Answers one canned response per connection and returns the requests.
async fn fixture(responses: Vec<String>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/email", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut socket, _) = timeout(DEADLINE, listener.accept()).await.unwrap().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = timeout(DEADLINE, socket.read(&mut buffer))
                    .await
                    .unwrap()
                    .unwrap();
                request.extend_from_slice(&buffer[..read]);
                let text = String::from_utf8_lossy(&request).to_string();
                let complete = text.split_once("\r\n\r\n").is_some_and(|(head, body)| {
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(str::to_owned)
                        })
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    body.len() >= length
                });
                if read == 0 || complete {
                    requests.push(text);
                    break;
                }
            }
            let _ = socket.write_all(response.as_bytes()).await;
        }
        requests
    });
    (endpoint, server)
}

async fn send(driver: &impl MailDriver) -> Result<(), MailError> {
    timeout(DEADLINE, driver.send(&welcome())).await.unwrap()
}

#[tokio::test]
async fn a_receipt_is_accepted_and_the_request_carries_the_token() {
    let (endpoint, server) = fixture(vec![response("200 OK", "", RECEIPT)]).await;
    let driver = PostmarkDriver::try_new("live-token")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    send(&driver).await.unwrap();
    let requests = timeout(DEADLINE, server).await.unwrap().unwrap();
    let (head, body) = requests[0].split_once("\r\n\r\n").unwrap();
    let head = head.to_ascii_lowercase();
    assert!(head.starts_with("post /email http/1.1\r\n"), "{head}");
    for header in [
        "x-postmark-server-token: live-token",
        "accept: application/json",
        "content-type: application/json",
    ] {
        assert!(head.contains(header), "{header}");
    }
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["From"], "Acme <hello@acme.example>");
    assert_eq!(body["ReplyTo"], "support@acme.example");
    assert!(driver.offline_outbox().lock().unwrap().is_empty());
}

#[tokio::test]
async fn receipts_are_bounded_and_must_report_an_accepted_message() {
    let padded = |length: usize| format!("{RECEIPT}{}", " ".repeat(length - RECEIPT.len()));
    let (endpoint, server) = fixture(vec![
        response("200 OK", "", &padded(16 * 1024)),
        response("200 OK", "", &padded(16 * 1024 + 1)),
        response(
            "200 OK",
            "",
            r#"{"ErrorCode":406,"MessageID":"0a12","Message":"Inactive"}"#,
        ),
        response("200 OK", "", "not json"),
    ])
    .await;
    let driver = PostmarkDriver::try_new("live-token")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    send(&driver).await.unwrap();
    for _ in 0..3 {
        assert!(matches!(send(&driver).await, Err(MailError::SendError(_))));
    }
    timeout(DEADLINE, server).await.unwrap().unwrap();
}

#[tokio::test]
async fn failures_are_classified_without_the_provider_body() {
    let (endpoint, server) = fixture(vec![
        response(
            "422 Unprocessable Entity",
            "",
            r#"{"ErrorCode":406,"Message":"ada@example.com is inactive"}"#,
        ),
        response(
            "401 Unauthorized",
            "",
            r#"{"ErrorCode":10,"Message":"Bad token"}"#,
        ),
        response("429 Too Many Requests", "Retry-After: 7\r\n", ""),
        response("429 Too Many Requests", "Retry-After: 100000\r\n", ""),
        response("503 Service Unavailable", "", "{}"),
    ])
    .await;
    let driver = PostmarkDriver::try_new("live-token")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    for status in [422, 401] {
        let error = send(&driver).await.unwrap_err();
        assert_eq!(error.failure_class(), MailFailureClass::Permanent);
        assert!(
            matches!(error, MailError::ProviderResponse { provider: "postmark", status: s, .. } if s == status)
        );
        assert!(!format!("{error:?}").contains("ada@example.com"));
    }
    let limited = send(&driver).await.unwrap_err();
    assert_eq!(limited.failure_class(), MailFailureClass::RateLimited);
    assert_eq!(limited.retry_after(), Some(Duration::from_secs(7)));
    let capped = send(&driver).await.unwrap_err();
    assert_eq!(capped.retry_after(), Some(Duration::from_secs(86_400)));
    let outage = send(&driver).await.unwrap_err();
    assert_eq!(outage.failure_class(), MailFailureClass::Transient);
    timeout(DEADLINE, server).await.unwrap().unwrap();
}

#[tokio::test]
async fn an_outage_fails_over_and_an_unreachable_endpoint_is_a_transport_error() {
    let (endpoint, server) = fixture(vec![response("503 Service Unavailable", "", "{}")]).await;
    let driver = PostmarkDriver::try_new("live-token")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    let (fallback, store) = MemoryDriver::isolated();
    let failover = FailoverDriver::new(driver).with_fallback(fallback);
    send(&failover).await.unwrap();
    assert_eq!(store.lock().unwrap().len(), 1);
    timeout(DEADLINE, server).await.unwrap().unwrap();

    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/email", closed.local_addr().unwrap());
    drop(closed);
    let unreachable = PostmarkDriver::try_new("live-token")
        .unwrap()
        .try_with_endpoint(endpoint)
        .unwrap();
    let error = send(&unreachable).await.unwrap_err();
    assert!(matches!(
        error,
        MailError::TransportError {
            provider: "postmark",
            ..
        }
    ));
}
