//! Signature, content-sniffing and delivery-path contracts of the local
//! inspector that the broader inspection tests leave implicit.
use super::*;
use crate::drivers::MemoryDriver;

const STRICT: LocalAttachmentInspector = LocalAttachmentInspector::strict();
const OPAQUE: LocalAttachmentInspector = LocalAttachmentInspector::allowing_opaque();

async fn inspect(
    inspector: LocalAttachmentInspector,
    filename: &str,
    content: &[u8],
    mime_type: &str,
) -> Result<(), AttachmentInspectionError> {
    inspector
        .inspect(&Attachment::new(filename, content.to_vec(), mime_type))
        .await
}

fn rejected(reason: &'static str) -> Result<(), AttachmentInspectionError> {
    Err(AttachmentInspectionError::Rejected(reason))
}

#[test]
fn errors_display_only_their_reason() {
    assert_eq!(
        AttachmentInspectionError::Rejected("type_mismatch").to_string(),
        "attachment rejected: type_mismatch"
    );
    assert_eq!(
        AttachmentInspectionError::Unavailable.to_string(),
        "attachment inspection unavailable"
    );
}

#[tokio::test]
async fn jpeg_and_gif_are_recognized_by_declared_type_or_extension() {
    const JPEG: &[u8] = b"\xff\xd8\xff\xe0fixture";
    for (filename, content, mime_type) in [
        ("photo", JPEG, "image/jpeg"),
        ("photo.jpg", JPEG, "image/jpeg"),
        ("photo.jpg", JPEG, "application/octet-stream"),
        ("photo.jpeg", JPEG, "application/octet-stream"),
        ("anim", b"GIF89afixture".as_slice(), "image/gif"),
        ("anim.gif", b"GIF87afixture", "image/gif"),
        ("anim.gif", b"GIF89afixture", "application/octet-stream"),
    ] {
        assert_eq!(
            inspect(STRICT, filename, content, mime_type).await,
            Ok(()),
            "{filename} as {mime_type}"
        );
    }
    for (filename, mime_type) in [
        ("photo", "image/jpeg"),
        ("photo.jpeg", "application/octet-stream"),
        ("anim", "image/gif"),
        ("anim.gif", "application/octet-stream"),
    ] {
        assert_eq!(
            inspect(OPAQUE, filename, b"not an image", mime_type).await,
            rejected("type_mismatch"),
            "{filename} as {mime_type}"
        );
    }
}

#[tokio::test]
async fn declared_svg_pdf_and_zip_types_are_checked_without_an_extension() {
    assert_eq!(
        inspect(OPAQUE, "image", b"plain words", "image/svg+xml").await,
        rejected("active_svg_content")
    );
    for inspector in [STRICT, OPAQUE] {
        assert_eq!(
            inspect(inspector, "report", b"plain words", "application/pdf").await,
            rejected("type_mismatch")
        );
    }
    for (filename, mime_type) in [
        ("archive", "application/zip"),
        ("archive.zip", "application/octet-stream"),
    ] {
        assert_eq!(
            inspect(OPAQUE, filename, b"not a zip", mime_type).await,
            rejected("type_mismatch"),
            "{filename} as {mime_type}"
        );
    }
}

#[tokio::test]
async fn every_zip_signature_is_accepted_as_an_opaque_archive() {
    for signature in [b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08"] {
        let content = [signature.as_slice(), b"fixture"].concat();
        assert_eq!(
            inspect(OPAQUE, "archive.zip", &content, "application/zip").await,
            Ok(())
        );
        assert_eq!(
            inspect(STRICT, "archive.zip", &content, "application/zip").await,
            rejected("opaque_content")
        );
    }
}

#[tokio::test]
async fn pdf_action_names_count_only_in_the_first_kib_of_pdf_content() {
    for inspector in [STRICT, OPAQUE] {
        assert_eq!(
            inspect(
                inspector,
                "notes.txt",
                b"see /JS and /Launch in the PDF spec",
                "text/plain"
            )
            .await,
            Ok(())
        );
    }
    let late_header = [vec![b' '; 1_024], b"%PDF-1.7 /JavaScript".to_vec()].concat();
    assert_eq!(
        inspect(OPAQUE, "blob.bin", &late_header, "application/octet-stream").await,
        Ok(())
    );
}

