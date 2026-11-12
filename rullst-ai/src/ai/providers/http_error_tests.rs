//! Loopback checks that every transport keeps the `HTTP {status}` prefix and
//! adds the provider's bounded, redacted error message.
use super::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAiProvider,
    openai_compatible::OpenAiCompatibleProvider,
};
use crate::ai::{AiCancellation, AiError, AiProvider, Message, StreamingAiClient};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

/// Answers one request with `status` and a JSON `body`; returns the request head.
async fn serve_error(status: &'static str, body: String) -> (String, JoinHandle<String>) {
    serve_error_with(status, "", body).await
}

/// As [`serve_error`], with `headers` (each ending in CRLF) in the response.
async fn serve_error_with(
    status: &'static str,
    headers: &'static str,
    body: String,
) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut request = Vec::new();
        let mut chunk = [0; 4096];
        // The whole request is read so closing the socket cannot reset it.
        let head = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let count = stream.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..count]);
                let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                    assert!(count > 0, "request ended before its headers");
                    continue;
                };
                let head = String::from_utf8_lossy(&request[..end]).into_owned();
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if count == 0 || request.len() >= end + 4 + length {
                    break head;
                }
            }
        })
        .await
        .unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        head
    });
    (url, task)
}

fn api_error(result: Result<String, AiError>) -> String {
    match result {
        Err(AiError::HttpStatus(error)) => error.to_string(),
        other => panic!("expected an HTTP status error, got {other:?}"),
    }
}

#[tokio::test]
async fn native_providers_add_the_provider_message_after_the_status() {
    let body = r#"{"error":{"code":429,"message":"Quota exceeded for metric: free_tier_requests. See https://ai.dev/usage?key=AIzaSyFixtureSecret0123456789","status":"RESOURCE_EXHAUSTED"}}"#;
    for name in ["Gemini", "Anthropic", "OpenAI", "DeepSeek", "Ollama"] {
        let (url, server) = serve_error("429 Too Many Requests", body.to_string()).await;
        let timeout = Duration::from_secs(3);
        let provider: Box<dyn AiProvider> = match name {
            "Gemini" => Box::new(
                GeminiProvider::new("fixture-key")
                    .with_base_url(url)
                    .with_request_timeout(timeout),
            ),
            "Anthropic" => Box::new(
                AnthropicProvider::new("fixture-key")
                    .with_base_url(url)
                    .with_request_timeout(timeout),
            ),
            "OpenAI" => Box::new(
                OpenAiProvider::new("fixture-key")
                    .with_base_url(url)
                    .with_request_timeout(timeout),
            ),
            "DeepSeek" => Box::new(
                OpenAiCompatibleProvider::deepseek("fixture-key")
                    .unwrap()
                    .with_test_base_url(&url)
                    .with_request_timeout(timeout),
            ),
            "Ollama" => Box::new(
                OpenAiCompatibleProvider::ollama(url, "fixture-model")
                    .unwrap()
                    .with_request_timeout(timeout),
            ),
            _ => unreachable!("test cases"),
        };
        let message = api_error(provider.prompt("hello").await);
        server.await.unwrap();
        assert_eq!(
            message,
            format!(
                "{name} returned HTTP 429 Too Many Requests: RESOURCE_EXHAUSTED: Quota exceeded \
                 for metric: free_tier_requests. See https://ai.dev/usage"
            )
        );
        assert!(
            !message.contains("AIza"),
            "{name} echoed a key-shaped token"
        );
    }
}

#[tokio::test]
async fn gemini_default_model_reports_why_it_failed() {
    let body = r#"{"error":{"code":404,"message":"models/gemini-3.5-flash-lite is not found for API version v1beta.","status":"NOT_FOUND"}}"#;
    let (url, server) = serve_error("404 Not Found", body.to_string()).await;
    let message = api_error(
        GeminiProvider::new("fixture-key")
            .with_base_url(url)
            .with_request_timeout(Duration::from_secs(3))
            .prompt("hello")
            .await,
    );
    let head = server.await.unwrap();
    assert!(
        head.starts_with("POST /models/gemini-3.5-flash-lite:generateContent "),
        "{head}"
    );
    assert_eq!(
        message,
        "Gemini returned HTTP 404 Not Found: NOT_FOUND: models/gemini-3.5-flash-lite is not \
         found for API version v1beta."
    );
}

