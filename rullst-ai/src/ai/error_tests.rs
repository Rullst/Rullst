#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::error::Error as _;

#[test]
fn http_status_keeps_the_alpha4_text_prefix() {
    let error = AiError::HttpStatus(AiHttpError::new("Gemini", 429));
    assert_eq!(
        error.to_string(),
        "API error: Gemini returned HTTP 429 Too Many Requests"
    );
    let error = AiError::HttpStatus(
        AiHttpError::new("OpenAI", 404)
            .with_provider_message(None::<String>, "The model does not exist"),
    );
    assert_eq!(
        error.to_string(),
        "API error: OpenAI returned HTTP 404 Not Found: The model does not exist"
    );
}

#[test]
fn http_status_carries_status_delay_code_and_message() {
    let http = AiHttpError::new(String::from("Anthropic"), 529)
        .with_retry_after(Duration::from_secs(30))
        .with_provider_message(Some("overloaded_error"), "Overloaded");
    assert_eq!(http.provider(), "Anthropic");
    assert_eq!(http.status(), 529);
    assert_eq!(http.retry_after(), Some(Duration::from_secs(30)));
    assert_eq!(http.code(), Some("overloaded_error"));
    assert_eq!(http.message(), Some("Overloaded"));
    // 529 has no standard reason phrase.
    assert_eq!(
        http.to_string(),
        "Anthropic returned HTTP 529: overloaded_error: Overloaded"
    );
    assert!(http.source().is_none());
    let plain = AiHttpError::new("Ollama", 503);
    assert_eq!(plain.retry_after(), None);
    assert_eq!(plain.code(), None);
    assert_eq!(plain.message(), None);
}

#[test]
fn json_errors_keep_their_message_and_variant() {
    let source = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let message = source.to_string();
    let error = AiError::json(source);
    assert_eq!(error.to_string(), format!("Serialization error: {message}"));
    let AiError::SerializationError(inner) = &error else {
        panic!("expected a serialization error");
    };
    assert_eq!(inner.to_string(), message);
    assert_eq!(error.source().unwrap().to_string(), message);
}

#[test]
#[cfg_attr(miri, ignore)]
fn request_errors_drop_the_url_and_report_their_kind() {
    let source = reqwest::Client::new()
        .get("http://example.invalid/v1?key=secret")
        .header("bad header", "x")
        .build()
        .unwrap_err();
    let error = AiError::request(source);
    let AiError::RequestError(inner) = &error else {
        panic!("expected a request error");
    };
    assert!(!inner.is_timeout());
    assert!(!inner.is_connect());
    let rendered = format!("{error} {error:?}");
    assert!(!rendered.contains("secret"), "{rendered}");
    assert_eq!(inner.to_string(), "builder error");
    assert_eq!(error.to_string(), "Request error: builder error");
    assert_eq!(error.source().unwrap().to_string(), "builder error");
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_refused_connection_is_a_connect_error_with_its_cause() {
    // A port that was just released refuses the connection.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let source = reqwest::Client::new()
        .get(format!("http://{address}/"))
        .send()
        .await
        .unwrap_err();
    let AiError::RequestError(inner) = AiError::request(source) else {
        panic!("expected a request error");
    };
    assert!(inner.is_connect());
    assert!(!inner.is_timeout());
    assert!(inner.source().is_some(), "the transport cause is kept");
}

/// A reader failure whose cause has its own cause: `serde_json` reports the
/// I/O error's source as its own.
struct FailingReader;

#[derive(Debug)]
struct DeviceError(std::io::Error);

impl std::fmt::Display for DeviceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("device error")
    }
}

impl std::error::Error for DeviceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::io::Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other(DeviceError(std::io::Error::other(
            "disk gone",
        ))))
    }
}

#[test]
fn json_io_errors_keep_their_cause() {
    let source = serde_json::from_reader::<_, serde_json::Value>(FailingReader).unwrap_err();
    let AiError::SerializationError(inner) = AiError::json(source) else {
        panic!("expected a serialization error");
    };
    assert_eq!(inner.source().unwrap().to_string(), "disk gone");
}
