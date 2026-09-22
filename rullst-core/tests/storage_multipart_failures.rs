#![cfg(feature = "storage-multipart")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, Response},
    routing::any,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rullst_core::{
    Storage,
    storage::cloud::{
        CloudCredentials, CloudStorageConfig,
        multipart::{
            CompletionStatus, MultipartError, MultipartKey, MultipartLimits, MultipartStorage,
        },
    },
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

struct Reply {
    status: u16,
    body: String,
    delay: Duration,
}
impl Reply {
    fn xml(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            delay: Duration::ZERO,
        }
    }
}
#[derive(Default)]
struct Observations {
    replies: VecDeque<Reply>,
    marker: String,
    calls: usize,
    parts: Vec<Vec<u8>>,
}
type Shared = Arc<Mutex<Observations>>;
struct Fixture {
    uploader: MultipartStorage,
    observed: Shared,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn handler(State(state): State<Shared>, request: Request<Body>) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, 1024).await.unwrap();
    let is_head = parts.method == "HEAD";
    let (reply, marker) = {
        let mut state = state.lock().unwrap();
        state.calls += 1;
        assert!(parts.headers.contains_key("authorization"));
        let digest = ring::digest::digest(&ring::digest::SHA256, &body);
        let expected: String = digest.as_ref().iter().map(|v| format!("{v:02x}")).collect();
        assert_eq!(
            parts.headers.get("x-amz-content-sha256").unwrap(),
            expected.as_str()
        );
        if let Some(marker) = parts.headers.get("x-amz-meta-rullst-upload") {
            state.marker = marker.to_str().unwrap().into();
        }
        if parts.method == "PUT" {
            let url = reqwest::Url::parse(&format!("http://127.0.0.1{}", parts.uri)).unwrap();
            let id = url
                .query_pairs()
                .find(|(k, _)| k == "uploadId")
                .unwrap()
                .1
                .into_owned();
            assert_eq!(id, "opaque+/=?id");
            state.parts.push(body.to_vec());
        }
        (
            state
                .replies
                .pop_front()
                .expect("unexpected extra network call"),
            state.marker.clone(),
        )
    };
    if !reply.delay.is_zero() {
        tokio::time::sleep(reply.delay).await;
    }
    let mut response = Response::builder()
        .status(reply.status)
        .header("etag", "\"opaque-part\"");
    if is_head {
        response = response
            .header("x-amz-meta-rullst-upload", marker)
            .header("content-length", "5");
    }
    if reply.status == 307 {
        response = response.header("location", "http://127.0.0.1:1/forbidden");
    }
    response.body(Body::from(reply.body)).unwrap()
}

