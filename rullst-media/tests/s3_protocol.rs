#![cfg(feature = "s3")]
//! The adapter's HTTP path against a local loopback object store fixture.
//! It checks request signing headers and payload digests, not provider acceptance.
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rullst_media::{s3::*, *};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

const MARKER: &str = "rullst-video-00112233445566778899aabbccddeeff";

#[derive(Default)]
struct Bucket {
    objects: BTreeMap<String, (String, Vec<u8>)>,
    requests: Vec<String>,
    unavailable: bool,
}
type Shared = Arc<Mutex<Bucket>>;

fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

async fn object(
    State(bucket): State<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
    };
    let mut bucket = bucket.lock().unwrap();
    bucket.requests.push(format!("{method} {}", uri.path()));
    if bucket.unavailable {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let signed = header("authorization")
        .starts_with("AWS4-HMAC-SHA256 Credential=fixture_access/20")
        && header("authorization").contains("/us-east-1/s3/aws4_request")
        && header("x-amz-content-sha256") == sha256_hex(&body)
        && !header("x-amz-date").is_empty();
    let Some(key) = uri.path().strip_prefix("/videos/") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !signed || uri.query().is_some() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let key = key.to_owned();
    match method {
        Method::PUT => {
            let kind = header("content-type").to_owned();
            bucket.objects.insert(key, (kind, body.to_vec()));
            StatusCode::OK.into_response()
        }
        Method::GET | Method::HEAD => match bucket.objects.get(&key) {
            Some((kind, bytes)) => (
                [
                    ("content-type", kind.clone()),
                    ("content-length", bytes.len().to_string()),
                ],
                if method == Method::GET {
                    bytes.clone()
                } else {
                    Vec::new()
                },
            )
                .into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Method::DELETE => {
            bucket.objects.remove(&key);
            StatusCode::NO_CONTENT.into_response()
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

async fn fixture() -> (S3Storage, Shared) {
    let bucket = Shared::default();
    // A fallback handler: the fixture has no parameterized route.
    let app = Router::new().fallback(object).with_state(bucket.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    let credentials = S3Credentials::new("fixture_access", "fixture_secret").unwrap();
    let config = S3Config::custom(
        LibraryId::new(5).unwrap(),
        format!("http://{address}"),
        "us-east-1",
        "videos",
        credentials,
    )
    .unwrap()
    .with_key_prefix("course/")
    .unwrap();
    assert!(
        S3Storage::new(
            S3Config::custom(
                LibraryId::new(5).unwrap(),
                format!("http://{address}"),
                "us-east-1",
                "videos",
                S3Credentials::new("fixture_access", "fixture_secret").unwrap(),
            )
            .unwrap()
        )
        .is_err()
    );
    (S3Storage::for_protocol_tests(config).unwrap(), bucket)
}

#[tokio::test]
async fn signed_requests_create_confirm_update_and_delete_objects() {
    let (storage, bucket) = fixture().await;
    assert_eq!(storage.binding().mode, ProviderMode::ProtocolFixture);
    let created = storage.create(MARKER).await.unwrap();
    assert_eq!(created.processing, Processing::AwaitingUpload);
    let key = format!("course/{}/original", created.id.as_str());
    let metadata_key = format!("course/{}/metadata.json", created.id.as_str());
    assert!(bucket.lock().unwrap().objects.contains_key(&metadata_key));
    assert_eq!(
        storage.find_created(MARKER).await.unwrap().unwrap().id,
        created.id
    );
    let metadata = Metadata::new("Borrowing", "Line one\nline two").unwrap();
    storage.update(&created.id, &metadata).await.unwrap();
    let current = storage.get(&created.id).await.unwrap().unwrap();
    assert_eq!(
        (current.title.as_str(), current.description.as_str()),
        ("Borrowing", "Line one\nline two")
    );
    assert_eq!(current.processing, Processing::AwaitingUpload);
    // A browser PUT lands; HEAD reports its size and type.
    bucket
        .lock()
        .unwrap()
        .objects
        .insert(key.clone(), ("video/mp4".into(), vec![0; 2_048]));
    assert_eq!(
        storage.get(&created.id).await.unwrap().unwrap().processing,
        Processing::Ready
    );
    bucket.lock().unwrap().objects.insert(
        key.clone(),
        ("application/x-msdownload".into(), vec![0; 16]),
    );
    assert_eq!(
        storage.get(&created.id).await.unwrap().unwrap().processing,
        Processing::Failed
    );
    storage.delete(&created.id).await.unwrap();
    assert!(storage.get(&created.id).await.unwrap().is_none());
    assert!(bucket.lock().unwrap().objects.is_empty());
    // Deleting an absent object is idempotent.
    storage.delete(&created.id).await.unwrap();
    let requests = bucket.lock().unwrap().requests.clone();
    assert!(requests.iter().all(|r| r.contains("/videos/course/")));
    assert!(requests.iter().any(|r| r.starts_with("HEAD ")));
}

#[tokio::test]
async fn provider_failures_are_typed_and_tampered_metadata_is_rejected() {
    let (storage, bucket) = fixture().await;
    let created = storage.create(MARKER).await.unwrap();
    let metadata_key = format!("course/{}/metadata.json", created.id.as_str());
    bucket.lock().unwrap().unavailable = true;
    assert_eq!(
        storage.get(&created.id).await.unwrap_err(),
        MediaError::Unavailable
    );
    bucket.lock().unwrap().unavailable = false;
    let other = VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap();
    let body = format!(
        r#"{{"version":1,"video":"{}","title":"x","description":""}}"#,
        other.as_str()
    );
    bucket.lock().unwrap().objects.insert(
        metadata_key.clone(),
        ("application/json".into(), body.into_bytes()),
    );
    assert_eq!(
        storage.get(&created.id).await.unwrap_err(),
        MediaError::Protocol
    );
    bucket.lock().unwrap().objects.insert(
        metadata_key,
        ("application/json".into(), vec![b' '; 20_000]),
    );
    assert_eq!(
        storage.get(&created.id).await.unwrap_err(),
        MediaError::Protocol
    );
    assert_eq!(
        storage
            .update(&other, &Metadata::new("t", "").unwrap())
            .await
            .unwrap_err(),
        MediaError::NotFound
    );
}
