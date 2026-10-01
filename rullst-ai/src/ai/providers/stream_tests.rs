//! Loopback fixtures for provider-reported usage.
use super::{
    anthropic::AnthropicProvider,
    deepseek::DeepSeekProvider,
    gemini::GeminiProvider,
    ollama::OllamaProvider,
    openai::OpenAiProvider,
    openai_compatible::{OpenAiCompatibleCapabilities, OpenAiCompatibleProvider},
};
use crate::ai::{
    AiCancellation, AiError, AiProvider, Message, StreamingAiClient, StreamingAiProvider,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};

/// Serves one response and returns the captured request.
fn serve_once(body: &'static str, content_type: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture binds");
    let address = listener.local_addr().expect("fixture address");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("fixture accepts");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4_096];
        loop {
            let read = stream.read(&mut buffer).expect("fixture reads");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                break;
            }
        }
        let _ = sender.send(String::from_utf8_lossy(&request).into_owned());
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    });
    (format!("http://{address}"), receiver)
}

fn json_body(request: &mpsc::Receiver<String>) -> (String, serde_json::Value) {
    let request = request
        .recv_timeout(Duration::from_secs(2))
        .expect("captured request");
    let (head, body) = request.split_once("\r\n\r\n").expect("request body");
    (
        head.to_string(),
        serde_json::from_str(body).expect("JSON body"),
    )
}

const SSE: &str = "text/event-stream";
const JSON: &str = "application/json";

#[tokio::test]
async fn chat_with_usage_reads_each_providers_documented_fields() {
    let (url, _request) = serve_once(
        r#"{"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7,"prompt_tokens_details":{"cached_tokens":1}}}"#,
        JSON,
    );
    let answer = OpenAiProvider::new("sk-test")
        .with_base_url(url)
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("OpenAI answer");
    let usage = answer.usage().expect("OpenAI usage");
    assert_eq!((answer.text(), usage.total_tokens()), ("ok", Some(7)));
    assert_eq!(usage.cached_input_tokens(), Some(1));

    let (url, _request) = serve_once(
        r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":9,"completion_tokens":1,"prompt_cache_hit_tokens":8}}"#,
        JSON,
    );
    let usage = DeepSeekProvider::new("sk-test")
        .with_base_url(url)
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("DeepSeek answer")
        .usage()
        .expect("DeepSeek usage");
    assert_eq!(
        (usage.total_tokens(), usage.cached_input_tokens()),
        (Some(10), Some(8))
    );

    let (url, _request) = serve_once(
        r#"{"content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":3,"cache_read_input_tokens":4}}"#,
        JSON,
    );
    let usage = AnthropicProvider::new("sk-test")
        .with_base_url(url)
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("Anthropic answer")
        .usage()
        .expect("Anthropic usage");
    assert_eq!(
        (usage.input_tokens(), usage.output_tokens()),
        (Some(14), Some(3))
    );

    let (url, _request) = serve_once(
        r#"{"candidates":[{"content":{"parts":[{"text":"ok"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":6,"candidatesTokenCount":2,"totalTokenCount":8}}"#,
        JSON,
    );
    let usage = GeminiProvider::new("g-test")
        .with_base_url(url)
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("Gemini answer")
        .usage()
        .expect("Gemini usage");
    assert_eq!(usage.total_tokens(), Some(8));

    let (url, _request) = serve_once(
        r#"{"message":{"content":"ok"},"done":true,"prompt_eval_count":3,"eval_count":4}"#,
        JSON,
    );
    let usage = OllamaProvider::new(url, "llama3")
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("Ollama answer")
        .usage()
        .expect("Ollama usage");
    assert_eq!(usage.total_tokens(), Some(7));

    let (url, _request) = serve_once(r#"{"choices":[{"message":{"content":"ok"}}]}"#, JSON);
    let answer = OpenAiCompatibleProvider::try_local(format!("{url}/v1"), "local")
        .expect("loopback provider")
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("compatible answer");
    assert_eq!(answer.usage(), None, "absent usage is never invented");
}

async fn stream_text<P: StreamingAiProvider>(
    provider: P,
) -> Result<(String, crate::ai::StreamSummary), AiError> {
    let client = StreamingAiClient::new(provider);
    let mut text = String::new();
    let mut sink = |chunk: &str| {
        text.push_str(chunk);
        Ok(())
    };
    let summary = client
        .stream_chat(
            &[Message::system("be brief"), Message::user("hi")],
            &AiCancellation::new(),
            &mut sink,
        )
        .await?;
    Ok((text, summary))
}

#[tokio::test]
async fn compatible_stream_requests_and_reports_usage_when_declared() {
    let (url, request) = serve_once(
        concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}],\"usage\":null}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":1,\"total_tokens\":5}}\n\n",
            "data: [DONE]\n\n"
        ),
        SSE,
    );
    let provider = OpenAiCompatibleProvider::try_local(format!("{url}/v1"), "local")
        .expect("loopback provider")
        .with_capabilities(OpenAiCompatibleCapabilities::chat_only().with_stream_usage());
    let (text, summary) = stream_text(provider).await.expect("stream");
    assert_eq!(text, "ok");
    assert_eq!(
        summary.usage().and_then(|usage| usage.total_tokens()),
        Some(5)
    );
    let (_, body) = json_body(&request);
    assert_eq!(body["stream_options"]["include_usage"], true);

    let (url, request) = serve_once(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n",
        SSE,
    );
    let provider = OpenAiCompatibleProvider::try_local(format!("{url}/v1"), "local")
        .expect("loopback provider")
        .with_capabilities(OpenAiCompatibleCapabilities::chat_only().with_streaming());
    let (_, summary) = stream_text(provider).await.expect("stream");
    assert_eq!(summary.usage(), None);
    let (_, body) = json_body(&request);
    assert!(
        body.get("stream_options").is_none(),
        "not sent unless declared"
    );
}