async fn fixture(replies: Vec<Reply>) -> Fixture {
    let observed = Arc::new(Mutex::new(Observations {
        replies: replies.into(),
        ..Default::default()
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new()
        .fallback(any(handler))
        .with_state(observed.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let config =
        CloudStorageConfig::new(CloudCredentials::new("local", "owned-fixture-secret").unwrap())
            .with_limits(1024, Duration::from_millis(150))
            .unwrap()
            .with_loopback_test_endpoint(endpoint)
            .unwrap();
    let uploader = Storage::s3("private-files", "auto")
        .with_cloud_config(config)
        .unwrap()
        .multipart(
            "one",
            MultipartKey::new(URL_SAFE_NO_PAD.encode([61; 32])).unwrap(),
            MultipartLimits::new(5, 5 * 1024 * 1024, Duration::from_secs(60)).unwrap(),
        )
        .unwrap();
    Fixture {
        uploader,
        observed,
        server,
    }
}
fn init() -> Reply {
    Reply::xml(
        "<InitiateMultipartUploadResult><Bucket>private-files</Bucket><Key>one</Key><UploadId>opaque+/=?id</UploadId></InitiateMultipartUploadResult>",
    )
}
fn sha() -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, b"hello")
        .as_ref()
        .try_into()
        .unwrap()
}

#[tokio::test]
async fn success_status_with_embedded_error_is_never_completion() {
    let fixture = fixture(vec![
        init(),
        Reply::xml(""),
        Reply::xml("<Error><Code>InternalError</Code></Error>"),
    ])
    .await;
    let token = fixture.uploader.begin(5).await.unwrap();
    let token = fixture
        .uploader
        .upload_part(&token, 1, b"hello", sha())
        .await
        .unwrap();
    assert_eq!(
        fixture.uploader.complete(&token).await,
        Err(MultipartError::CompletionUncertain)
    );
    assert_eq!(
        fixture.observed.lock().unwrap().parts,
        vec![b"hello".to_vec()]
    );
}

#[tokio::test]
async fn provider_completion_failure_is_uncertain_and_abort_checks_absence() {
    let fixture = fixture(vec![
        init(),
        Reply::xml(""),
        Reply {
            status: 503,
            body: String::new(),
            delay: Duration::ZERO,
        },
        Reply {
            status: 204,
            body: String::new(),
            delay: Duration::ZERO,
        },
        Reply::xml(""),
        Reply {
            status: 404,
            body: String::new(),
            delay: Duration::ZERO,
        },
        Reply {
            status: 404,
            body: String::new(),
            delay: Duration::ZERO,
        },
    ])
    .await;
    let token = fixture.uploader.begin(5).await.unwrap();
    let token = fixture
        .uploader
        .upload_part(&token, 1, b"hello", sha())
        .await
        .unwrap();
    assert_eq!(
        fixture.uploader.complete(&token).await,
        Err(MultipartError::CompletionUncertain)
    );
    use rullst_core::storage::cloud::multipart::AbortStatus;
    assert_eq!(
        fixture.uploader.abort(&token).await.unwrap(),
        AbortStatus::RetryRequired
    );
    assert_eq!(
        fixture.uploader.abort(&token).await.unwrap(),
        AbortStatus::Gone
    );
}

#[tokio::test]
async fn lost_completion_response_retains_reconciliation_and_never_reinitiates() {
    let mut lost = Reply::xml("");
    lost.delay = Duration::from_secs(1);
    let fixture = fixture(vec![init(), Reply::xml(""), lost, Reply::xml("")]).await;
    let token = fixture.uploader.begin(5).await.unwrap();
    let token = fixture
        .uploader
        .upload_part(&token, 1, b"hello", sha())
        .await
        .unwrap();
    assert_eq!(
        fixture.uploader.complete(&token).await,
        Err(MultipartError::CompletionUncertain)
    );
    assert_eq!(
        fixture.uploader.reconcile_completion(&token).await.unwrap(),
        CompletionStatus::Confirmed
    );
    assert_eq!(fixture.observed.lock().unwrap().calls, 4);
}

#[tokio::test]
async fn malformed_oversized_foreign_xml_redirect_and_dtd_fail_closed() {
    let responses = [
        "<InitiateMultipartUploadResult><Bucket>another-bucket</Bucket><Key>one</Key><UploadId>id</UploadId></InitiateMultipartUploadResult>".into(),
        "<InitiateMultipartUploadResult><Bucket>private-files</Bucket><Bucket>private-files</Bucket><Key>one</Key><UploadId>id</UploadId></InitiateMultipartUploadResult>".into(),
        "<!DOCTYPE x [<!ENTITY a SYSTEM 'file:///etc/passwd'>]><InitiateMultipartUploadResult>&a;</InitiateMultipartUploadResult>".into(),
        "<InitiateMultipartUploadResult xmlns='urn:unexpected'><Bucket>private-files</Bucket><Key>one</Key><UploadId>id</UploadId></InitiateMultipartUploadResult>".into(),
        "x".repeat(256 * 1024 + 1),
    ];
    for body in responses {
        let fixture = fixture(vec![Reply::xml(body)]).await;
        assert!(fixture.uploader.begin(5).await.is_err());
        assert_eq!(fixture.observed.lock().unwrap().calls, 1);
    }
    let fixture = fixture(vec![Reply {
        status: 307,
        body: "".into(),
        delay: Duration::ZERO,
    }])
    .await;
    assert!(fixture.uploader.begin(5).await.is_err());
    assert_eq!(fixture.observed.lock().unwrap().calls, 1);
}

#[tokio::test]
async fn changed_remote_parts_and_truncated_or_ambiguous_lists_are_rejected() {
    for entries in [
        "<IsTruncated>true</IsTruncated>",
        "<IsTruncated>false</IsTruncated><Part><PartNumber>257</PartNumber><Size>5</Size><ETag>&quot;p&quot;</ETag></Part>",
        "<IsTruncated>false</IsTruncated><Part><PartNumber>1</PartNumber><Size>5</Size><ETag>&quot;p&quot;</ETag></Part><Part><PartNumber>1</PartNumber><Size>5</Size><ETag>&quot;p&quot;</ETag></Part>",
    ] {
        let xml = format!(
            "<ListPartsResult><Bucket>private-files</Bucket><Key>one</Key><UploadId>opaque+/=?id</UploadId>{entries}</ListPartsResult>"
        );
        let fixture = fixture(vec![init(), Reply::xml(xml)]).await;
        let token = fixture.uploader.begin(5).await.unwrap();
        assert!(fixture.uploader.progress(&token).await.is_err());
    }
}
