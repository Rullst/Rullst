//! Request and response checks of the bounded OTLP client against a loopback
//! peer that answers one request with a canned HTTP response, and the text of
//! its stderr failure report.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTracePartialSuccess;
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::task::{Context, Poll, Waker};

const PROTOBUF: &str = "application/x-protobuf";

/// Answers one request on a loopback port with `response`.
fn peer(response: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1/traces", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap_or(0);
            }
            if line == "\r\n" {
                break;
            }
        }
        let mut body = vec![0; length];
        let _ = std::io::Read::read_exact(&mut reader, &mut body);
        let _ = reader.get_mut().write_all(&response);
    });
    endpoint
}

fn reply(body: &[u8], chunked: bool, content_type: &str) -> Vec<u8> {
    let mut response =
        format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nConnection: close\r\n");
    if chunked {
        response.push_str(&format!(
            "Transfer-Encoding: chunked\r\n\r\n{:x}\r\n",
            body.len()
        ));
    } else {
        response.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    }
    let mut response = response.into_bytes();
    response.extend_from_slice(body);
    if chunked {
        response.extend_from_slice(b"\r\n0\r\n\r\n");
    }
    response
}

fn client(endpoint: &str) -> BoundedOtlpClient {
    let config = TelemetryConfig::loopback_for_tests("service", endpoint, ["operation"]).unwrap();
    BoundedOtlpClient::new(&config, Arc::default()).unwrap()
}

/// Whether `send_bytes` accepts the export. It has no await point, so one
/// poll completes it, outside any Tokio runtime as on the exporter thread.
fn accepted(client: &BoundedOtlpClient, request: Request<Bytes>) -> bool {
    let mut future = std::pin::pin!(client.send_bytes(request));
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result.is_ok(),
        Poll::Pending => panic!("send_bytes has no await point"),
    }
}

fn export(method: &str, content_type: &str, body: Vec<u8>) -> Request<Bytes> {
    Request::builder()
        .method(method)
        .header("content-type", content_type)
        .body(Bytes::from(body))
        .unwrap()
}

#[test]
fn requests_must_be_bounded_protobuf_posts() {
    let success = || reply(&[], false, PROTOBUF);
    assert!(accepted(
        &client(&peer(success())),
        export("POST", PROTOBUF, vec![0; 1_048_576])
    ));
    for request in [
        export("POST", PROTOBUF, vec![0; 1_048_577]),
        export("PUT", PROTOBUF, Vec::new()),
        export("POST", "application/json", Vec::new()),
    ] {
        assert!(!accepted(&client(&peer(success())), request));
    }
    assert_eq!(
        format!("{:?}", client("http://127.0.0.1:9/v1/traces")),
        "BoundedOtlpClient([REDACTED])"
    );
}

#[test]
fn responses_must_be_bounded_protobuf_without_partial_failure() {
    // 16 KiB: an unknown length-delimited field (number 15) of 16,381 bytes.
    let mut filler = vec![122, 0xFD, 0x7F];
    filler.resize(16_384, 0);
    let partial = |rejected_spans: i64, error_message: &str| {
        ExportTraceServiceResponse {
            partial_success: Some(ExportTracePartialSuccess {
                rejected_spans,
                error_message: error_message.into(),
            }),
        }
        .encode_to_vec()
    };
    for (response, expected) in [
        (reply(&filler, false, PROTOBUF), true),
        (reply(&filler, true, PROTOBUF), true),
        (reply(&[], false, "text/plain"), false),
        (reply(&partial(0, ""), false, PROTOBUF), true),
        (reply(&partial(1, ""), false, PROTOBUF), false),
    ] {
        let client = client(&peer(response));
        assert_eq!(
            accepted(&client, export("POST", PROTOBUF, Vec::new())),
            expected
        );
    }
}

#[test]
#[ignore = "child process of failure_reports_name_the_request_limit_only_when_exceeded"]
fn failure_report_child() {
    let Ok(bytes) = std::env::var("RULLST_TEST_FAILURE_BYTES") else {
        return;
    };
    FailureReport::default().report(bytes.parse().unwrap(), Instant::now());
}

#[test]
fn failure_reports_name_the_request_limit_only_when_exceeded() {
    for (bytes, limit_named) in [(1_usize, false), (1_048_576, false), (1_048_577, true)] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "telemetry::distributed::http::tests::failure_report_child",
                "--nocapture",
            ])
            .env("RULLST_TEST_FAILURE_BYTES", bytes.to_string())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("dropped a span batch"), "{stderr}");
        assert_eq!(stderr.contains("request limit"), limit_named, "{bytes}");
    }
}
