//! Byte accounting of the offline mock store and its multipart service.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{CloudError, MockStore};

const PART: usize = 16 * 1024 * 1024;

#[test]
fn the_mock_store_holds_exactly_64_mib_and_releases_deleted_bytes() {
    let store = MockStore::default();
    let chunk = vec![0_u8; PART];
    for name in ["a", "b", "c", "d"] {
        store.put(name, &chunk).unwrap();
    }
    assert_eq!(
        store.put("one-more", b"x"),
        Err(CloudError::MockUnavailable)
    );
    store.delete("a").unwrap();
    store.put("a", &chunk).unwrap();
    assert_eq!(
        store.put("one-more", b"x"),
        Err(CloudError::MockUnavailable)
    );
}

#[cfg(feature = "storage-multipart")]
#[test]
fn completions_need_the_part_etags_and_keep_the_byte_count_exact() {
    use super::super::multipart::protocol::escaped;
    use reqwest::{
        Method,
        header::{HeaderMap, HeaderValue},
    };

    let store = MockStore::default();
    let chunk = vec![0_u8; PART];
    let call = |method: Method, key: &str, query: &[(&str, String)], body: &[u8]| {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-meta-rullst-upload",
            HeaderValue::from_static("marker"),
        );
        store
            .multipart_request(method, "bucket", key, query, headers, body)
            .unwrap()
    };
    let complete = |etag: &str| {
        format!(
            "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>{}</ETag></Part></CompleteMultipartUpload>",
            escaped(etag)
        )
    };

    // A completed upload replaces an existing object of the same size.
    store.put("object", &chunk).unwrap();
    store.put("other", &chunk).unwrap();
    call(Method::POST, "object", &[("uploads", String::new())], b"");
    let upload = || ("uploadId", "mock-upload-1".to_string());
    let part = call(
        Method::PUT,
        "object",
        &[upload(), ("partNumber", "1".into())],
        &chunk,
    );
    let etag = part.headers["etag"].to_str().unwrap().to_string();
    let forged = call(
        Method::POST,
        "object",
        &[upload()],
        complete("\"forged\"").as_bytes(),
    );
    assert_eq!(forged.status, 400);
    let completed = call(
        Method::POST,
        "object",
        &[upload()],
        complete(&etag).as_bytes(),
    );
    assert_eq!(completed.status, 200);
    // 32 MiB are stored, so two more parts' worth fit exactly.
    store.put("third", &chunk).unwrap();
    store.put("fourth", &chunk).unwrap();
    assert_eq!(
        store.put("one-more", b"x"),
        Err(CloudError::MockUnavailable)
    );

    // An aborted upload releases its parts.
    store.delete("fourth").unwrap();
    call(Method::POST, "spare", &[("uploads", String::new())], b"");
    let spare = || ("uploadId", "mock-upload-2".to_string());
    call(
        Method::PUT,
        "spare",
        &[spare(), ("partNumber", "1".into())],
        &chunk,
    );
    assert_eq!(call(Method::DELETE, "spare", &[spare()], b"").status, 204);
    store.put("fourth", &chunk).unwrap();
    assert_eq!(
        store.put("one-more", b"x"),
        Err(CloudError::MockUnavailable)
    );
}
