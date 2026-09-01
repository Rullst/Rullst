#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn all_kinds_policy() -> UploadPolicy {
    UploadPolicy::try_new(
        1_024,
        [
            UploadKind::Png,
            UploadKind::Jpeg,
            UploadKind::Pdf,
            UploadKind::Mp4,
            UploadKind::WebM,
            UploadKind::PlainText,
        ],
    )
    .expect("valid all-kind policy")
}

#[test]
fn every_supported_signature_is_admitted_with_canonical_metadata() {
    let policy = all_kinds_policy();
    let cases: [(&str, &str, &[u8], UploadKind, &str); 6] = [
        (
            "image.png",
            " IMAGE/PNG ",
            b"\x89PNG\r\n\x1a\n",
            UploadKind::Png,
            "png",
        ),
        (
            "photo.JPEG",
            "image/jpeg",
            &[0xff, 0xd8, 0xff, 0x00],
            UploadKind::Jpeg,
            "jpg",
        ),
        (
            "report.pdf",
            "application/pdf",
            b"%PDF-1.7",
            UploadKind::Pdf,
            "pdf",
        ),
        (
            "clip.mp4",
            "video/mp4",
            &[0, 0, 0, 12, b'f', b't', b'y', b'p', 0, 0, 0, 0],
            UploadKind::Mp4,
            "mp4",
        ),
        (
            "clip.webm",
            "video/webm",
            &[0x1a, 0x45, 0xdf, 0xa3],
            UploadKind::WebM,
            "webm",
        ),
        (
            "notes.txt",
            "text/plain",
            b"bounded plain text",
            UploadKind::PlainText,
            "txt",
        ),
    ];

    for (name, media_type, bytes, expected_kind, expected_extension) in cases {
        assert_eq!(expected_kind.extension(), expected_extension);
        let admitted = policy
            .admit("tenant:academy", name, media_type, bytes)
            .expect("signature and metadata agree");
        assert_eq!(admitted.display_name(), name);
        assert_eq!(admitted.kind(), expected_kind);
        assert_eq!(admitted.byte_len(), bytes.len());
        assert_eq!(admitted.sha256_hex().len(), 64);
        assert!(
            admitted
                .quarantine_key()
                .ends_with(&format!(".{expected_extension}"))
        );
    }
}

#[test]
fn policy_and_admission_bounds_reject_unsafe_inputs() {
    assert!(UploadPolicy::try_new(0, [UploadKind::PlainText]).is_err());
    assert!(UploadPolicy::try_new(100 * 1_024 * 1_024 + 1, [UploadKind::PlainText]).is_err());
    assert!(UploadPolicy::try_new(10, []).is_err());

    let policy = UploadPolicy::try_new(
        4,
        [
            UploadKind::PlainText,
            UploadKind::PlainText,
            UploadKind::Jpeg,
        ],
    )
    .expect("valid deduplicated policy");
    assert_eq!(policy.max_bytes(), 4);
    assert_eq!(
        policy.allowed_kinds(),
        &[UploadKind::Jpeg, UploadKind::PlainText]
    );

    for (tenant, name, bytes, expected) in [
        ("", "note.txt", b"ok".as_slice(), UploadError::InvalidTenant),
        ("tenant", "", b"ok".as_slice(), UploadError::InvalidFileName),
        (
            "tenant",
            ".",
            b"ok".as_slice(),
            UploadError::InvalidFileName,
        ),
        (
            "tenant",
            "..",
            b"ok".as_slice(),
            UploadError::InvalidFileName,
        ),
        (
            "tenant",
            "bad\\name.txt",
            b"ok".as_slice(),
            UploadError::InvalidFileName,
        ),
        (
            "tenant",
            "note.txt",
            b"".as_slice(),
            UploadError::InvalidSize,
        ),
        (
            "tenant",
            "note.txt",
            b"large".as_slice(),
            UploadError::InvalidSize,
        ),
    ] {
        assert_eq!(
            policy.admit(tenant, name, "text/plain", bytes),
            Err(expected)
        );
    }

    assert!(matches!(
        policy.admit("tenant", "note", "text/plain", b"ok"),
        Err(UploadError::MediaTypeDenied)
    ));
    assert!(matches!(
        policy.admit("tenant", "note.txt", "text/plain", &[0xfe, 0xff]),
        Err(UploadError::MediaTypeDenied)
    ));
    assert!(matches!(
        policy.admit("tenant", "note.txt", "text/plain", b"a\x01"),
        Err(UploadError::MediaTypeDenied)
    ));
}

#[test]
fn active_content_variants_and_scan_evidence_fail_closed() {
    let policy = UploadPolicy::try_new(1_024, [UploadKind::PlainText]).expect("valid text policy");
    for content in [
        "<HTML>",
        "<!doctype html>",
        "<script>alert(1)</script>",
        "link javascript:alert(1)",
    ] {
        assert!(matches!(
            policy.admit("tenant", "note.txt", "text/plain", content.as_bytes()),
            Err(UploadError::ActiveContentDenied)
        ));
    }

    let admitted = policy
        .admit("tenant", "note.txt", "text/plain", b"clean")
        .expect("admitted fixture");
    assert!(matches!(
        OfflineMockScanner.scan(&admitted, b"other"),
        Err(UploadError::ScanUnavailable)
    ));

    for (engine, evidence_id) in [
        ("", "evidence".to_string()),
        ("scanner", "".to_string()),
        ("scanner\n", "evidence".to_string()),
        ("scanner", "x".repeat(257)),
    ] {
        let admitted = policy
            .admit("tenant", "note.txt", "text/plain", b"clean")
            .expect("admitted fixture");
        assert!(matches!(
            admitted.release(ScanVerdict::Clean {
                engine: engine.to_string(),
                evidence_id,
            }),
            Err(UploadError::ScanUnavailable)
        ));
    }
}
