use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};

pub const TEST_KEY: &str = "0123456789abcdef0123456789abcdef";
pub const EVENT_TIME_MS: u64 = 1_788_000_000_000;

#[derive(Debug)]
pub struct CapturedRequest {
    pub headers: String,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct FixtureResponse {
    pub status: u16,
    pub body: String,
    pub declared_length: Option<usize>,
    pub content_type: &'static str,
}

fn read_request(stream: &mut std::net::TcpStream) -> CapturedRequest {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("fixture read timeout");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4_096];
    loop {
        let read = stream.read(&mut buffer).expect("fixture reads request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_string)
            })
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if request.len() >= header_end + 4 + content_length {
            let headers = String::from_utf8(request[..header_end].to_vec())
                .expect("fixture headers are UTF-8");
            return CapturedRequest {
                headers,
                body: request[header_end + 4..header_end + 4 + content_length].to_vec(),
            };
        }
    }
    panic!("fixture received an incomplete request");
}

pub fn serve(
    responses: Vec<FixtureResponse>,
) -> (
    String,
    mpsc::Receiver<CapturedRequest>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback fixture binds");
    let address = listener.local_addr().expect("fixture address resolves");
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("fixture accepts request");
            let request = read_request(&mut stream);
            let _ = sender.send(request);
            let reason = if response.status == 202 {
                "Accepted"
            } else {
                "Service Unavailable"
            };
            let declared_length = response.declared_length.unwrap_or(response.body.len());
            write!(
                stream,
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status,
                reason,
                response.content_type,
                declared_length,
                response.body
            )
            .expect("fixture writes response");
        }
    });
    (format!("http://{address}/audit"), receiver, server)
}

pub fn header<'a>(headers: &'a str, name: &str) -> &'a str {
    headers
        .lines()
        .find_map(|line| {
            let (candidate, value) = line.split_once(": ")?;
            candidate.eq_ignore_ascii_case(name).then_some(value)
        })
        .expect("expected request header")
}

pub fn expected_signature(body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(TEST_KEY.as_bytes()).expect("valid HMAC key");
    mac.update(b"RULLST-AI-AUDIT-V1\n");
    mac.update(b"key-2026\n");
    mac.update(EVENT_TIME_MS.to_string().as_bytes());
    mac.update(b"\n");
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256={encoded}")
}
