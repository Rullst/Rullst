//! Multipart text-field inspection: the marker and its two modes.

use super::*;
use std::sync::{Arc, Mutex};

fn multipart_request(path: &str, content_type: &str) -> Request {
    Request::post(path)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::empty())
        .unwrap()
}

fn marked(mut req: Request, mode: MultipartFieldInspection) -> Option<MultipartFieldInspection> {
    mark_multipart_fields(&mut req, mode);
    req.extensions().get::<MultipartFieldInspection>().copied()
}

#[test]
fn only_multipart_requests_on_inspected_paths_are_marked() {
    for mode in [
        MultipartFieldInspection::Enforce,
        MultipartFieldInspection::Report,
    ] {
        for content_type in [
            "multipart/form-data; boundary=x",
            "Multipart/Form-Data ; boundary=x",
        ] {
            let req = multipart_request("/upload", content_type);
            assert_eq!(marked(req, mode), Some(mode), "{content_type}");
        }
        for content_type in [
            "multipart/mixed; boundary=x",
            "application/x-www-form-urlencoded",
        ] {
            let req = multipart_request("/upload", content_type);
            assert_eq!(marked(req, mode), None, "{content_type}");
        }
        let unlabelled = Request::post("/upload").body(Body::empty()).unwrap();
        assert_eq!(marked(unlabelled, mode), None);

        let security = crate::config::SecurityConfig {
            waf_body_exempt_paths: vec!["/upload".to_string()],
            ..Default::default()
        };
        let mut exempt = multipart_request("/upload", "multipart/form-data; boundary=x");
        exempt.extensions_mut().insert(security.clone());
        assert_eq!(marked(exempt, mode), None);
        let mut other = multipart_request("/other", "multipart/form-data; boundary=x");
        other.extensions_mut().insert(security);
        assert_eq!(marked(other, mode), Some(mode));
    }
}

#[test]
fn enforcement_refuses_a_signature_in_a_name_or_a_value() {
    let enforce = MultipartFieldInspection::Enforce;
    assert!(enforce.refuses("/upload", "comment", "<script>alert(1)</script>"));
    assert!(enforce.refuses("/upload", "x%3Cscript%3E", "fine"));
    assert!(enforce.refuses("/upload", "path", "../../etc/passwd"));
    assert!(!enforce.refuses("/upload", "comment", "please select an option"));
    assert_eq!(Rule::MultipartField.name(), "multipart-field-signature");
    let refusal = multipart_field_refusal();
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
    assert_eq!(refusal.headers()[header::CACHE_CONTROL], "no-store");
}

#[test]
fn reporting_logs_the_rule_and_path_without_the_field() {
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let report = MultipartFieldInspection::Report;
    let (refused, benign) = tracing::subscriber::with_default(subscriber, || {
        (
            report.refuses("/upload", "comment", "<script>alert(1)</script>"),
            report.refuses("/upload", "comment", "fine"),
        )
    });
    assert!(!refused && !benign);
    let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();
    assert_eq!(logs.lines().count(), 1, "{logs}");
    assert!(logs.contains("WARN"), "{logs}");
    assert!(
        logs.contains("rule=\"multipart-field-signature\""),
        "{logs}"
    );
    assert!(logs.contains("path=\"/upload\""), "{logs}");
    assert!(
        !logs.contains("script") && !logs.contains("comment"),
        "{logs}"
    );
}