#[tokio::test]
async fn elf_and_thin_mach_o_headers_are_executable_content() {
    for magic in [
        b"\x7fELF".as_slice(),
        &[0xfe, 0xed, 0xfa, 0xce],
        &[0xce, 0xfa, 0xed, 0xfe],
        &[0xfe, 0xed, 0xfa, 0xcf],
        &[0xcf, 0xfa, 0xed, 0xfe],
    ] {
        let content = [magic, &[2, 1, 1, 0]].concat();
        for inspector in [STRICT, OPAQUE] {
            assert_eq!(
                inspect(inspector, "Updater", &content, "application/octet-stream").await,
                rejected("executable_content"),
                "{magic:02x?}"
            );
        }
    }
}

#[tokio::test]
async fn svg_is_recognized_by_prefix_prologue_doctype_and_root_element() {
    let documents: [&[u8]; 7] = [
        // An `svg`-prefixed element, even without a namespace declaration.
        br#"<svg:rect width="1"/>"#,
        br#"<!-- chart --><root><svg onload="x()"/></root>"#,
        br#"<?xml version="1.0"?><root><svg onload="x()"/></root>"#,
        br#"<!DOCTYPE svg><root><svg onload="x()"/></root>"#,
        br#"<!DOCTYPE x><svg onload="x()"/>"#,
        // An internal subset is skipped up to its closing `]>`.
        br#"<!DOCTYPE x [<!ELEMENT svg ANY>]><svg onload="x()"/>"#,
        // A `[` after the DOCTYPE does not open an internal subset.
        br#"<!DOCTYPE x><svg onload="x()">a[1]</svg>"#,
    ];
    for content in documents {
        for inspector in [STRICT, OPAQUE] {
            assert_eq!(
                inspect(inspector, "chart", content, "application/octet-stream").await,
                rejected("active_svg_content"),
                "{}",
                String::from_utf8_lossy(content)
            );
        }
    }
}

#[tokio::test]
async fn active_markup_is_recognized_by_html_doctype_and_xml_prologue() {
    for content in [
        b"<!doctype html><p>hi</p>".as_slice(),
        br#"<?xml version="1.0"?><root><script>x()</script></root>"#,
    ] {
        assert_eq!(
            inspect(STRICT, "page", content, "application/octet-stream").await,
            rejected("active_markup_content"),
            "{}",
            String::from_utf8_lossy(content)
        );
    }
}

#[tokio::test]
async fn delivery_id_and_tenant_sends_are_inspected_before_delivery() {
    let message = |filename: &str, content: &[u8], mime_type: &str| {
        Message::new()
            .to("recipient@example.com")
            .subject("Attachment inspection")
            .text("safe")
            .attach_bytes(filename, content.to_vec(), mime_type)
    };
    let (driver, deliveries) = MemoryDriver::isolated();
    let guard = AttachmentInspectionGuard::new(driver, STRICT);
    let safe = message("note.txt", b"safe", "text/plain");
    guard
        .send_with_delivery_id(&safe, "delivery-0000000001")
        .await
        .expect("inspected delivery");
    guard
        .send_for_tenant("tenant_acme", &safe)
        .await
        .expect("inspected tenant delivery");
    assert_eq!(deliveries.lock().expect("deliveries").len(), 2);

    let spoofed = message("spoof.png", b"not png", "image/png");
    let expected = Err(MailError::AttachmentRejected {
        reason: "type_mismatch",
    });
    assert_eq!(
        guard
            .send_with_delivery_id(&spoofed, "delivery-0000000002")
            .await,
        expected
    );
    assert_eq!(
        guard.send_for_tenant("tenant_acme", &spoofed).await,
        expected
    );
    assert_eq!(deliveries.lock().expect("deliveries").len(), 2);
}
