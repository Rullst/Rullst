#![cfg(feature = "storage-s3")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Deliberately malformed or hostile HTTP peers supplement the independent S3 service.
use rullst_core::{
    Storage, StorageError,
    storage::cloud::{CloudCredentials, CloudError, CloudStorageConfig},
};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

struct Peer {
    endpoint: String,
    received: oneshot::Receiver<Vec<u8>>,
    task: JoinHandle<()>,
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn peer(response: &'static [u8], delay: Duration) -> Peer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (send, received) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let mut used = 0;
        while used < request.len() {
            let n = socket.read(&mut request[used..]).await.unwrap();
            used += n;
            if n == 0 || request[..used].windows(4).any(|s| s == b"\r\n\r\n") {
                break;
            }
        }
        request.truncate(used);
        let _ = send.send(request);
        tokio::time::sleep(delay).await;
        let _ = socket.write_all(response).await;
    });
    Peer {
        endpoint,
        received,
        task,
    }
}

fn storage(peer: &Peer, limit: usize, timeout: Duration) -> Storage {
    Storage::s3("private-files", "us-east-1")
        .with_cloud_config(
            CloudStorageConfig::new(
                CloudCredentials::new("LOCALACCESS", "local-test-secret").unwrap(),
            )
            .with_loopback_test_endpoint(&peer.endpoint)
            .unwrap()
            .with_limits(limit, timeout)
            .unwrap(),
        )
        .unwrap()
}

#[tokio::test]
async fn declared_and_streaming_body_limits_are_enforced() {
    for reply in [
        &b"HTTP/1.1 200 OK\r\nContent-Length: 999999999\r\nConnection: close\r\n\r\n"[..],
        &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n9\r\n123456789\r\n0\r\n\r\n"[..],
    ] {
        let peer = peer(reply, Duration::ZERO).await;
        assert_eq!(storage(&peer,4,Duration::from_secs(2)).get("file").await.unwrap_err(), StorageError::Cloud(CloudError::SizeLimit));
    }
}

#[tokio::test]
async fn redirect_and_error_bodies_never_become_object_data_or_error_details() {
    let peer = peer(
        b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/private\r\nContent-Length: 0\r\n\r\n",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        storage(&peer, 64, Duration::from_secs(2))
            .get("file")
            .await
            .unwrap_err(),
        StorageError::Cloud(CloudError::Rejected(302))
    );
    let peer = self::peer(
        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 35\r\n\r\nlocal-test-secret /private/file.pdf",
        Duration::ZERO,
    )
    .await;
    let error = storage(&peer, 64, Duration::from_secs(2))
        .get("private/file.pdf")
        .await
        .unwrap_err();
    assert_eq!(error, StorageError::Cloud(CloudError::Rejected(403)));
    assert!(!format!("{error:?} {error}").contains("local-test-secret"));
    assert!(!format!("{error:?} {error}").contains("private/file.pdf"));
}

#[tokio::test]
async fn timeouts_and_truncated_objects_are_explicit_failures() {
    let peer = peer(
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx",
        Duration::from_secs(1),
    )
    .await;
    assert_eq!(
        storage(&peer, 64, Duration::from_millis(20))
            .get("file")
            .await
            .unwrap_err(),
        StorageError::Cloud(CloudError::Timeout)
    );
    let peer = self::peer(
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nx",
        Duration::ZERO,
    )
    .await;
    assert!(matches!(
        storage(&peer, 64, Duration::from_secs(2)).get("file").await,
        Err(StorageError::Cloud(
            CloudError::Transport | CloudError::InvalidResponse
        ))
    ));
}

#[tokio::test]
async fn metadata_requires_length_and_missing_objects_map_to_storage_not_found() {
    let peer = peer(
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        storage(&peer, 64, Duration::from_secs(2))
            .metadata("file")
            .await
            .unwrap_err(),
        StorageError::Cloud(CloudError::InvalidResponse)
    );
    let peer = self::peer(
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n",
        Duration::ZERO,
    )
    .await;
    assert!(matches!(
        storage(&peer, 64, Duration::from_secs(2)).get("file").await,
        Err(StorageError::NotFound(_))
    ));
}

#[tokio::test]
async fn requests_are_signed_and_grant_generation_does_not_contact_the_backend() {
    let mut peer = peer(
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        Duration::ZERO,
    )
    .await;
    let storage = storage(&peer, 64, Duration::from_secs(2));
    storage
        .signed_download("file", Duration::from_secs(60))
        .unwrap();
    assert!(matches!(
        peer.received.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    storage.put("file", b"payload").await.unwrap();
    let request = (&mut peer.received).await.unwrap();
    let request = String::from_utf8(request).unwrap();
    assert!(request.starts_with("PUT /private-files/file HTTP/1.1\r\n"));
    assert!(request.contains("authorization: AWS4-HMAC-SHA256 Credential=LOCALACCESS/"));
    assert!(request.contains("content-type: application/octet-stream"));
    assert!(request.contains(
        "x-amz-content-sha256: 239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5"
    ));
    assert!(!request.contains("local-test-secret"));
}

#[tokio::test]
async fn oversize_upload_is_rejected_before_network_io() {
    let mut peer = peer(
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        storage(&peer, 1, Duration::from_secs(2))
            .put("file", b"too large")
            .await
            .unwrap_err(),
        StorageError::Cloud(CloudError::SizeLimit)
    );
    assert!(matches!(
        peer.received.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
}
