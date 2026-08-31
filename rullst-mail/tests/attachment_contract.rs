use rullst_mail::{
    Attachment, DeliveryPipeline, MAX_ATTACHMENT_BYTES, MAX_ATTACHMENT_COUNT,
    MAX_TOTAL_ATTACHMENT_BYTES, MailError, Message,
};

fn base_message() -> Message {
    Message::new()
        .to("alice@example.com")
        .from("sender@example.com")
        .subject("Attachment contract")
        .html("<p>Hello</p><img src=\"cid:brand_logo\">")
}

#[test]
fn attachment_debug_omits_owned_bytes() {
    let attachment = Attachment::new(
        "private.txt",
        b"attachment-secret-marker".to_vec(),
        "text/plain",
    );
    let debug = format!("{attachment:?}");
    assert!(debug.contains("content_bytes: 24"));
    assert!(!debug.contains("attachment-secret-marker"));
}

#[test]
fn preflight_accepts_a_referenced_unique_inline_asset() {
    let message =
        base_message().attach_cid("brand_logo", "logo.png", b"image".to_vec(), "image/png");
    DeliveryPipeline::prepare(&message).expect("valid inline attachment");
}

#[test]
fn preflight_rejects_unsafe_or_ambiguous_attachment_metadata() {
    let invalid_filename =
        base_message().attach_bytes("../invoice.pdf", b"pdf".to_vec(), "application/pdf");
    assert!(matches!(
        DeliveryPipeline::prepare(&invalid_filename),
        Err(MailError::ValidationError(message)) if message.contains("bounded basename")
    ));

    let invalid_mime = base_message().attach_bytes(
        "invoice.pdf",
        b"pdf".to_vec(),
        "application/pdf; charset=utf-8",
    );
    assert!(matches!(
        DeliveryPipeline::prepare(&invalid_mime),
        Err(MailError::ValidationError(message)) if message.contains("parameter-free")
    ));

    let bracketed_cid =
        base_message().attach_cid("<brand_logo>", "logo.png", b"image".to_vec(), "image/png");
    assert!(matches!(
        DeliveryPipeline::prepare(&bracketed_cid),
        Err(MailError::ValidationError(message)) if message.contains("without angle brackets")
    ));
}

#[test]
fn preflight_rejects_missing_duplicate_or_unreferenced_content_ids() {
    let no_html = Message::new()
        .to("alice@example.com")
        .subject("No HTML")
        .text("plain")
        .attach_cid("brand_logo", "logo.png", b"image".to_vec(), "image/png");
    assert!(matches!(
        DeliveryPipeline::prepare(&no_html),
        Err(MailError::ValidationError(message)) if message.contains("require an HTML body")
    ));

    let unreferenced =
        base_message().attach_cid("different_logo", "logo.png", b"image".to_vec(), "image/png");
    assert!(matches!(
        DeliveryPipeline::prepare(&unreferenced),
        Err(MailError::ValidationError(message)) if message.contains("must be referenced")
    ));

    let prefix_only = Message::new()
        .to("alice@example.com")
        .subject("CID prefix")
        .html("<img src=\"cid:brand_logo_dark\">")
        .attach_cid("brand_logo", "logo.png", b"image".to_vec(), "image/png");
    assert!(matches!(
        DeliveryPipeline::prepare(&prefix_only),
        Err(MailError::ValidationError(message)) if message.contains("must be referenced")
    ));

    let duplicate = base_message()
        .attach_cid("brand_logo", "logo.png", b"first".to_vec(), "image/png")
        .attach_cid(
            "brand_logo",
            "logo-dark.png",
            b"second".to_vec(),
            "image/png",
        );
    assert!(matches!(
        DeliveryPipeline::prepare(&duplicate),
        Err(MailError::ValidationError(message)) if message.contains("must be unique")
    ));
}

#[test]
fn preflight_caps_the_number_of_attachments_before_transport_encoding() {
    let mut message = base_message();
    for index in 0..=MAX_ATTACHMENT_COUNT {
        message = message.attach_bytes(format!("attachment-{index}.txt"), Vec::new(), "text/plain");
    }
    assert!(matches!(
        DeliveryPipeline::prepare(&message),
        Err(MailError::ValidationError(message)) if message.contains("at most 32 attachments")
    ));
}

#[test]
fn preflight_caps_individual_and_aggregate_attachment_bytes() {
    let oversized = base_message().attach_bytes(
        "oversized.bin",
        vec![0; MAX_ATTACHMENT_BYTES + 1],
        "application/octet-stream",
    );
    assert!(matches!(
        DeliveryPipeline::prepare(&oversized),
        Err(MailError::ValidationError(message)) if message.contains("one attachment")
    ));

    let first_bytes = MAX_TOTAL_ATTACHMENT_BYTES / 2 + 1;
    let aggregate = base_message()
        .attach_bytes(
            "first.bin",
            vec![0; first_bytes],
            "application/octet-stream",
        )
        .attach_bytes(
            "second.bin",
            vec![0; first_bytes],
            "application/octet-stream",
        );
    assert!(matches!(
        DeliveryPipeline::prepare(&aggregate),
        Err(MailError::ValidationError(message)) if message.contains("aggregate attachment")
    ));
}