#[tokio::test]
async fn gemini_stream_adds_the_provider_message() {
    let body =
        r#"{"error":{"code":503,"message":"The model is overloaded.","status":"UNAVAILABLE"}}"#;
    let (url, server) = serve_error("503 Service Unavailable", body.to_string()).await;
    let client = StreamingAiClient::new(
        GeminiProvider::new("fixture-key")
            .with_base_url(url)
            .with_request_timeout(Duration::from_secs(3)),
    );
    let mut sink = |_: &str| -> Result<(), AiError> { Ok(()) };
    let result = client
        .stream_chat(&[Message::user("hi")], &AiCancellation::new(), &mut sink)
        .await;
    server.await.unwrap();
    match result {
        Err(AiError::HttpStatus(error)) => {
            assert_eq!(error.status(), 503);
            assert_eq!(error.code(), Some("UNAVAILABLE"));
            assert_eq!(error.message(), Some("The model is overloaded."));
            assert_eq!(
                error.to_string(),
                "Gemini returned HTTP 503 Service Unavailable: UNAVAILABLE: The model is overloaded."
            );
        }
        other => panic!("expected an HTTP status error, got {other:?}"),
    }
}

#[tokio::test]
async fn the_status_and_retry_after_are_typed() {
    let body = r#"{"error":{"message":"Rate limit reached","type":"requests","code":"rate_limit_exceeded"}}"#;
    let (url, server) = serve_error_with(
        "429 Too Many Requests",
        "Retry-After: 7\r\n",
        body.to_string(),
    )
    .await;
    let result = OpenAiProvider::new("fixture-key")
        .with_base_url(url)
        .with_request_timeout(Duration::from_secs(3))
        .prompt("hello")
        .await;
    server.await.unwrap();
    let Err(AiError::HttpStatus(error)) = result else {
        panic!("expected an HTTP status error, got {result:?}");
    };
    assert_eq!(error.provider(), "OpenAI");
    assert_eq!(error.status(), 429);
    assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
    assert_eq!(error.code(), Some("rate_limit_exceeded"));
    assert_eq!(error.message(), Some("Rate limit reached"));

    let gemini = r#"{"error":{"code":429,"message":"Quota exceeded","status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"52s"}]}}"#;
    let (url, server) = serve_error("429 Too Many Requests", gemini.to_string()).await;
    let result = GeminiProvider::new("fixture-key")
        .with_base_url(url)
        .with_request_timeout(Duration::from_secs(3))
        .prompt("hello")
        .await;
    server.await.unwrap();
    let Err(AiError::HttpStatus(error)) = result else {
        panic!("expected an HTTP status error, got {result:?}");
    };
    assert_eq!(error.retry_after(), Some(Duration::from_secs(52)));
    assert_eq!(error.code(), Some("RESOURCE_EXHAUSTED"));
}

#[tokio::test]
async fn compatible_provider_adds_a_bounded_single_line_message() {
    let long = "x ".repeat(400);
    let body = format!(r#"{{"object":"error","message":"model not loaded\n{long}","code":404}}"#);
    let (url, server) = serve_error("404 Not Found", body).await;
    let message = api_error(
        OpenAiCompatibleProvider::try_local(url, "fixture-model")
            .unwrap()
            .with_request_timeout(Duration::from_secs(3))
            .prompt("hello")
            .await,
    );
    server.await.unwrap();
    let detail = message
        .strip_prefix("OpenAI-compatible returned HTTP 404 Not Found: model not loaded x x")
        .expect("status prefix and message");
    assert!(!message.contains('\n'));
    assert!(detail.ends_with('…'));
    assert_eq!(
        message.chars().count(),
        "OpenAI-compatible returned HTTP 404 Not Found: ".len() + 300
    );
}

/// A JSON error body of exactly `len` bytes whose message is `padded`.
fn padded_error_body(len: usize) -> String {
    let head = r#"{"error":{"message":"padded"},"pad":""#;
    let tail = r#""}"#;
    let pad = len - head.len() - tail.len();
    format!("{head}{}{tail}", "x".repeat(pad))
}

#[tokio::test]
async fn reads_an_error_body_of_at_most_sixteen_kib() {
    for (len, detail) in [(16 * 1024, ": padded"), (16 * 1024 + 1, ""), (20_000, "")] {
        let (url, server) = serve_error("400 Bad Request", padded_error_body(len)).await;
        let message = api_error(
            OpenAiProvider::new("fixture-key")
                .with_base_url(url)
                .with_request_timeout(Duration::from_secs(3))
                .prompt("hello")
                .await,
        );
        server.await.unwrap();
        assert_eq!(
            message,
            format!("OpenAI returned HTTP 400 Bad Request{detail}"),
            "{len}-byte body"
        );
    }
}

#[tokio::test]
async fn bodies_without_a_provider_message_keep_the_plain_status() {
    let (url, server) = serve_error(
        "502 Bad Gateway",
        "<html><body>fixture-key upstream</body></html>".to_string(),
    )
    .await;
    let message = api_error(
        OpenAiProvider::new("fixture-key")
            .with_base_url(url)
            .with_request_timeout(Duration::from_secs(3))
            .prompt("hello")
            .await,
    );
    server.await.unwrap();
    assert_eq!(message, "OpenAI returned HTTP 502 Bad Gateway");
}
