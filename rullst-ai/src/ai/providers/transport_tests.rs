//! Real loopback protocol checks; every server and client wait is bounded.
use super::{
    anthropic::AnthropicProvider, deepseek::DeepSeekProvider, gemini::GeminiProvider,
    ollama::OllamaProvider, openai::OpenAiProvider,
};
use crate::ai::AiProvider;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut request = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let count = stream.read(&mut chunk).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            assert!(request.len() < 64 * 1024);
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]);
                let len = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + len {
                    break;
                }
            }
        }
        request
    })
    .await
    .unwrap()
}

async fn serve(response: String) -> (String, JoinHandle<Option<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let Ok(Ok((mut stream, _))) =
            tokio::time::timeout(Duration::from_secs(1), listener.accept()).await
        else {
            return None;
        };
        let request = read_request(&mut stream).await;
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            stream.write_all(response.as_bytes()),
        )
        .await;
        Some(request)
    });
    (url, task)
}
fn success(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn custom_header_credentials_and_prompts_do_not_follow_cross_origin_redirects() {
    for name in ["gemini", "anthropic", "openai", "deepseek", "ollama"] {
        let body = r#"{"content":[{"type":"text","text":"ok"}],"candidates":[{"content":{"parts":[{"text":"ok"}]}}]}"#;
        let (destination, capture) = serve(success(body)).await;
        let redirect = format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: {destination}/capture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let (origin, first) = serve(redirect).await;
        let provider: Box<dyn AiProvider> = match name {
            "gemini" => Box::new(
                GeminiProvider::new("fixture-api-secret")
                    .with_base_url(origin)
                    .with_request_timeout(Duration::from_secs(3)),
            ),
            "anthropic" => Box::new(
                AnthropicProvider::new("fixture-api-secret")
                    .with_base_url(origin)
                    .with_request_timeout(Duration::from_secs(3)),
            ),
            "openai" => Box::new(
                OpenAiProvider::new("fixture-api-secret")
                    .with_base_url(origin)
                    .with_request_timeout(Duration::from_secs(3)),
            ),
            "deepseek" => Box::new(
                DeepSeekProvider::new("fixture-api-secret")
                    .with_base_url(origin)
                    .with_request_timeout(Duration::from_secs(3)),
            ),
            "ollama" => Box::new(
                OllamaProvider::new(origin, "fixture-model")
                    .with_request_timeout(Duration::from_secs(3)),
            ),
            _ => unreachable!("test cases"),
        };
        let result = provider.prompt("private-workflow-text").await;
        let first_request = first.await.unwrap().unwrap();
        let first_request = String::from_utf8_lossy(&first_request);
        assert!(first_request.contains("private-workflow-text"));
        if name != "ollama" {
            assert!(first_request.contains("fixture-api-secret"));
        }
        let redirected = capture.await.unwrap();
        assert!(
            redirected.is_none(),
            "{name} forwarded a sensitive request to a redirected origin"
        );
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn native_provider_rejects_oversized_streamed_json() {
    let payload = "a".repeat(2 * 1024 * 1024 + 1);
    let body = format!(r#"{{"choices":[{{"message":{{"content":"{payload}"}}}}]}}"#);
    // No Content-Length: the streaming read must enforce its own ceiling.
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}"
    );
    let (endpoint, server) = serve(response).await;
    let result = OpenAiProvider::new("fixture-api-key")
        .with_base_url(endpoint)
        .with_request_timeout(Duration::from_secs(3))
        .prompt("hello")
        .await;
    assert!(server.await.unwrap().is_some());
    assert!(result.is_err(), "accepted unbounded provider JSON");
}

#[tokio::test]
async fn native_provider_response_size_boundary_is_exact() {
    let prefix = r#"{"choices":[{"message":{"content":""#;
    let suffix = r#""}}]}"#;
    let payload = "a".repeat(super::support::MAX_RESPONSE_BYTES - prefix.len() - suffix.len());
    let body = format!("{prefix}{payload}{suffix}");
    let (endpoint, server) = serve(success(&body)).await;
    let answer = OpenAiProvider::new("fixture-api-key")
        .with_base_url(endpoint)
        .with_request_timeout(Duration::from_secs(3))
        .prompt("hello")
        .await
        .unwrap();
    assert_eq!(answer, payload);
    assert!(server.await.unwrap().is_some());
}

#[tokio::test]
async fn native_provider_deadline_covers_stalled_response_body() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
            .await
            .unwrap()
            .unwrap();
        read_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        OpenAiProvider::new("fixture-api-key")
            .with_base_url(endpoint)
            .with_request_timeout(Duration::from_millis(150))
            .prompt("hello"),
    )
    .await;
    server.abort();
    let _ = server.await;
    let crate::ai::AiError::RequestError(error) = result.unwrap().unwrap_err() else {
        panic!("expected actual transport timeout");
    };
    assert!(error.is_timeout());
    assert!(error.url().is_none());
}
