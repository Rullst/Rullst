#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use axum::http::{Request as HttpRequest, StatusCode};
use axum::response::IntoResponse;
use futures_util::StreamExt;

pub(super) const CONTENT_TYPE: &str = "multipart/form-data; boundary=XyZ";

pub(super) fn field(name: &str, value: &str) -> Vec<u8> {
    format!("--XyZ\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
        .into_bytes()
}

pub(super) fn file_part(name: &str, file_name: &str, content: &[u8]) -> Vec<u8> {
    let mut part = format!(
        "--XyZ\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    part.extend_from_slice(content);
    part.extend_from_slice(b"\r\n");
    part
}

pub(super) fn form(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut body = parts.concat();
    body.extend_from_slice(b"--XyZ--\r\n");
    body
}

/// Runs `future` with a deadline, so a reader that waits forever fails.
pub(super) async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .expect("finished within the deadline")
}

async fn extract(request: HttpRequest<Body>) -> Result<Multipart, MultipartError> {
    Multipart::from_request(request, &()).await
}

fn request(body: Body) -> HttpRequest<Body> {
    HttpRequest::post("/upload")
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .body(body)
        .unwrap()
}

async fn read(body: Vec<u8>, policy: &MultipartPolicy) -> Result<UploadForm, MultipartError> {
    within(async { extract(request(Body::from(body))).await?.read(policy).await }).await
}

/// A body that yields `chunks` and then never ends.
fn stalled(chunks: Vec<Vec<u8>>) -> Body {
    Body::from_stream(
        futures_util::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>))
            .chain(futures_util::stream::pending()),
    )
}

#[tokio::test]
async fn fields_and_files_are_read_in_order() {
    let body = form(&[
        field("_token", "abc"),
        field("title", "Relatório"),
        file_part("doc", "../../report.pdf", b"%PDF-1.7\n"),
        field("title", "second"),
        file_part("doc", "b.txt", b"text"),
    ]);
    let mut form = read(body, &MultipartPolicy::default()).await.unwrap();
    assert_eq!(form.field("title"), Some("Relatório"));
    assert_eq!(form.field("missing"), None);
    assert_eq!(
        form.fields().collect::<Vec<_>>(),
        vec![
            ("_token", "abc"),
            ("title", "Relatório"),
            ("title", "second")
        ]
    );
    assert_eq!(form.files().len(), 2);
    let first = form.file("doc").unwrap();
    assert_eq!(first.display_name(), Some("report.pdf"));
    assert_eq!(first.kind(), Some(UploadKind::Pdf));
    assert_eq!(
        first.declared_content_type(),
        Some("application/octet-stream")
    );
    let taken = form.take_file("doc").unwrap();
    assert_eq!(taken.bytes(), Some(&b"%PDF-1.7\n"[..]));
    assert_eq!(form.file("doc").unwrap().bytes(), Some(&b"text"[..]));
    assert!(form.take_file("doc").is_some());
    assert!(form.take_file("doc").is_none());
}

#[tokio::test]
async fn only_multipart_form_data_with_a_valid_boundary_is_extracted() {
    for content_type in [
        None,
        Some("application/x-www-form-urlencoded"),
        Some("multipart/form-data"),
        Some("multipart/mixed; boundary=XyZ"),
    ] {
        let mut builder = HttpRequest::post("/upload");
        if let Some(value) = content_type {
            builder = builder.header(header::CONTENT_TYPE, value);
        }
        let error = extract(builder.body(Body::empty()).unwrap())
            .await
            .unwrap_err();
        assert_eq!(error, MultipartError::NotMultipart, "{content_type:?}");
    }
}

#[tokio::test]
async fn a_declared_length_over_the_total_limit_is_refused_before_reading() {
    let policy = MultipartPolicy::default().max_total_bytes(100);
    let refused = HttpRequest::post("/upload")
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .header(header::CONTENT_LENGTH, "101")
        .body(stalled(Vec::new()))
        .unwrap();
    let result = within(async { extract(refused).await?.read(&policy).await }).await;
    assert_eq!(result.unwrap_err(), MultipartError::TotalTooLarge);

    let body = form(&[field("a", "b")]);
    let exact = HttpRequest::post("/upload")
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .header(header::CONTENT_LENGTH, body.len().to_string())
        .body(Body::from(body.clone()))
        .unwrap();
    let policy = MultipartPolicy::default().max_total_bytes(body.len() as u64);
    let form = within(async { extract(exact).await?.read(&policy).await }).await;
    assert_eq!(form.unwrap().field("a"), Some("b"));
}

