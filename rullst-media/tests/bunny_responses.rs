#![cfg(feature = "bunny")]
//! Crafted Bunny API responses: content type, body size, create echo, the
//! marker search page and delete status checks of the adapter.
use rullst_media::{bunny::*, *};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const MARKER: &str = "rullst-video-0123456789abcdef0123456789abcdef";
const VIDEO: &str = "12345678-1234-4234-8234-123456789abc";
const MIB: usize = 1_048_576;

#[derive(Clone)]
struct Canned {
    content_type: &'static str,
    body: Vec<u8>,
    chunked: bool,
}

/// Answers every request with the current canned `200` response.
struct Server {
    origin: String,
    canned: Arc<Mutex<Canned>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let canned = Arc::new(Mutex::new(Canned {
            content_type: "application/json",
            body: b"{}".to_vec(),
            chunked: false,
        }));
        let shared = canned.clone();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let response = shared.lock().unwrap().clone();
                tokio::spawn(answer(stream, response));
            }
        });
        Self {
            origin,
            canned,
            task,
        }
    }
    fn set(&self, content_type: &'static str, body: impl Into<Vec<u8>>, chunked: bool) {
        *self.canned.lock().unwrap() = Canned {
            content_type,
            body: body.into(),
            chunked,
        };
    }
    fn json(&self, body: serde_json::Value) {
        self.set("application/json", body.to_string(), false);
    }
    fn provider(&self) -> BunnyStream {
        let credentials = BunnyCredentials::new(
            "fixture_api_key_123456789",
            "fixture_webhook_key_123456789",
            "fixture_embed_key_123456789",
            "fixture_cdn_key_123456789",
        )
        .unwrap();
        let config = BunnyConfig::new(
            LibraryId::new(7).unwrap(),
            Reference::new("acceptance").unwrap(),
            "test-library.b-cdn.net",
            credentials,
            PrivateDelivery::configured(true, true, true).unwrap(),
        )
        .unwrap();
        BunnyStream::for_protocol_tests(config, &self.origin).unwrap()
    }
}

