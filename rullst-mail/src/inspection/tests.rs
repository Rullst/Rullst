use super::*;
use crate::drivers::MemoryDriver;

fn message_with(filename: &str, content: &[u8], mime_type: &str) -> Message {
    Message::new()
        .to("recipient@example.com")
        .subject("Attachment inspection")
        .text("safe")
        .attach_bytes(filename, content.to_vec(), mime_type)
}

#[tokio::test]
// TM-MAIL-01: executable, active and type-confused attachments fail before transport.
async fn local_inspector_accepts_bounded_safe_types_and_rejects_spoofed_or_active_content() {
    let inspector = LocalAttachmentInspector::strict();
    for safe in [
        Attachment::new("note.txt", b"safe text".to_vec(), "text/plain"),
        Attachment::new(
            "document.pdf",
            b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF".to_vec(),
            "application/pdf",
        ),
        Attachment::new(
            "pixel.png",
            b"\x89PNG\r\n\x1a\nfixture".to_vec(),
            "image/png",
        ),
    ] {
        inspector.inspect(&safe).await.expect("safe local shape");
    }

    let rejected = [
        Attachment::new("spoof.png", b"not png".to_vec(), "image/png"),
        Attachment::new(
            "active.pdf",
            b"%PDF-1.7\n/JavaScript (alert)".to_vec(),
            "application/pdf",
        ),
        Attachment::new(
            "secret.txt",
            b"api_key=should-not-leave".to_vec(),
            "text/plain",
        ),
        Attachment::new("program.txt", b"MZfixture".to_vec(), "text/plain"),
        Attachment::new(
            "archive.zip",
            b"PK\x03\x04fixture".to_vec(),
            "application/zip",
        ),
        Attachment::new("spoof.zip", b"not zip".to_vec(), "application/zip"),
    ];
    for attachment in rejected {
        assert!(matches!(
            inspector.inspect(&attachment).await,
            Err(AttachmentInspectionError::Rejected(_))
        ));
    }
    assert!(
        LocalAttachmentInspector::allowing_opaque()
            .inspect(&Attachment::new(
                "archive.zip",
                b"PK\x03\x04fixture".to_vec(),
                "application/zip",
            ))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn inspection_guard_blocks_rejected_and_unavailable_scans_before_transport() {
    let (driver, deliveries) = MemoryDriver::isolated();
    let guard = AttachmentInspectionGuard::new(driver, LocalAttachmentInspector::strict());
    let spoofed = message_with("spoof.png", b"not png", "image/png");
    assert_eq!(
        guard.send(&spoofed).await,
        Err(MailError::AttachmentRejected {
            reason: "type_mismatch"
        })
    );
    assert!(deliveries.lock().expect("deliveries").is_empty());

    struct UnavailableInspector;
    impl AttachmentInspector for UnavailableInspector {
        async fn inspect(&self, _attachment: &Attachment) -> Result<(), AttachmentInspectionError> {
            Err(AttachmentInspectionError::Unavailable)
        }
    }

    let (driver, deliveries) = MemoryDriver::isolated();
    let unavailable = AttachmentInspectionGuard::new(driver, UnavailableInspector);
    assert_eq!(
        unavailable
            .send(&message_with("safe.txt", b"safe", "text/plain"))
            .await,
        Err(MailError::AttachmentInspectionUnavailable)
    );
    assert!(deliveries.lock().expect("deliveries").is_empty());
}

#[tokio::test]
async fn text_scanning_rejects_unsafe_links_and_invalid_utf8_without_leaking_content() {
    let inspector = LocalAttachmentInspector::strict();
    let invalid_utf8 = Attachment::new("invalid.txt", vec![0xff], "text/plain");
    assert_eq!(
        inspector.inspect(&invalid_utf8).await,
        Err(AttachmentInspectionError::Rejected("invalid_text_encoding"))
    );
    let unsafe_link = Attachment::new(
        "link.txt",
        br#"<a href="javascript:alert(1)">click</a>"#.to_vec(),
        "text/plain",
    );
    let error = inspector
        .inspect(&unsafe_link)
        .await
        .expect_err("unsafe link");
    assert_eq!(
        error,
        AttachmentInspectionError::Rejected("unsafe_link_content")
    );
    assert!(!error.to_string().contains("javascript"));
}