#[tokio::test]
async fn every_limit_holds_at_its_exact_boundary() {
    let body = form(&[field("a", "b")]);
    let total = body.len() as u64;
    let policy = MultipartPolicy::default().max_total_bytes(total);
    assert!(read(body.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_total_bytes(total - 1);
    assert_eq!(
        read(body, &policy).await.unwrap_err(),
        MultipartError::TotalTooLarge
    );

    let file = form(&[file_part("f", "a.bin", &[7; 100])]);
    let policy = MultipartPolicy::default().max_file_bytes(100);
    assert!(read(file.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_file_bytes(99);
    assert_eq!(
        read(file, &policy).await.unwrap_err(),
        MultipartError::FileTooLarge
    );

    let text = form(&[field("a", "0123456789")]);
    let policy = MultipartPolicy::default().max_field_bytes(10);
    assert!(read(text.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_field_bytes(9);
    assert_eq!(
        read(text, &policy).await.unwrap_err(),
        MultipartError::FieldTooLarge
    );

    let named = form(&[field("abcd", "x")]);
    let policy = MultipartPolicy::default().max_field_name_bytes(4);
    assert!(read(named.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_field_name_bytes(3);
    assert_eq!(
        read(named, &policy).await.unwrap_err(),
        MultipartError::FieldNameTooLong
    );

    let files = form(&[file_part("a", "1", b"1"), file_part("b", "2", b"2")]);
    let policy = MultipartPolicy::default().max_files(2);
    assert!(read(files.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_files(1);
    assert_eq!(
        read(files, &policy).await.unwrap_err(),
        MultipartError::TooManyFiles
    );

    let fields = form(&[field("a", "1"), field("b", "2")]);
    let policy = MultipartPolicy::default().max_fields(2);
    assert!(read(fields.clone(), &policy).await.is_ok());
    let policy = MultipartPolicy::default().max_fields(1);
    assert_eq!(
        read(fields, &policy).await.unwrap_err(),
        MultipartError::TooManyFields
    );
}

#[tokio::test]
async fn empty_file_inputs_are_skipped_and_not_counted() {
    let body = form(&[
        file_part("avatar", "", b""),
        file_part("doc", "", b""),
        file_part("doc", "a.txt", b"kept"),
    ]);
    let policy = MultipartPolicy::default().max_files(1);
    let read_form = read(body, &policy).await.unwrap();
    assert_eq!(read_form.files().len(), 1);
    assert_eq!(read_form.files()[0].bytes(), Some(&b"kept"[..]));
    // A nameless file with content still counts.
    let body = form(&[file_part("a", "", b"x"), file_part("b", "b.txt", b"y")]);
    assert_eq!(
        read(body, &policy).await.unwrap_err(),
        MultipartError::TooManyFiles
    );
}

#[tokio::test]
async fn the_defaults_are_the_documented_values() {
    let policy = MultipartPolicy::new();
    assert_eq!(policy, MultipartPolicy::default());
    assert_eq!(policy.max_file_bytes, 10_485_760);
    assert_eq!(policy.max_total_bytes, 16_777_216);
    assert_eq!(policy.max_files, 4);
    assert_eq!(policy.max_fields, 32);
    assert_eq!(policy.max_field_bytes, 65_536);
    assert_eq!(policy.max_field_name_bytes, 128);
    assert_eq!(policy.memory_threshold, 32_768);
    assert_eq!(policy.idle_timeout, Duration::from_secs(30));
    assert_eq!(policy.temp_directory(), std::env::temp_dir());
    assert_eq!(
        policy.clone().temp_dir("/srv/uploads").temp_directory(),
        PathBuf::from("/srv/uploads")
    );
    let kinds = policy
        .allowed_kinds([UploadKind::Pdf, UploadKind::Png, UploadKind::Pdf])
        .allowed_kinds;
    assert_eq!(kinds, vec![UploadKind::Png, UploadKind::Pdf]);
}

#[tokio::test]
async fn invalid_policies_are_refused() {
    let body = form(&[field("a", "b")]);
    let policy = MultipartPolicy::default().memory_threshold(1024 * 1024);
    assert!(read(body.clone(), &policy).await.is_ok());
    for (policy, reason) in [
        (
            MultipartPolicy::default().memory_threshold(1024 * 1024 + 1),
            "memory_threshold exceeds 1 MiB",
        ),
        (
            MultipartPolicy::default().expect_signature(Vec::new()),
            "a signature must be 1 to 64 bytes long",
        ),
        (
            MultipartPolicy::default().expect_signature(vec![1; 65]),
            "a signature must be 1 to 64 bytes long",
        ),
    ] {
        assert_eq!(
            read(body.clone(), &policy).await.unwrap_err(),
            MultipartError::InvalidPolicy(reason)
        );
    }
    let policy = MultipartPolicy::default()
        .expect_signature(vec![1; 64])
        .expect_signature(*b"b");
    assert!(read(body, &policy).await.is_ok());
}

#[tokio::test]
async fn slow_partial_and_failing_bodies_end_with_typed_errors() {
    let policy = MultipartPolicy::default().idle_timeout(Duration::from_millis(50));
    let slow = request(stalled(vec![field("a", "b")]));
    let result = within(async { extract(slow).await?.read(&policy).await }).await;
    assert_eq!(result.unwrap_err(), MultipartError::Timeout);

    let partial = field("a", "unfinished");
    assert_eq!(
        read(partial[..partial.len() - 2].to_vec(), &policy)
            .await
            .unwrap_err(),
        MultipartError::Malformed("the body ended before the closing boundary")
    );

    let failing = Body::from_stream(futures_util::stream::iter(vec![
        Ok(field("a", "b")),
        Err(std::io::Error::other("connection reset")),
    ]));
    let result = within(async { extract(request(failing)).await?.read(&policy).await }).await;
    assert_eq!(result.unwrap_err(), MultipartError::BodyRead);
}

#[tokio::test]
async fn a_body_in_one_byte_frames_reads_like_a_whole_one() {
    let body = form(&[field("a", "b"), file_part("f", "f.txt", &[b'z'; 40_000])]);
    let frames = body
        .iter()
        .map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![*byte])))
        .collect::<Vec<_>>();
    let streamed = request(Body::from_stream(futures_util::stream::iter(frames)));
    let policy = MultipartPolicy::default();
    let form = within(async { extract(streamed).await?.read(&policy).await })
        .await
        .unwrap();
    assert_eq!(form.field("a"), Some("b"));
    assert_eq!(form.files()[0].len(), 40_000);
}

#[tokio::test]
async fn many_tiny_parts_are_bounded_by_the_field_count() {
    let parts = (0..5_000)
        .map(|index| field(&format!("f{index}"), "v"))
        .collect::<Vec<_>>();
    let body = form(&parts);
    let policy = MultipartPolicy::default().max_fields(5_000);
    assert_eq!(
        read(body.clone(), &policy).await.unwrap().fields().count(),
        5_000
    );
    assert_eq!(
        read(body, &MultipartPolicy::default()).await.unwrap_err(),
        MultipartError::TooManyFields
    );
}

#[tokio::test]
async fn malformed_and_nested_parts_are_refused() {
    let nested = form(&[b"--XyZ\r\nContent-Disposition: form-data; name=\"files\"\r\nContent-Type: multipart/mixed; boundary=inner\r\n\r\n--inner\r\n\r\nx\r\n--inner--\r\n".to_vec()]);
    assert_eq!(
        read(nested, &MultipartPolicy::default()).await.unwrap_err(),
        MultipartError::Malformed("nested multipart parts are not supported")
    );
    let headerless = form(&[b"--XyZ\r\n\r\nx\r\n".to_vec()]);
    assert_eq!(
        read(headerless, &MultipartPolicy::default())
            .await
            .unwrap_err(),
        MultipartError::Malformed("a part has no Content-Disposition")
    );
    let mut binary = field("a", "");
    binary.truncate(binary.len() - 2);
    binary.extend_from_slice(b"\xff\xfe\r\n");
    assert_eq!(
        read(form(&[binary]), &MultipartPolicy::default())
            .await
            .unwrap_err(),
        MultipartError::FieldNotUtf8
    );
}

#[tokio::test]
async fn content_checks_deny_mismatched_files_and_remove_their_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .memory_threshold(0)
        .temp_dir(dir.path())
        .expect_signature(*b"%PDF-");
    let spoofed = form(&[
        file_part("ok", "a.pdf", b"%PDF-1.4 fine"),
        file_part("doc", "fake.pdf", b"\x89PNG\r\n\x1a\nnot a pdf"),
    ]);
    assert_eq!(
        read(spoofed, &policy).await.unwrap_err(),
        MultipartError::ContentDenied
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

    let kinds = MultipartPolicy::default().allowed_kinds([UploadKind::PlainText]);
    let active = form(&[file_part("doc", "note.txt", b"<svg onload=alert(1)>")]);
    assert_eq!(
        read(active, &kinds).await.unwrap_err(),
        MultipartError::ContentDenied
    );
    let plain = form(&[file_part("doc", "note.txt", b"just words")]);
    assert_eq!(
        read(plain, &kinds).await.unwrap().files()[0].kind(),
        Some(UploadKind::PlainText)
    );
}

#[tokio::test]
async fn temporary_files_are_removed_on_error_and_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let policy = MultipartPolicy::default()
        .memory_threshold(0)
        .max_file_bytes(1_000)
        .temp_dir(dir.path());
    let failing = form(&[
        file_part("a", "a.bin", &[1; 500]),
        file_part("b", "b.bin", &[2; 1_001]),
    ]);
    assert_eq!(
        read(failing, &policy).await.unwrap_err(),
        MultipartError::FileTooLarge
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

    let ok = form(&[file_part("a", "a.bin", &[1; 500])]);
    let form = read(ok, &policy).await.unwrap();
    let path = form.files()[0].temp_path().unwrap().to_path_buf();
    assert!(path.exists());
    drop(form);
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

pub(super) fn token_form(filler: usize, token_last: bool) -> Vec<u8> {
    let filler = file_part("doc", "a.bin", &vec![b'f'; filler]);
    let token = field("_token", "abc");
    if token_last {
        form(&[filler, token])
    } else {
        form(&[token, filler])
    }
}

async fn read_with(
    body: Vec<u8>,
    csrf_header: bool,
    inspection: Option<MultipartFieldInspection>,
) -> Result<UploadForm, MultipartError> {
    let mut builder = HttpRequest::post("/upload").header(header::CONTENT_TYPE, CONTENT_TYPE);
    if csrf_header {
        builder = builder.header("x-csrf-token", "abc");
    }
    let mut request = builder.body(Body::from(body)).unwrap();
    if let Some(inspection) = inspection {
        request.extensions_mut().insert(inspection);
    }
    let policy = MultipartPolicy::default();
    within(async { extract(request).await?.read(&policy).await }).await
}

#[tokio::test]
async fn a_token_the_csrf_check_cannot_read_is_refused_unless_a_header_is_sent() {
    // When the `_token` part ends, only the final `--\r\n` is unread.
    let base = token_form(0, true).len();
    let at_limit = token_form(65_540 - base, true);
    assert_eq!(at_limit.len() - 4, 65_536);
    assert!(read_with(at_limit, false, None).await.is_ok());
    let over = token_form(65_541 - base, true);
    assert_eq!(
        read_with(over.clone(), false, None).await.unwrap_err(),
        MultipartError::CsrfTokenTooLate
    );
    assert!(read_with(over, true, None).await.is_ok());
    assert!(
        read_with(token_form(100_000, false), false, None)
            .await
            .is_ok()
    );
    // Only the first `_token` is checked, as the middleware does.
    let late_second = form(&[
        field("_token", "abc"),
        file_part("doc", "a.bin", &[0; 70_000]),
        field("_token", "abc"),
    ]);
    assert!(read_with(late_second, false, None).await.is_ok());
}

#[tokio::test]
async fn text_fields_are_inspected_only_where_the_waf_marks_the_request() {
    let attack = form(&[field("comment", "<script>alert(1)</script>")]);
    let named = form(&[field("<script>x", "1")]);
    let enforce = Some(MultipartFieldInspection::Enforce);
    for body in [attack.clone(), named.clone()] {
        assert_eq!(
            read_with(body, false, enforce).await.unwrap_err(),
            MultipartError::WafRefused
        );
    }
    for body in [attack, named] {
        assert!(
            read_with(body.clone(), false, Some(MultipartFieldInspection::Report))
                .await
                .is_ok()
        );
        assert!(read_with(body, false, None).await.is_ok());
    }
    let benign = form(&[field("comment", "please select an option")]);
    assert!(read_with(benign, false, enforce).await.is_ok());
    // File contents and names are not inspected.
    let file = form(&[file_part(
        "doc",
        "<script>.txt",
        b"<script>alert(1)</script>",
    )]);
    assert!(read_with(file, false, enforce).await.is_ok());
}

#[tokio::test]
async fn errors_answer_with_their_status_and_a_fixed_text() {
    for (error, status) in [
        (
            MultipartError::NotMultipart,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            MultipartError::ContentDenied,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (MultipartError::TotalTooLarge, StatusCode::PAYLOAD_TOO_LARGE),
        (MultipartError::FileTooLarge, StatusCode::PAYLOAD_TOO_LARGE),
        (MultipartError::TooManyFiles, StatusCode::PAYLOAD_TOO_LARGE),
        (MultipartError::TooManyFields, StatusCode::PAYLOAD_TOO_LARGE),
        (MultipartError::FieldTooLarge, StatusCode::PAYLOAD_TOO_LARGE),
        (MultipartError::FieldNameTooLong, StatusCode::BAD_REQUEST),
        (MultipartError::PartHeadersTooLarge, StatusCode::BAD_REQUEST),
        (MultipartError::Malformed("x"), StatusCode::BAD_REQUEST),
        (MultipartError::FieldNotUtf8, StatusCode::BAD_REQUEST),
        (MultipartError::BodyRead, StatusCode::BAD_REQUEST),
        (MultipartError::Timeout, StatusCode::REQUEST_TIMEOUT),
        (MultipartError::WafRefused, StatusCode::FORBIDDEN),
        (MultipartError::CsrfTokenTooLate, StatusCode::FORBIDDEN),
        (
            MultipartError::Storage(std::io::ErrorKind::Other),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            MultipartError::InvalidPersistTarget,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            MultipartError::PersistTargetExists,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            MultipartError::InvalidPolicy("x"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ] {
        assert_eq!(error.status(), status, "{error:?}");
        let response = error.clone().into_response();
        assert_eq!(response.status(), status, "{error:?}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let text = |error: MultipartError| async move {
        let body = axum::body::to_bytes(error.into_response().into_body(), 4096)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    };
    assert_eq!(
        text(MultipartError::FileTooLarge).await,
        "an uploaded file exceeds its size limit"
    );
    assert_eq!(
        text(MultipartError::Malformed("a part is not form-data")).await,
        "malformed multipart body: a part is not form-data"
    );
    assert_eq!(
        text(MultipartError::WafRefused).await,
        "Access Denied: Malicious pattern detected by Rullst Shield WAF."
    );
    assert_eq!(
        text(MultipartError::CsrfTokenTooLate).await,
        crate::security::MULTIPART_TOKEN_ORDER_MESSAGE
    );
    assert_eq!(
        text(MultipartError::Storage(
            std::io::ErrorKind::PermissionDenied
        ))
        .await,
        "the upload could not be processed"
    );
    assert_eq!(
        MultipartError::Storage(std::io::ErrorKind::PermissionDenied).to_string(),
        "the upload could not be stored (permission denied)"
    );
}