async fn answer(mut stream: TcpStream, canned: Canned) {
    let mut request = Vec::new();
    let mut buffer = [0u8; 4096];
    let body_start = loop {
        let Ok(read) = stream.read(&mut buffer).await else {
            return;
        };
        if read == 0 {
            return;
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8_lossy(&request[..body_start]).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .map_or(0, |value| value.trim().parse::<usize>().unwrap());
    while request.len() < body_start + length {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
    let mut response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: {}\r\nconnection: close\r\n",
        canned.content_type
    )
    .into_bytes();
    if canned.chunked {
        response.extend_from_slice(b"transfer-encoding: chunked\r\n\r\n");
        for chunk in canned.body.chunks(65_536) {
            response.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            response.extend_from_slice(chunk);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
    } else {
        response
            .extend_from_slice(format!("content-length: {}\r\n\r\n", canned.body.len()).as_bytes());
        response.extend_from_slice(&canned.body);
    }
    let _ = stream.write_all(&response).await;
    let _ = stream.shutdown().await;
}

fn video_json(id: &str, title: &str, status: u8) -> serde_json::Value {
    serde_json::json!({
        "videoLibraryId": 7, "guid": id, "title": title, "status": status,
        "length": 12, "hasMP4Fallback": false,
    })
}

fn padded(length: usize) -> Vec<u8> {
    let mut body = video_json(VIDEO, "Lesson", 4).to_string().into_bytes();
    body.resize(length, b' ');
    body
}

fn video() -> VideoId {
    VideoId::new(VIDEO).unwrap()
}

#[tokio::test]
async fn responses_need_a_json_content_type() {
    let server = Server::new().await;
    let provider = server.provider();
    let body = video_json(VIDEO, "Lesson", 4).to_string();
    server.set("text/plain", body.clone(), false);
    assert_eq!(provider.get(&video()).await, Err(MediaError::Protocol));
    server.set("application/json; charset=utf-8", body, false);
    let found = provider.get(&video()).await.unwrap().unwrap();
    assert_eq!(found.title, "Lesson");
    assert_eq!(found.processing, Processing::Ready);
}

#[tokio::test]
async fn response_bodies_are_bounded_at_one_mebibyte() {
    let server = Server::new().await;
    let provider = server.provider();
    for chunked in [false, true] {
        server.set("application/json", padded(MIB), chunked);
        let found = provider.get(&video()).await.unwrap().unwrap();
        assert_eq!(found.title, "Lesson", "chunked: {chunked}");
        server.set("application/json", padded(MIB + 1), chunked);
        assert_eq!(
            provider.get(&video()).await,
            Err(MediaError::Protocol),
            "chunked: {chunked}"
        );
    }
}

#[tokio::test]
async fn create_must_echo_the_marker_awaiting_upload() {
    let server = Server::new().await;
    let provider = server.provider();
    server.json(video_json(VIDEO, MARKER, 4));
    assert_eq!(provider.create(MARKER).await, Err(MediaError::Protocol));
    server.json(video_json(VIDEO, "Another title", 0));
    assert_eq!(provider.create(MARKER).await, Err(MediaError::Protocol));
    server.json(video_json(VIDEO, MARKER, 0));
    let created = provider.create(MARKER).await.unwrap();
    assert_eq!(created.id.as_str(), VIDEO);
    assert_eq!(created.processing, Processing::AwaitingUpload);
}

fn page(per_page: u32, total: u32, items: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "items": items, "totalItems": total, "currentPage": 1, "itemsPerPage": per_page,
    })
}

#[tokio::test]
async fn marker_search_reads_one_complete_page_of_one_hundred() {
    let server = Server::new().await;
    let provider = server.provider();
    let found = || vec![video_json(VIDEO, MARKER, 0)];
    server.json(page(50, 1, found()));
    assert_eq!(
        provider.find_created(MARKER).await,
        Err(MediaError::Protocol)
    );
    server.json(page(100, 2, found()));
    assert_eq!(
        provider.find_created(MARKER).await,
        Err(MediaError::Protocol)
    );
    let full: Vec<_> = (1..=100u32)
        .map(|i| {
            let id = format!("12345678-1234-4234-8234-{i:012x}");
            let title = if i == 42 {
                MARKER.to_owned()
            } else {
                format!("Lesson {i}")
            };
            video_json(&id, &title, 0)
        })
        .collect();
    server.json(page(100, 100, full));
    let created = provider.find_created(MARKER).await.unwrap().unwrap();
    assert_eq!(created.id.as_str(), "12345678-1234-4234-8234-00000000002a");
}

#[tokio::test]
async fn delete_needs_success_and_status_code_200() {
    let server = Server::new().await;
    let provider = server.provider();
    server.json(serde_json::json!({"success": true, "statusCode": 500}));
    assert_eq!(provider.delete(&video()).await, Err(MediaError::Protocol));
    server.json(serde_json::json!({"success": false, "statusCode": 200}));
    assert_eq!(provider.delete(&video()).await, Err(MediaError::Protocol));
    server.json(serde_json::json!({"success": true, "statusCode": 200}));
    assert_eq!(provider.delete(&video()).await, Ok(()));
}

#[tokio::test]
async fn simulated_processing_is_offline_only() {
    let offline = BunnyStream::new(
        BunnyConfig::upload_only(
            LibraryId::new(7).unwrap(),
            Reference::new("acceptance").unwrap(),
            BunnyCredentials::upload_only("mock_api").unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let created = offline.create(MARKER).await.unwrap();
    assert_eq!(
        offline.simulate_processing(&video(), Processing::Ready),
        Err(MediaError::NotFound)
    );
    offline
        .simulate_processing(&created.id, Processing::Ready)
        .unwrap();
    let current = offline.get(&created.id).await.unwrap().unwrap();
    assert_eq!(current.processing, Processing::Ready);
    let server = Server::new().await;
    assert_eq!(
        server
            .provider()
            .simulate_processing(&created.id, Processing::Ready),
        Err(MediaError::Unsupported)
    );
}
